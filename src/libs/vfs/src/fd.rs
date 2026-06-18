// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! VFS file descriptor table and POSIX-compatible FD operations.
//!
//! This module provides the system-wide file descriptor table that maps
//! integer FDs to backend-specific file handles, and POSIX-compatible
//! operations (`open`, `read`, `write`, `lseek`, `fstat`, `close`, `stat`)
//! that route through the FD table.
//!
//! # File Handle Abstraction
//!
//! [`VfsFileHandle`] is an enum that dispatches to concrete filesystem
//! backends. To add a new backend:
//! 1. Add a variant to [`VfsFileHandle`].
//! 2. Implement `read`, `write`, `seek`, and `size` for the new variant.
//! 3. Update [`crate::fat32_backend`] (or create a new backend module).

//==================================================================================================
// Imports
//==================================================================================================

use crate::fat32_backend;
use ::alloc::{
    collections::BTreeMap,
    string::String,
    sync::Arc,
    vec::Vec,
};
use ::config::fds::{
    VFS_FD_BASE,
    VFS_MAX_OPEN_FILES,
};
use ::core::sync::atomic::{
    AtomicI32,
    Ordering,
};
use ::fat32::Fat32Error;
use ::spin::Mutex;
use ::sys::pm::ProcessIdentifier;
use ::sysapi::{
    fcntl::{
        file_control_request,
        file_creation_flags,
        file_descriptor_flags::{
            FD_CLOEXEC,
            FD_CLOFORK,
        },
        file_status_flags,
    },
    ffi::c_int,
    sys_stat::{
        file_mode,
        file_type,
    },
    sys_types::{
        c_size_t,
        gid_t,
        off_t,
        uid_t,
    },
    time::timespec,
    unistd::file_seek,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Block size reported in stat results (bytes).
const STAT_BLOCK_SIZE: i64 = 4096;

/// Sector size used for `st_blocks` computation (POSIX convention: 512 bytes).
const STAT_SECTOR_SIZE: u64 = 512;

/// Working directory assigned to a process that has no recorded directory of its own.
///
/// [`ProcessState::new`] seeds every freshly created state — including the placeholder that
/// [`set_current_process`] inserts lazily — with this value, so a state whose `cwd` still equals it
/// has never been the target of an explicit `chdir`.
const DEFAULT_CWD: &str = "/";

//==================================================================================================
// Metadata
//==================================================================================================

/// File metadata returned by stat operations.
///
/// This is the VFS-level metadata type, independent of any concrete
/// filesystem. Backend modules translate their native metadata into this
/// type.
pub struct VfsStat {
    /// File size in bytes (0 for directories).
    size: u64,
    /// Whether this entry is a directory.
    is_dir: bool,
}

impl VfsStat {
    /// Creates a new `VfsStat`.
    pub fn new(size: u64, is_dir: bool) -> Self {
        Self { size, is_dir }
    }

    /// Returns the file size in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Returns whether this entry is a directory.
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
}

//==================================================================================================
// Direct Read Handle
//==================================================================================================

/// Zero-copy direct memory access handle for file reads.
///
/// When a file's data is stored contiguously in an in-memory filesystem
/// image, reads can be served directly from the image buffer via memcpy,
/// bypassing all cluster chain traversal.
pub struct DirectReadHandle {
    /// Pointer to the file's data within the filesystem image.
    data: *const u8,
    /// File size in bytes.
    size: usize,
    /// Current read position.
    position: usize,
}

impl DirectReadHandle {
    /// Creates a new direct read handle.
    pub fn new(data: *const u8, size: usize) -> Self {
        Self {
            data,
            size,
            position: 0,
        }
    }

    /// Reads data from the direct memory region.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let remaining: usize = self.size.saturating_sub(self.position);
        let to_read: usize = buf.len().min(remaining);
        if to_read == 0 {
            return 0;
        }
        // SAFETY: data pointer is valid for the lifetime of the filesystem
        // image, and position + to_read <= size (guaranteed by min above).
        unsafe {
            let src: *const u8 = self.data.add(self.position);
            core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), to_read);
        }
        self.position += to_read;
        to_read
    }

    /// Seeks to a position in the direct memory region.
    pub fn seek(&mut self, offset: off_t, whence: c_int) -> Result<off_t, Fat32Error> {
        let new_pos: i64 = match whence {
            file_seek::SEEK_SET => offset,
            file_seek::SEEK_CUR => self.position as i64 + offset,
            file_seek::SEEK_END => self.size as i64 + offset,
            _ => return Err(Fat32Error::InvalidArgument),
        };
        if new_pos < 0 || new_pos > self.size as i64 {
            return Err(Fat32Error::InvalidSeek);
        }
        self.position = new_pos as usize;
        Ok(new_pos as off_t)
    }

    /// Returns the file size.
    pub fn size(&self) -> usize {
        self.size
    }
}

//==================================================================================================
// VFS File Handle
//==================================================================================================

/// An open file handle managed by the VFS.
///
/// Each variant corresponds to a concrete filesystem backend or an
/// optimization path. The VFS FD table stores these handles and
/// dispatches operations to the appropriate variant.
pub enum VfsFileHandle {
    /// File opened through the FAT32 backend.
    Fat32(crate::File),
    /// Zero-copy direct memory read (contiguous file optimization).
    DirectRead(DirectReadHandle),
    /// Open directory handle for `readdir()`/`getdents()` operations.
    Directory(DirectoryHandle),
    /// Remote file opened through the host filesystem daemon (hostfsd).
    /// Operations on this handle must be forwarded via IKC by the caller (vfsd).
    HostFs(HostFsHandle),
    /// One end of a POSIX unnamed pipe.
    Pipe(crate::pipe::PipeEnd),
    /// Routing token for a console stream (stdin/stdout/stderr).
    ///
    /// This is not a real handle: it carries no buffer and performs no I/O. It only lets vfsd own
    /// the descriptor slot and its per-descriptor flags while console I/O is routed elsewhere. No
    /// production path constructs this variant yet (that lands in a later plan).
    Console(ConsoleHandle),
    /// Routing token for a socket, holding the descriptor assigned by `networkd`.
    ///
    /// Socket I/O is not served by vfsd; like [`VfsFileHandle::HostFs`] this token only stores the
    /// remote descriptor so vfsd can own the slot and its per-descriptor flags. No production path
    /// constructs this variant yet (that lands in a later plan).
    Socket(SocketHandle),
}

/// Handle for a file opened on the host filesystem via hostfsd.
///
/// This handle stores the remote file descriptor returned by hostfsd.
/// The VFS cannot perform I/O on this handle directly — all operations
/// must be forwarded via IKC by the owning daemon (vfsd).
///
/// The `is_dir` flag is set once at open time and never re-checked. If the
/// host-side path changes type out-of-band (e.g., replaced by a directory),
/// subsequent operations will use the stale classification.
pub struct HostFsHandle {
    /// Remote file descriptor on the host side.
    remote_fd: i32,
    /// Whether this is a directory.
    is_dir: bool,
    /// Absolute path used to open this handle (stored only for directories to support dirfd).
    path: Option<String>,
    /// Next directory entry index to return on the following `getdents` call.
    ///
    /// hostfsd serves directory listings via offset-based iteration (one entry per
    /// `offset`), so vfsd tracks the per-FD cursor here. It advances as entries are
    /// consumed and is meaningful only for directory handles.
    readdir_offset: u32,
}

impl HostFsHandle {
    /// Creates a new HostFs handle with the given remote file descriptor.
    ///
    /// The `path` argument is only meaningful for directory handles (used by dirfd resolution).
    /// Pass `None` for regular file handles to avoid unnecessary allocations.
    pub fn new(remote_fd: i32, is_dir: bool, path: Option<String>) -> Self {
        Self {
            remote_fd,
            is_dir,
            path: if is_dir { path } else { None },
            readdir_offset: 0,
        }
    }

    /// Returns the remote file descriptor.
    pub fn remote_fd(&self) -> i32 {
        self.remote_fd
    }

    /// Returns whether this is a directory handle.
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Returns the path used to open this handle (only available for directories).
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the current directory iteration cursor.
    pub fn readdir_offset(&self) -> u32 {
        self.readdir_offset
    }

    /// Sets the directory iteration cursor.
    pub fn set_readdir_offset(&mut self, offset: u32) {
        self.readdir_offset = offset;
    }
}

/// Identifies which standard console stream a [`ConsoleHandle`] represents.
///
/// A console-backed descriptor performs no I/O of its own; the stream identity is the only state it
/// needs so that `fstat` can synthesize a stable device identity in a later plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleStream {
    /// Standard input (descriptor 0).
    Stdin,
    /// Standard output (descriptor 1).
    Stdout,
    /// Standard error (descriptor 2).
    Stderr,
}

/// Routing token for a console-backed descriptor.
///
/// A console handle records which standard stream the descriptor represents but holds no buffer and
/// performs no I/O. It exists so that vfsd can own the descriptor slot and its per-descriptor flags
/// while the actual console I/O is routed elsewhere.
pub struct ConsoleHandle {
    /// Which standard stream this descriptor represents.
    stream: ConsoleStream,
}

impl ConsoleHandle {
    /// Creates a console handle for the given standard stream.
    pub fn new(stream: ConsoleStream) -> Self {
        Self { stream }
    }

    /// Returns the standard stream this handle represents.
    pub fn stream(&self) -> ConsoleStream {
        self.stream
    }
}

/// Routing token for a socket-backed descriptor.
///
/// A socket handle stores the descriptor that `networkd` assigned to the socket (its remote fd),
/// analogous to [`HostFsHandle::remote_fd`]. Socket I/O is not served by vfsd; this token only lets
/// vfsd own the descriptor slot and its per-descriptor flags. Closing the remote descriptor when the
/// last reference is dropped is wired in a later plan.
pub struct SocketHandle {
    /// Descriptor assigned by `networkd` (the remote fd).
    remote_fd: i32,
}

impl SocketHandle {
    /// Creates a socket handle for the given `networkd` descriptor.
    pub fn new(remote_fd: i32) -> Self {
        Self { remote_fd }
    }

    /// Returns the `networkd` descriptor (remote fd) backing this socket.
    pub fn remote_fd(&self) -> i32 {
        self.remote_fd
    }
}

/// Handle for an open directory.
///
/// Stores the resolved path and lazily-loaded directory entries.
/// Entries are loaded on the first `getdents()` call and returned
/// in subsequent calls via an internal cursor.
pub struct DirectoryHandle {
    /// Absolute path of the directory in the VFS.
    path: String,
    /// Cached directory entries (populated on first read).
    entries: Option<Vec<crate::DirEntry>>,
    /// Cursor into `entries` for sequential reads.
    cursor: usize,
}

impl DirectoryHandle {
    /// Creates a new directory handle for the given VFS path.
    pub fn new(path: String) -> Self {
        Self {
            path,
            entries: None,
            cursor: 0,
        }
    }

    /// Returns the next batch of directory entries.
    ///
    /// Lazily loads entries from the VFS on the first call and returns
    /// up to `count` entries per invocation.
    pub fn read_entries(&mut self, count: usize) -> Result<Vec<crate::DirEntry>, Fat32Error> {
        if self.entries.is_none() {
            self.entries = Some(crate::read_dir(&self.path)?);
        }
        let all: &[crate::DirEntry] = self.entries.as_ref().unwrap();
        let remaining: &[crate::DirEntry] = if self.cursor < all.len() {
            &all[self.cursor..]
        } else {
            &[]
        };
        let take: usize = core::cmp::min(count, remaining.len());
        let batch: Vec<crate::DirEntry> = remaining[..take].to_vec();
        self.cursor += take;
        Ok(batch)
    }
}

impl VfsFileHandle {
    /// Reads data from the file.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.read(buf),
            VfsFileHandle::DirectRead(handle) => Ok(handle.read(buf)),
            VfsFileHandle::Directory(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::HostFs(_) => Err(Fat32Error::NotSupported),
            // Pipes are served by vfsd through the dedicated non-blocking primitives.
            VfsFileHandle::Pipe(_) => Err(Fat32Error::NotSupported),
            // Console and socket tokens are inert routing markers; vfsd serves no I/O on them.
            VfsFileHandle::Console(_) | VfsFileHandle::Socket(_) => Err(Fat32Error::NotSupported),
        }
    }

    /// Writes data to the file.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.write(buf),
            VfsFileHandle::DirectRead(_) => Err(Fat32Error::ReadOnly),
            VfsFileHandle::Directory(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::HostFs(_) => Err(Fat32Error::NotSupported),
            // Pipes are served by vfsd through the dedicated non-blocking primitives.
            VfsFileHandle::Pipe(_) => Err(Fat32Error::NotSupported),
            // Console and socket tokens are inert routing markers; vfsd serves no I/O on them.
            VfsFileHandle::Console(_) | VfsFileHandle::Socket(_) => Err(Fat32Error::NotSupported),
        }
    }

    /// Seeks to a position in the file.
    pub fn seek(&mut self, offset: off_t, whence: c_int) -> Result<off_t, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => {
                let pos: u64 = file.seek(whence, offset)?;
                Ok(pos as off_t)
            },
            VfsFileHandle::DirectRead(handle) => handle.seek(offset, whence),
            VfsFileHandle::Directory(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::HostFs(_) => Err(Fat32Error::NotSupported),
            // A pipe is not seekable (`ESPIPE`); the daemon rejects seeks before reaching here.
            VfsFileHandle::Pipe(_) => Err(Fat32Error::NotSupported),
            // Console and socket tokens are inert routing markers; vfsd serves no I/O on them.
            VfsFileHandle::Console(_) | VfsFileHandle::Socket(_) => Err(Fat32Error::NotSupported),
        }
    }

    /// Returns the file size in bytes.
    pub fn size(&mut self) -> Result<u64, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.size(),
            VfsFileHandle::DirectRead(handle) => Ok(handle.size() as u64),
            VfsFileHandle::Directory(_) => Ok(0),
            VfsFileHandle::HostFs(_) => Ok(0),
            VfsFileHandle::Pipe(_) => Ok(0),
            // Console and socket tokens have no size of their own.
            VfsFileHandle::Console(_) | VfsFileHandle::Socket(_) => Ok(0),
        }
    }

    /// Returns whether this handle is a directory.
    pub fn is_dir(&self) -> bool {
        match self {
            VfsFileHandle::Directory(_) => true,
            VfsFileHandle::HostFs(h) => h.is_dir(),
            _ => false,
        }
    }

    /// Returns whether this handle is backed by the host filesystem.
    pub fn is_hostfs(&self) -> bool {
        matches!(self, VfsFileHandle::HostFs(_))
    }

    /// Returns the remote FD if this is a HostFs handle.
    pub fn hostfs_remote_fd(&self) -> Option<i32> {
        match self {
            VfsFileHandle::HostFs(h) => Some(h.remote_fd()),
            _ => None,
        }
    }

    /// Returns the pipe end if this handle is one end of a pipe.
    pub fn pipe_end(&self) -> Option<&crate::pipe::PipeEnd> {
        match self {
            VfsFileHandle::Pipe(end) => Some(end),
            _ => None,
        }
    }
}

//==================================================================================================
// File Descriptor Table
//==================================================================================================

/// An open file description managed by the VFS.
///
/// Tracks a POSIX-compliant virtual position independently of the
/// underlying backend. This is necessary because FAT32 (via fatfs)
/// clamps seeks past EOF, while POSIX `lseek` allows it.
///
/// An open file description is shared (via [`Arc`]) by every file descriptor that refers to it.
/// `fork()` duplicates the file descriptors of a process by cloning these shared references, so the
/// parent and child observe the same file offset — matching POSIX semantics — while each holds an
/// independent descriptor that can be closed on its own.
struct VfsEntry {
    /// The file handle from any backend.
    handle: VfsFileHandle,
    /// POSIX-compliant virtual file position (may exceed file size).
    virtual_pos: off_t,
    /// Open file status flags (the mutable subset settable via `fcntl(F_SETFL)`).
    ///
    /// Only `O_NONBLOCK` is honored today; it is consulted by vfsd's pipe read/write handlers to
    /// choose `EAGAIN` over blocking. Non-pipe descriptors are unaffected because they never block.
    status_flags: c_int,
}

// SAFETY: VfsEntry contains FAT filesystem types that use `Cell` internally
// (e.g., `FsStatusFlags`), which prevents auto-impl of `Send`. This is safe
// because all access to VfsEntry goes through a `spin::Mutex`, ensuring
// exclusive access. The Cell is never shared across threads without the mutex.
unsafe impl Send for VfsEntry {}

/// A shared, reference-counted open file description.
///
/// Every file descriptor that refers to the same open file description holds one of these
/// references. The description is dropped — and its backend handle released — only when the last
/// referring descriptor is closed. This is the mechanism by which `fork()` shares open files
/// between a parent and its child.
type OpenFile = Arc<Mutex<VfsEntry>>;

/// Per-descriptor flags carried by a single file-descriptor slot.
///
/// These hold the POSIX `fcntl(F_GETFD/F_SETFD)` descriptor flags — `FD_CLOEXEC` and `FD_CLOFORK`.
/// They are stored on the slot rather than on the shared [`VfsEntry`] because POSIX requires them to
/// be *per descriptor*, not per open file description: two descriptors that share one description
/// through `dup` (or `fork`) each carry an independent copy, so setting `FD_CLOEXEC` on one must not
/// affect the other.
///
/// The default is empty (no flags set), so a freshly allocated descriptor behaves exactly as it did
/// before these flags existed. The flags are stored and cloned but not yet acted upon — honoring
/// `FD_CLOFORK` at fork and `FD_CLOEXEC` at exec lands in later plans.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FdFlags(c_int);

impl FdFlags {
    /// Returns whether the close-on-exec (`FD_CLOEXEC`) flag is set.
    pub const fn close_on_exec(self) -> bool {
        self.0 & FD_CLOEXEC != 0
    }

    /// Returns whether the close-on-fork (`FD_CLOFORK`) flag is set.
    pub const fn close_on_fork(self) -> bool {
        self.0 & FD_CLOFORK != 0
    }

    /// Sets or clears the close-on-exec (`FD_CLOEXEC`) flag.
    pub fn set_close_on_exec(&mut self, enable: bool) {
        self.set(FD_CLOEXEC, enable);
    }

    /// Sets or clears the close-on-fork (`FD_CLOFORK`) flag.
    pub fn set_close_on_fork(&mut self, enable: bool) {
        self.set(FD_CLOFORK, enable);
    }

    /// Sets or clears `flag` according to `enable`.
    fn set(&mut self, flag: c_int, enable: bool) {
        if enable {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }
}

/// A single file-descriptor slot: a reference to an open file description plus the per-descriptor
/// flags that belong to this descriptor alone.
///
/// The flags live here, not on the shared [`VfsEntry`], so that descriptors which share one open
/// file description through `dup` or `fork` keep independent `FD_CLOEXEC`/`FD_CLOFORK` settings.
#[derive(Clone)]
struct Slot {
    /// The shared open file description this descriptor refers to.
    file: OpenFile,
    /// Per-descriptor flags (`FD_CLOEXEC`, `FD_CLOFORK`).
    fd_flags: FdFlags,
}

impl Slot {
    /// Creates a slot referring to `file` with default (empty) descriptor flags.
    fn new(file: OpenFile) -> Self {
        Self {
            file,
            fd_flags: FdFlags::default(),
        }
    }
}

/// Per-process VFS state: the open file descriptor table and the current working directory.
///
/// Each process is given its own descriptor table so that closing a descriptor in one process does
/// not affect another, and its own working directory so that `chdir()` is process-local. `fork()`
/// gives the child a copy of this state: the descriptor slots are cloned as shared references to
/// the parent's open file descriptions, while the working directory is deep-copied.
struct ProcessState {
    /// File descriptor slots indexed by `(fd - VFS_FD_BASE)`.
    slots: Vec<Option<Slot>>,
    /// Current working directory (always absolute, never ends with "/").
    cwd: String,
}

impl ProcessState {
    /// Creates a new, empty per-process state whose working directory defaults to [`DEFAULT_CWD`].
    fn new() -> Self {
        Self {
            slots: alloc::vec![None; VFS_MAX_OPEN_FILES],
            cwd: String::from(DEFAULT_CWD),
        }
    }

    /// Creates a copy of this state for a freshly forked child.
    ///
    /// Each descriptor slot is cloned: the open file description is shared as a reference (so the
    /// parent and child share file offsets) while the per-descriptor flags are copied, giving the
    /// child its own independent `FD_CLOEXEC`/`FD_CLOFORK` settings. The working directory is
    /// deep-copied.
    fn fork(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            cwd: self.cwd.clone(),
        }
    }

    /// Reports whether this state holds no open file descriptors.
    ///
    /// A freshly created [`ProcessState`] is empty until a descriptor is allocated. This is used to
    /// recognize the placeholder state that [`set_current_process`] inserts lazily when a request
    /// arrives for a process the VFS has not seen before, so that a later fork-clone may safely
    /// overwrite it without orphaning any host-backed remote handles.
    fn is_empty(&self) -> bool {
        self.slots.iter().all(Option::is_none)
    }
}

/// Registry of per-process VFS state, keyed by process identifier.
static PROCESSES: Mutex<BTreeMap<ProcessIdentifier, ProcessState>> = Mutex::new(BTreeMap::new());

/// Identifier of the process on whose behalf the VFS is currently operating.
///
/// The daemon sets this before handling each request so that the descriptor and working-directory
/// operations below resolve against the correct process. The default (`0`) provides a single
/// implicit process for callers — such as unit tests and benchmarks — that never set it explicitly.
///
/// This is process-global. Any unit test that mutates it (directly or via [`set_current_process`])
/// or that depends on the per-process registry must serialize against every other such test by
/// holding `FORK_TEST_GUARD`, since the test harness runs tests concurrently.
static CURRENT_PID: AtomicI32 = AtomicI32::new(0);

/// Returns the identifier of the process the VFS is currently operating on behalf of.
#[inline]
fn current_pid() -> ProcessIdentifier {
    ProcessIdentifier::from(CURRENT_PID.load(Ordering::Relaxed))
}

/// Selects the process on whose behalf subsequent VFS operations are performed.
///
/// The daemon must call this before dispatching a request so that descriptor and working-directory
/// operations resolve against the requesting process. The process's state is created lazily on
/// first use if it does not already exist.
pub fn set_current_process(pid: ProcessIdentifier) {
    {
        let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
            PROCESSES.lock();
        procs.entry(pid).or_insert_with(ProcessState::new);
    }
    CURRENT_PID.store(i32::from(pid), Ordering::Relaxed);
}

/// Returns the working directory of the process the VFS is operating on behalf of.
///
/// The working directory is owned solely by the per-process state; the VFS itself stores none.
/// Defaults to "/" if the process has no recorded state yet.
pub(crate) fn current_cwd() -> String {
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    procs
        .get(&current_pid())
        .map(|state| state.cwd.clone())
        .unwrap_or_else(|| String::from("/"))
}

/// Translates a file descriptor into a slot index, validating its range.
fn fd_index(fd: c_int) -> Result<usize, Fat32Error> {
    if fd < VFS_FD_BASE {
        return Err(Fat32Error::InvalidFd);
    }
    let idx: usize = (fd - VFS_FD_BASE) as usize;
    if idx >= VFS_MAX_OPEN_FILES {
        return Err(Fat32Error::InvalidFd);
    }
    Ok(idx)
}

/// Returns a shared reference to the open file description for `fd` in the current process.
fn entry_arc(fd: c_int) -> Result<OpenFile, Fat32Error> {
    let idx: usize = fd_index(fd)?;
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    let state: &ProcessState = procs.get(&current_pid()).ok_or(Fat32Error::InvalidFd)?;
    state
        .slots
        .get(idx)
        .and_then(|slot| slot.as_ref())
        .map(|slot| slot.file.clone())
        .ok_or(Fat32Error::InvalidFd)
}

/// Allocates a new file descriptor for the given handle in the current process.
fn alloc_fd(handle: VfsFileHandle) -> Result<c_int, Fat32Error> {
    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    let state: &mut ProcessState = procs.entry(current_pid()).or_insert_with(ProcessState::new);
    for i in 0..VFS_MAX_OPEN_FILES {
        if state.slots[i].is_none() {
            let file: OpenFile = Arc::new(Mutex::new(VfsEntry {
                handle,
                virtual_pos: 0,
                status_flags: 0,
            }));
            state.slots[i] = Some(Slot::new(file));
            return Ok(VFS_FD_BASE + i as c_int);
        }
    }
    Err(Fat32Error::TooManyOpenFiles)
}

/// Records the current working directory for the process the VFS is operating on behalf of.
///
/// This persists the directory in the per-process registry so that it is restored by
/// [`set_current_process`] on the process's next request and copied to children by
/// [`vfs_fork_clone`].
pub(crate) fn set_current_cwd(cwd: String) {
    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    procs
        .entry(current_pid())
        .or_insert_with(ProcessState::new)
        .cwd = cwd;
}

/// Duplicates the filesystem state of `parent` onto a freshly forked `child`.
///
/// The child inherits a copy of the parent's open file descriptors — sharing the underlying open
/// file descriptions, and therefore file offsets, as POSIX requires — together with a private copy
/// of the parent's current working directory.
///
/// `procd` owns process lifecycle: it registers every process with the VFS when the process is
/// created and is the only component permitted to do so. A fork is therefore always cloned from a
/// parent that `procd` has already registered.
///
/// # Errors
///
/// - [`Fat32Error::NotFound`] if `parent` has no recorded state. Because `procd` registers a
///   process before it can ever fork, a missing parent is a lifecycle contract violation rather
///   than a recoverable condition, and the child is left unregistered. The `child` state is
///   examined before the `parent`, so a `child` that already holds open descriptors yields
///   [`Fat32Error::AlreadyExists`] even when `parent` is missing as well.
/// - [`Fat32Error::AlreadyExists`] if `child` already has recorded state that holds open file
///   descriptors. A forked child must be a fresh process; overwriting a populated table would drop
///   its open file descriptions and leak any host-backed remote handles they hold. The caller must
///   reclaim that state first (e.g., via [`vfs_process_exit`]). An empty placeholder state — which
///   [`set_current_process`] inserts lazily when the child's first request races ahead of this
///   fork-clone notification — holds no descriptors and is overwritten in place; any working
///   directory the child set via `chdir` before the notification arrived is preserved rather than
///   reverted to the parent's.
pub fn vfs_fork_clone(
    parent: ProcessIdentifier,
    child: ProcessIdentifier,
) -> Result<(), Fat32Error> {
    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    // The child's first request can reach the VFS before procd's fork-clone notification, in which
    // case `set_current_process` has already inserted an empty placeholder state for it. That
    // placeholder holds no descriptors, so it is safe to overwrite. Refuse only when the existing
    // state actually holds open descriptors, since clobbering those would orphan any host-backed
    // remote handles they reference.
    //
    // The placeholder can still carry a working directory: the racing child may have issued a
    // `chdir` before this notification arrived. Capture a directory that differs from the default
    // so the clone below keeps the child's own cwd instead of reverting it to the parent's.
    let mut child_cwd: Option<String> = None;
    if let Some(existing) = procs.get(&child) {
        if !existing.is_empty() {
            return Err(Fat32Error::AlreadyExists);
        }
        if existing.cwd != DEFAULT_CWD {
            child_cwd = Some(existing.cwd.clone());
        }
    }
    // The parent must already be registered. `procd` registers every process at creation and is the
    // sole authority for doing so, so forking from an unregistered parent is a contract violation:
    // surface it rather than fabricating a default child that would mask the bug.
    let mut child_state: ProcessState = procs.get(&parent).ok_or(Fat32Error::NotFound)?.fork();
    // Honor a working directory the child established before the fork-clone notification arrived.
    if let Some(cwd) = child_cwd {
        child_state.cwd = cwd;
    }
    procs.insert(child, child_state);
    Ok(())
}

/// Pipe count-to-zero transitions surfaced by [`vfs_process_exit`].
///
/// Each entry records that the exiting process held the final reference to one end of a pipe, so
/// that end's reference count dropped to zero when its open file description was released. vfsd
/// uses these to run the corresponding wakeup (EOF for readers when a write end vanishes, `EPIPE`
/// for writers when a read end vanishes).
pub struct PipeClosure {
    /// Stable identity of the affected pipe.
    pub pipe_id: u64,
    /// Whether the released end was the write end (`true`) or the read end (`false`).
    pub was_write: bool,
}

/// Resources surfaced by [`vfs_process_exit`] that the daemon must act on after a process exits.
pub struct ProcessExitReclaim {
    /// Remote file descriptors of host-backed descriptions for which the process held the final
    /// reference. Each must be closed on hostfsd or the remote handle leaks.
    pub orphaned_hostfs_fds: Vec<i32>,
    /// Pipe ends whose reference count reached zero because the process held the final reference.
    pub pipe_closures: Vec<PipeClosure>,
}

/// Reclaims the per-process filesystem state of a terminated process.
///
/// Drops `pid`'s recorded state, releasing its references to the open file descriptions it held.
/// This keeps the last-reference accounting correct for surviving siblings that still share those
/// descriptions, and prevents the per-process table from growing without bound. Reclaiming an
/// unknown `pid` is a no-op.
///
/// Returns the resources the daemon must act on: the remote file descriptors of any host-backed
/// open file descriptions for which `pid` held the final reference (which it must close on hostfsd,
/// because the process is gone and can no longer close them itself), and the pipe ends whose
/// reference count reached zero (so the daemon can fire EOF/`EPIPE` wakeups for any suspended
/// counterparts). Descriptions still shared with a surviving process are not returned.
#[must_use = "the returned hostfs fds must be closed and pipe closures must trigger wakeups"]
pub fn vfs_process_exit(pid: ProcessIdentifier) -> ProcessExitReclaim {
    // Detach the process's state while holding the registry lock, but defer dropping it until the
    // lock is released. Dropping an open file description may run backend teardown; deferring keeps
    // the registry lock from ever nesting with backend locks, matching `vfs_close`.
    let removed: Option<ProcessState> = {
        let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
            PROCESSES.lock();
        procs.remove(&pid)
    };
    let Some(state) = removed else {
        return ProcessExitReclaim {
            orphaned_hostfs_fds: Vec::new(),
            pipe_closures: Vec::new(),
        };
    };
    // The process has been removed from the registry, so an `Arc` strong count of one means no
    // surviving descriptor — in this or any other process — still shares the open file description.
    // A host-backed handle must therefore be closed on hostfsd, and a pipe end's count drops to
    // zero (which the dropped end's `Drop` applies just below). As the sole owner, the lock is
    // uncontended.
    let mut orphaned: Vec<i32> = Vec::new();
    let mut pipe_closures: Vec<PipeClosure> = Vec::new();
    for slot in state.slots.into_iter().flatten() {
        if Arc::strong_count(&slot.file) != 1 {
            continue;
        }
        match &slot.file.lock().handle {
            VfsFileHandle::HostFs(h) => orphaned.push(h.remote_fd()),
            VfsFileHandle::Pipe(end) => pipe_closures.push(PipeClosure {
                pipe_id: end.pipe_id(),
                was_write: end.is_write(),
            }),
            // A console token holds no external resource, so it contributes nothing to reclaim.
            VfsFileHandle::Console(_) => {},
            // A socket token holds the `networkd` descriptor, but closing it on last reference is
            // wired in a later plan; nothing is reclaimed here yet. This arm must stay distinct so
            // a socket is never mistaken for a hostfs or pipe handle.
            VfsFileHandle::Socket(_) => {},
            _ => {},
        }
        // `slot` — and the open file description it holds — is dropped here, after the registry lock
        // has been released; the pipe end's `Drop` decrements its reader/writer count as part of
        // that.
    }
    ProcessExitReclaim {
        orphaned_hostfs_fds: orphaned,
        pipe_closures,
    }
}

/// Reports whether `fd` is the last descriptor referencing its open file description.
///
/// Returns `true` if closing `fd` in the current process would drop the final reference to the
/// underlying open file description (so a host-backed close must be forwarded to hostfsd), and
/// `false` if other descriptors — for example in a forked child — still share it, or if `fd` is
/// invalid. This does not modify the descriptor table.
pub fn vfs_hostfs_is_last_ref(fd: c_int) -> bool {
    let Ok(idx) = fd_index(fd) else {
        return false;
    };
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    let Some(state) = procs.get(&current_pid()) else {
        return false;
    };
    match state.slots.get(idx).and_then(|slot| slot.as_ref()) {
        // The table holds exactly one reference, so a strong count of one means no other descriptor
        // shares this open file description.
        Some(slot) => Arc::strong_count(&slot.file) == 1,
        None => false,
    }
}

/// Reports whether closing `fd` would drop the final reference to a pipe end.
///
/// Returns `true` when `fd` refers to a pipe end and the current process holds the only descriptor
/// for that end's open file description, so closing it (or the owning process exiting) decrements
/// the pipe's reader/writer count to zero. Returns `false` when other descriptors still share the
/// end (for example in a forked child), when `fd` is not a pipe, or when `fd` is invalid. This does
/// not modify the descriptor table.
pub fn vfs_pipe_is_last_ref(fd: c_int) -> bool {
    let Ok(idx) = fd_index(fd) else {
        return false;
    };
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    let Some(state) = procs.get(&current_pid()) else {
        return false;
    };
    match state.slots.get(idx).and_then(|slot| slot.as_ref()) {
        Some(slot) => {
            // Must be a pipe and the sole reference for closing to drive the count to zero.
            Arc::strong_count(&slot.file) == 1
                && matches!(&slot.file.lock().handle, VfsFileHandle::Pipe(_))
        },
        None => false,
    }
}

//==================================================================================================
// HostFs FD Helpers
//==================================================================================================

/// Allocates a VFS file descriptor for a host filesystem handle.
///
/// This is called by vfsd after receiving a successful OPEN response from hostfsd.
/// The returned FD is handed back to the user process.
pub fn vfs_alloc_hostfs(
    remote_fd: i32,
    is_dir: bool,
    path: Option<String>,
) -> Result<c_int, Fat32Error> {
    let handle: VfsFileHandle = VfsFileHandle::HostFs(HostFsHandle::new(remote_fd, is_dir, path));
    alloc_fd(handle)
}

/// Returns the remote FD for a HostFs file descriptor, or `None` if the FD
/// is not backed by hostfs.
pub fn vfs_hostfs_remote_fd(fd: c_int) -> Option<i32> {
    let file: OpenFile = entry_arc(fd).ok()?;
    let guard = file.lock();
    guard.handle.hostfs_remote_fd()
}

/// Returns `true` if the given FD is backed by the host filesystem.
pub fn is_hostfs_fd(fd: c_int) -> bool {
    vfs_hostfs_remote_fd(fd).is_some()
}

/// Returns the current directory iteration cursor for a hostfs directory FD.
///
/// Returns `None` if the FD is not a hostfs directory handle (including hostfs
/// regular files), so callers can use this to distinguish hostfs directories from
/// hostfs files.
pub fn vfs_hostfs_readdir_offset(fd: c_int) -> Option<u32> {
    let file: OpenFile = entry_arc(fd).ok()?;
    let guard = file.lock();
    match &guard.handle {
        VfsFileHandle::HostFs(h) if h.is_dir() => Some(h.readdir_offset()),
        _ => None,
    }
}

/// Updates the directory iteration cursor for a hostfs directory FD.
///
/// Returns `true` if the FD is a hostfs directory handle and the cursor was updated.
/// The cursor is left untouched for non-directory hostfs handles (regular files).
pub fn vfs_hostfs_set_readdir_offset(fd: c_int, offset: u32) -> bool {
    let Ok(file) = entry_arc(fd) else {
        return false;
    };
    let mut guard = file.lock();
    match &mut guard.handle {
        VfsFileHandle::HostFs(h) if h.is_dir() => {
            h.set_readdir_offset(offset);
            true
        },
        _ => false,
    }
}

//==================================================================================================
// Pipe FD Helpers
//==================================================================================================

/// Re-exported pipe outcome types so the daemon can match on them without importing
/// [`crate::pipe`] directly.
pub use crate::pipe::{
    PipeReadOutcome,
    PipeWriteOutcome,
};

/// Creates a pipe and allocates its read and write file descriptors in the current process.
///
/// Returns `(read_fd, write_fd)`. The two descriptors share a single pipe buffer with the reader
/// and writer counts both initialized to one.
///
/// # Errors
///
/// Returns [`Fat32Error::TooManyOpenFiles`] when the descriptor table cannot accommodate both ends.
/// If only the read end could be allocated, it is released before returning so that no half-open
/// pipe is left behind.
pub fn vfs_pipe() -> Result<(c_int, c_int), Fat32Error> {
    let (read_end, write_end): (crate::pipe::PipeEnd, crate::pipe::PipeEnd) =
        crate::pipe::PipeEnd::new_pair();
    let read_fd: c_int = alloc_fd(VfsFileHandle::Pipe(read_end))?;
    match alloc_fd(VfsFileHandle::Pipe(write_end)) {
        Ok(write_fd) => Ok((read_fd, write_fd)),
        Err(e) => {
            // Releasing the read end drops its `PipeEnd`, decrementing the reader count; the write
            // end was already dropped when `alloc_fd` consumed and failed on it. No half-open pipe
            // survives.
            let _ = vfs_close(read_fd);
            Err(e)
        },
    }
}

/// Returns a pipe descriptor's identity and direction, or `None` if `fd` is not a pipe.
///
/// The returned tuple is `(pipe_id, is_write)`. vfsd uses `pipe_id` to key its blocked-request
/// queues and `is_write` to enforce I/O direction.
pub fn vfs_pipe_id(fd: c_int) -> Option<(u64, bool)> {
    let file: OpenFile = entry_arc(fd).ok()?;
    let guard = file.lock();
    let end: &crate::pipe::PipeEnd = guard.handle.pipe_end()?;
    Some((end.pipe_id(), end.is_write()))
}

/// Attempts a non-blocking read from a pipe read end.
///
/// On [`PipeReadOutcome::Read(n)`](PipeReadOutcome::Read), `n` bytes of space were freed, so vfsd
/// must try to revive a suspended writer.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidFd`] if `fd` is not a pipe or if it is the write end (reading the
/// write end is rejected, mirroring `EBADF`).
pub fn vfs_pipe_read(fd: c_int, buf: &mut [u8]) -> Result<PipeReadOutcome, Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let guard = file.lock();
    let end: &crate::pipe::PipeEnd = guard.handle.pipe_end().ok_or(Fat32Error::InvalidFd)?;
    end.read(buf).map_err(|_| Fat32Error::InvalidFd)
}

/// Attempts a non-blocking write to a pipe write end, honoring `PIPE_BUF` atomicity.
///
/// On [`PipeWriteOutcome::Wrote(n)`](PipeWriteOutcome::Wrote), `n` bytes became available, so vfsd
/// must try to revive a suspended reader.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidFd`] if `fd` is not a pipe or if it is the read end (writing the
/// read end is rejected, mirroring `EBADF`).
pub fn vfs_pipe_write(fd: c_int, buf: &[u8]) -> Result<PipeWriteOutcome, Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let guard = file.lock();
    let end: &crate::pipe::PipeEnd = guard.handle.pipe_end().ok_or(Fat32Error::InvalidFd)?;
    end.write(buf).map_err(|_| Fat32Error::InvalidFd)
}

/// Returns the open file status flags for `fd` (as reported by `fcntl(F_GETFL)`).
///
/// Returns `0` if `fd` is invalid, which is harmless: callers consult this only to decide whether
/// `O_NONBLOCK` is set, and a missing descriptor will fail the surrounding operation anyway.
pub fn vfs_get_status_flags(fd: c_int) -> c_int {
    match entry_arc(fd) {
        Ok(file) => file.lock().status_flags,
        Err(_) => 0,
    }
}

//==================================================================================================
// Per-Descriptor Flag Helpers
//==================================================================================================

/// Returns the per-descriptor flags (`FD_CLOEXEC`/`FD_CLOFORK`) recorded for `fd` in the current
/// process, or `None` if `fd` is invalid or refers to no open descriptor.
///
/// These are distinct from the open file status flags read by [`vfs_get_status_flags`]: they are
/// stored per descriptor, so descriptors that share one open file description through `dup` or
/// `fork` report them independently.
pub fn vfs_get_fd_flags(fd: c_int) -> Option<FdFlags> {
    let idx: usize = fd_index(fd).ok()?;
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    let state: &ProcessState = procs.get(&current_pid())?;
    Some(state.slots.get(idx)?.as_ref()?.fd_flags)
}

/// Sets the per-descriptor flags (`FD_CLOEXEC`/`FD_CLOFORK`) for `fd` in the current process.
///
/// Returns [`Fat32Error::InvalidFd`] if `fd` is invalid or refers to no open descriptor. Because the
/// flags are stored per descriptor, updating one descriptor does not affect another that shares the
/// same open file description.
pub fn vfs_set_fd_flags(fd: c_int, flags: FdFlags) -> Result<(), Fat32Error> {
    let idx: usize = fd_index(fd)?;
    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    let state: &mut ProcessState = procs.get_mut(&current_pid()).ok_or(Fat32Error::InvalidFd)?;
    let slot: &mut Slot = state
        .slots
        .get_mut(idx)
        .and_then(|slot| slot.as_mut())
        .ok_or(Fat32Error::InvalidFd)?;
    slot.fd_flags = flags;
    Ok(())
}

//==================================================================================================
// Path Routing
//==================================================================================================

/// Returns `true` if the given path is handled by the VFS.
pub fn is_vfs_path(path: &str) -> bool {
    fat32_backend::exists(path)
}

/// Returns `true` if the given file descriptor belongs to the VFS.
pub fn is_vfs_fd(fd: c_int) -> bool {
    ::config::fds::is_vfs_fd(fd)
}

/// Resolves a `dirfd` + `path` pair into an absolute VFS path.
///
/// If `path` is absolute, it is returned as-is (dirfd is ignored per POSIX).
/// If `dirfd` is `AT_FDCWD`, the path is resolved against the VFS current
/// working directory. If `dirfd` is a VFS directory fd, the path is resolved
/// relative to that directory's path.
///
/// Returns `None` if `dirfd` is not a VFS fd and not `AT_FDCWD`, indicating
/// that VFS cannot handle this request.
///
/// # Limitations
///
/// For hostfs directory fds, resolution uses the path stored at open time.
/// If the directory is renamed after being opened, subsequent `*at()` calls
/// using this dirfd will resolve against the stale path. A future protocol
/// extension could support `*at()` operations relative to a remote directory
/// FD on the host side to provide stable POSIX-like dirfd semantics.
pub fn vfs_resolve_path(dirfd: c_int, path: &str) -> Option<String> {
    use ::sysapi::fcntl::atflags::AT_FDCWD;

    // Absolute paths are always resolved directly (dirfd ignored per POSIX).
    if path.starts_with('/') {
        return Some(String::from(path));
    }

    // Relative path with AT_FDCWD: resolve against VFS cwd.
    if dirfd == AT_FDCWD {
        let cwd: String = crate::cwd().ok()?;
        return if cwd.ends_with('/') {
            Some(alloc::format!("{}{}", cwd, path))
        } else {
            Some(alloc::format!("{}/{}", cwd, path))
        };
    }

    // Relative path with a VFS directory fd: resolve against that directory.
    if !is_vfs_fd(dirfd) {
        return None;
    }

    let file: OpenFile = entry_arc(dirfd).ok()?;
    let guard = file.lock();
    let dir_path: &str = match &guard.handle {
        VfsFileHandle::Directory(dh) => &dh.path,
        VfsFileHandle::HostFs(hh) if hh.is_dir() => hh.path()?,
        _ => return None, // fd is not a directory
    };

    if dir_path.ends_with('/') {
        Some(alloc::format!("{}{}", dir_path, path))
    } else {
        Some(alloc::format!("{}/{}", dir_path, path))
    }
}

//==================================================================================================
// POSIX-Compatible Operations
//==================================================================================================

/// Opens a file through the VFS and allocates a system-wide FD.
pub fn vfs_open(path: &str, flags: c_int) -> Result<c_int, Fat32Error> {
    // If O_DIRECTORY is set, verify the path is a directory before opening.
    if flags & file_creation_flags::O_DIRECTORY != 0 {
        let info: VfsStat = fat32_backend::stat(path)?;
        if !info.is_dir() {
            return Err(Fat32Error::NotADirectory);
        }
        let normalized: String = crate::normalize(path)?;
        let handle: VfsFileHandle = VfsFileHandle::Directory(DirectoryHandle::new(normalized));
        return alloc_fd(handle);
    }
    let handle: VfsFileHandle = fat32_backend::open(path, flags)?;
    alloc_fd(handle)
}

/// Opens a file relative to a directory file descriptor through the VFS.
///
/// The `path` is resolved against `dirfd` via [`vfs_resolve_path`] and the
/// resulting path is opened with [`vfs_open`].
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidArgument`] if `dirfd` cannot be resolved (for
/// example, when it is not a VFS directory file descriptor). The `dirfd` is
/// never silently ignored.
pub fn vfs_openat(dirfd: c_int, path: &str, flags: c_int) -> Result<c_int, Fat32Error> {
    let resolved: String = vfs_resolve_path(dirfd, path).ok_or(Fat32Error::InvalidArgument)?;
    vfs_open(&resolved, flags)
}

/// Reads from a VFS file descriptor.
///
/// Uses the virtual position tracker. If the position is at or past EOF,
/// returns 0 (POSIX EOF semantics). Otherwise syncs the handle, reads,
/// and advances the virtual position.
pub fn vfs_read(fd: c_int, buf: &mut [u8]) -> Result<c_size_t, Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;

    if entry.handle.is_dir() {
        return Err(Fat32Error::NotAFile);
    }

    let size: u64 = entry.handle.size()?;
    if entry.virtual_pos as u64 >= size {
        return Ok(0);
    }

    // Sync handle to virtual position, read, advance.
    entry.handle.seek(entry.virtual_pos, file_seek::SEEK_SET)?;
    let n: usize = entry.handle.read(buf)?;
    entry.virtual_pos += n as off_t;
    Ok(n as c_size_t)
}

/// Writes to a VFS file descriptor.
///
/// Uses the virtual position tracker. If the position is past EOF, extends
/// the file with zeros first, then writes and advances the virtual position.
pub fn vfs_write(fd: c_int, buf: &[u8]) -> Result<c_size_t, Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;

    let size: u64 = entry.handle.size()?;
    if (entry.virtual_pos as u64) > size {
        // Extend file with zeros up to virtual_pos.
        entry.handle.seek(0, file_seek::SEEK_END)?;
        let gap: usize = (entry.virtual_pos as u64 - size) as usize;
        let zeros: [u8; 512] = [0u8; 512];
        let mut remaining: usize = gap;
        while remaining > 0 {
            let chunk: usize = core::cmp::min(remaining, zeros.len());
            let written: usize = entry.handle.write(&zeros[..chunk])?;
            if written == 0 {
                return Err(Fat32Error::NoSpace);
            }
            remaining -= written;
        }
    }

    // Sync handle to virtual position, write, advance.
    entry.handle.seek(entry.virtual_pos, file_seek::SEEK_SET)?;
    let n: usize = entry.handle.write(buf)?;
    entry.virtual_pos += n as off_t;
    Ok(n as c_size_t)
}

/// Seeks a VFS file descriptor.
///
/// Computes the new position according to POSIX semantics (past-EOF seeks
/// are allowed) and stores it in the entry's virtual position tracker. The
/// underlying backend handle is only synced when the position is within the
/// file bounds.
pub fn vfs_lseek(fd: c_int, offset: off_t, whence: c_int) -> Result<off_t, Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;

    let size: i64 = entry.handle.size()? as i64;
    let new_pos: i64 = match whence {
        file_seek::SEEK_SET => offset,
        file_seek::SEEK_CUR => entry.virtual_pos + offset,
        file_seek::SEEK_END => size + offset,
        _ => return Err(Fat32Error::InvalidArgument),
    };
    if new_pos < 0 {
        return Err(Fat32Error::InvalidSeek);
    }

    entry.virtual_pos = new_pos;

    // Sync the underlying handle when within file bounds.
    if new_pos <= size {
        let _ = entry.handle.seek(new_pos, file_seek::SEEK_SET);
    }

    Ok(new_pos)
}

/// Populates common stat fields for VFS entries.
///
/// FAT32 lacks Unix metadata so we use sensible defaults:
/// - `st_nlink = 1` (single link).
/// - Timestamps set to a fixed epoch value (FAT has no sub-second precision).
/// - Permissions: owner read+write for files, owner rwx for directories.
fn populate_stat_fields(buf: &mut ::sysapi::sys_stat::stat, size: u64, is_dir: bool) {
    // Fixed epoch timestamp: 2024-01-01T00:00:00Z (1704067200).
    const FIXED_EPOCH: i64 = 1_704_067_200;

    buf.st_size = size as off_t;
    buf.st_nlink = if is_dir { 2 } else { 1 };
    buf.st_dev = 1; // Synthetic device ID for the VFS.
    buf.st_ino = 1; // Synthetic inode (FAT has no inodes).
    buf.st_mode = if is_dir {
        file_type::S_IFDIR | file_mode::S_IRWXU
    } else {
        file_type::S_IFREG | file_mode::S_IRUSR | file_mode::S_IWUSR
    };
    buf.st_blksize = STAT_BLOCK_SIZE;
    buf.st_blocks = size.div_ceil(STAT_SECTOR_SIZE) as off_t;
    buf.st_atim = timespec {
        tv_sec: FIXED_EPOCH,
        tv_nsec: 0,
    };
    buf.st_mtim = timespec {
        tv_sec: FIXED_EPOCH,
        tv_nsec: 0,
    };
    buf.st_ctim = timespec {
        tv_sec: FIXED_EPOCH,
        tv_nsec: 0,
    };
}

/// Populates stat fields for a pipe (FIFO) descriptor.
///
/// A pipe has no name, size, or on-disk blocks: `st_mode` carries `S_IFIFO` with owner read/write
/// permissions and `st_size` is `0`, matching POSIX expectations for an unnamed pipe. `st_ino`
/// carries the pipe's unique identity, and `st_dev` a synthetic pipefs device distinct from the
/// VFS file device, so distinct pipes report distinct `(st_dev, st_ino)` pairs that never collide
/// with regular files — mirroring how a real pipefs assigns one inode per pipe.
fn populate_pipe_stat_fields(buf: &mut ::sysapi::sys_stat::stat, pipe_id: u64) {
    // Fixed epoch timestamp: 2024-01-01T00:00:00Z (1704067200).
    const FIXED_EPOCH: i64 = 1_704_067_200;

    buf.st_size = 0;
    buf.st_nlink = 1;
    buf.st_dev = 2; // Synthetic pipefs device ID, distinct from the VFS file device (1).
    buf.st_ino = pipe_id;
    buf.st_mode = file_type::S_IFIFO | file_mode::S_IRUSR | file_mode::S_IWUSR;
    buf.st_blksize = STAT_BLOCK_SIZE;
    buf.st_blocks = 0;
    buf.st_atim = timespec {
        tv_sec: FIXED_EPOCH,
        tv_nsec: 0,
    };
    buf.st_mtim = timespec {
        tv_sec: FIXED_EPOCH,
        tv_nsec: 0,
    };
    buf.st_ctim = timespec {
        tv_sec: FIXED_EPOCH,
        tv_nsec: 0,
    };
}

/// Gets file status for a VFS file descriptor.
pub fn vfs_fstat(fd: c_int, buf: &mut ::sysapi::sys_stat::stat) -> Result<(), Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;
    let pipe_id: Option<u64> = entry.handle.pipe_end().map(|end| end.pipe_id());
    let is_dir: bool = matches!(&entry.handle, VfsFileHandle::Directory(_));
    let size: u64 = entry.handle.size()?;

    // Zero-initialize the stat buffer.
    unsafe {
        ::core::ptr::write_bytes(buf as *mut ::sysapi::sys_stat::stat, 0, 1);
    }

    if let Some(pipe_id) = pipe_id {
        populate_pipe_stat_fields(buf, pipe_id);
    } else {
        populate_stat_fields(buf, size, is_dir);
    }

    Ok(())
}

/// Closes a VFS file descriptor.
pub fn vfs_close(fd: c_int) -> Result<(), Fat32Error> {
    let idx: usize = fd_index(fd)?;
    // Detach the descriptor while holding the registry lock, but defer dropping the open file
    // description until the lock is released. Its drop may run backend teardown (e.g. `File::drop`,
    // which takes a global VFS lock); deferring keeps the registry lock from ever nesting with
    // backend locks.
    let removed: Option<Slot> = {
        let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
            PROCESSES.lock();
        let state: &mut ProcessState =
            procs.get_mut(&current_pid()).ok_or(Fat32Error::InvalidFd)?;
        let slot: &mut Option<Slot> = state.slots.get_mut(idx).ok_or(Fat32Error::InvalidFd)?;
        slot.take()
    };
    match removed {
        // The slot — and the open file description it holds — is dropped here, after the registry
        // lock has been released.
        Some(_slot) => Ok(()),
        None => Err(Fat32Error::InvalidFd),
    }
}

/// Gets file status for a path through the VFS.
pub fn vfs_stat(path: &str, buf: &mut ::sysapi::sys_stat::stat) -> Result<(), Fat32Error> {
    let info: VfsStat = fat32_backend::stat(path)?;

    // Zero-initialize the stat buffer.
    unsafe {
        ::core::ptr::write_bytes(buf as *mut ::sysapi::sys_stat::stat, 0, 1);
    }

    populate_stat_fields(buf, info.size(), info.is_dir());

    Ok(())
}

/// Gets file status for a path relative to a directory file descriptor through the VFS.
///
/// The `path` is resolved against `dirfd` via [`vfs_resolve_path`] and the
/// resulting path is queried with [`vfs_stat`].
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidArgument`] if `dirfd` cannot be resolved (for
/// example, when it is not a VFS directory file descriptor). The `dirfd` is
/// never silently ignored.
pub fn vfs_fstatat(
    dirfd: c_int,
    path: &str,
    buf: &mut ::sysapi::sys_stat::stat,
) -> Result<(), Fat32Error> {
    let resolved: String = vfs_resolve_path(dirfd, path).ok_or(Fat32Error::InvalidArgument)?;
    vfs_stat(&resolved, buf)
}

/// Renames a file or directory through the VFS.
///
/// Both paths must be on the same VFS mount.
pub fn vfs_rename(old_path: &str, new_path: &str) -> Result<(), Fat32Error> {
    crate::rename(old_path, new_path)
}

/// Deletes a file through the VFS.
pub fn vfs_unlink(path: &str) -> Result<(), Fat32Error> {
    crate::unlink(path)
}

/// Creates a directory through the VFS.
pub fn vfs_mkdir(path: &str) -> Result<(), Fat32Error> {
    crate::mkdir(path)
}

/// Creates a directory relative to a directory file descriptor through the VFS.
///
/// The `path` is resolved against `dirfd` via [`vfs_resolve_path`] and the
/// resulting directory is created with [`vfs_mkdir`].
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidArgument`] if `dirfd` cannot be resolved (for
/// example, when it is not a VFS directory file descriptor). The `dirfd` is
/// never silently ignored.
pub fn vfs_mkdirat(dirfd: c_int, path: &str) -> Result<(), Fat32Error> {
    let resolved: String = vfs_resolve_path(dirfd, path).ok_or(Fat32Error::InvalidArgument)?;
    vfs_mkdir(&resolved)
}

/// Removes an empty directory through the VFS.
pub fn vfs_rmdir(path: &str) -> Result<(), Fat32Error> {
    crate::rmdir(path)
}

/// Changes the VFS current working directory.
pub fn vfs_chdir(path: &str) -> Result<(), Fat32Error> {
    crate::chdir(path)
}

/// Changes the current working directory to the directory referenced by a VFS FD.
///
/// Only works on directory handles. Returns an error for file handles.
pub fn vfs_fchdir(fd: c_int) -> Result<(), Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    // Extract the directory path and release the file lock before calling `chdir`, which acquires
    // its own locks during path resolution.
    let path: String = {
        let guard = file.lock();
        match &guard.handle {
            VfsFileHandle::Directory(dir) => dir.path.clone(),
            _ => return Err(Fat32Error::NotADirectory),
        }
    };
    crate::chdir(&path)
}

/// Gets the VFS current working directory.
pub fn vfs_getcwd() -> Result<alloc::string::String, Fat32Error> {
    crate::cwd()
}

/// Lists directory contents through the VFS.
///
/// Returns a vector of directory entries.
pub fn vfs_readdir(path: &str) -> Result<alloc::vec::Vec<crate::DirEntry>, Fat32Error> {
    crate::read_dir(path)
}

/// Truncates a VFS file descriptor to the given length.
///
/// POSIX requires that `ftruncate()` does not change the file offset.
/// The current offset is saved before truncation and restored afterwards.
pub fn vfs_ftruncate(fd: c_int, length: off_t) -> Result<(), Fat32Error> {
    let open_file: OpenFile = entry_arc(fd)?;
    let mut guard = open_file.lock();
    let entry: &mut VfsEntry = &mut guard;
    match &mut entry.handle {
        VfsFileHandle::Fat32(file) => {
            let current_size: u64 = file.size()?;
            let target: u64 = length as u64;

            // Save the current offset so we can restore it after truncation.
            let saved: u64 = file.seek(file_seek::SEEK_CUR, 0)?;

            if target <= current_size {
                // Shrink: seek to target and truncate.
                let result: Result<(), Fat32Error> = (|| {
                    file.seek(file_seek::SEEK_SET, length)?;
                    file.truncate()?;
                    Ok(())
                })();
                let _ = file.seek(file_seek::SEEK_SET, saved as off_t);
                result
            } else {
                // Extend: write zeros from current EOF to target size.
                file.seek(file_seek::SEEK_END, 0)?;
                let mut remaining: usize = (target - current_size) as usize;
                let zeros: [u8; 512] = [0u8; 512];
                while remaining > 0 {
                    let chunk: usize = core::cmp::min(remaining, zeros.len());
                    let written: usize = file.write(&zeros[..chunk])?;
                    if written == 0 {
                        let _ = file.seek(file_seek::SEEK_SET, saved as off_t);
                        return Err(Fat32Error::NoSpace);
                    }
                    remaining -= written;
                }
                let _ = file.seek(file_seek::SEEK_SET, saved as off_t);
                Ok(())
            }
        },
        VfsFileHandle::DirectRead(_) => Err(Fat32Error::ReadOnly),
        VfsFileHandle::Directory(_)
        | VfsFileHandle::HostFs(_)
        | VfsFileHandle::Pipe(_)
        | VfsFileHandle::Console(_)
        | VfsFileHandle::Socket(_) => Err(Fat32Error::NotSupported),
    }
}

/// Ensures a VFS file is at least `offset + len` bytes.
///
/// If the file is smaller than the target size, it is extended by writing
/// zero bytes. The file offset is preserved.
pub fn vfs_fallocate(fd: c_int, offset: off_t, len: off_t) -> Result<(), Fat32Error> {
    let open_file: OpenFile = entry_arc(fd)?;
    let mut guard = open_file.lock();
    let entry: &mut VfsEntry = &mut guard;
    match &mut entry.handle {
        VfsFileHandle::Fat32(file) => {
            let target_size: u64 = (offset + len) as u64;
            let current_size: u64 = file.size()?;
            if current_size >= target_size {
                return Ok(());
            }

            // Save the current offset.
            let saved: u64 = file.seek(file_seek::SEEK_CUR, 0)?;

            // Seek to end and write zeros in a loop (fatfs writes per-cluster).
            file.seek(file_seek::SEEK_END, 0)?;
            let mut remaining: usize = (target_size - current_size) as usize;
            let zeros: [u8; 512] = [0u8; 512];
            while remaining > 0 {
                let chunk: usize = core::cmp::min(remaining, zeros.len());
                let written: usize = file.write(&zeros[..chunk])?;
                if written == 0 {
                    let _ = file.seek(file_seek::SEEK_SET, saved as off_t);
                    return Err(Fat32Error::NoSpace);
                }
                remaining -= written;
            }

            // Restore the original offset.
            let _ = file.seek(file_seek::SEEK_SET, saved as off_t);
            Ok(())
        },
        VfsFileHandle::DirectRead(_) => Err(Fat32Error::ReadOnly),
        VfsFileHandle::Directory(_)
        | VfsFileHandle::HostFs(_)
        | VfsFileHandle::Pipe(_)
        | VfsFileHandle::Console(_)
        | VfsFileHandle::Socket(_) => Err(Fat32Error::NotSupported),
    }
}

/// Syncs a VFS file descriptor (flush buffered data).
///
/// For in-memory FAT, this flushes the fatfs buffers.
pub fn vfs_fsync(fd: c_int) -> Result<(), Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;
    match &mut entry.handle {
        VfsFileHandle::Fat32(file) => file.flush(),
        VfsFileHandle::DirectRead(_)
        | VfsFileHandle::Directory(_)
        | VfsFileHandle::HostFs(_)
        | VfsFileHandle::Pipe(_)
        | VfsFileHandle::Console(_)
        | VfsFileHandle::Socket(_) => Ok(()),
    }
}

/// Checks if a VFS file descriptor refers to a terminal.
///
/// VFS file descriptors are never terminals.
pub fn vfs_isatty(_fd: c_int) -> bool {
    false
}

/// Reads from a VFS file descriptor at a given offset without changing position.
///
/// POSIX semantics: reading past EOF returns 0 bytes (not an error).
pub fn vfs_pread(fd: c_int, buf: &mut [u8], offset: off_t) -> Result<c_size_t, Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;

    // If the offset is at or past EOF, return 0 (POSIX EOF semantics).
    let size: u64 = entry.handle.size()?;
    if offset as u64 >= size {
        return Ok(0);
    }

    // Save current position, seek to offset, read, then restore.
    let saved: off_t = entry.handle.seek(0, file_seek::SEEK_CUR)?;
    entry.handle.seek(offset, file_seek::SEEK_SET)?;
    let result: Result<usize, Fat32Error> = entry.handle.read(buf);
    // Always restore position, even if read failed.
    let _ = entry.handle.seek(saved, file_seek::SEEK_SET);
    let n: usize = result?;
    Ok(n as c_size_t)
}

/// Writes to a VFS file descriptor at a given offset without changing position.
///
/// POSIX semantics: writing past EOF extends the file with zeros up to the offset.
pub fn vfs_pwrite(fd: c_int, buf: &[u8], offset: off_t) -> Result<c_size_t, Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;

    // Save current handle position.
    let saved: off_t = entry.handle.seek(0, file_seek::SEEK_CUR)?;

    // If offset is past EOF, extend the file with zeros to fill the gap.
    let size: u64 = entry.handle.size()?;
    if (offset as u64) > size {
        entry.handle.seek(0, file_seek::SEEK_END)?;
        let gap: usize = (offset as u64 - size) as usize;
        let zeros: [u8; 512] = [0u8; 512];
        let mut remaining: usize = gap;
        while remaining > 0 {
            let chunk: usize = core::cmp::min(remaining, zeros.len());
            let written: usize = entry.handle.write(&zeros[..chunk])?;
            if written == 0 {
                let _ = entry.handle.seek(saved, file_seek::SEEK_SET);
                return Err(Fat32Error::NoSpace);
            }
            remaining -= written;
        }
    }

    // Seek to offset and write.
    entry.handle.seek(offset, file_seek::SEEK_SET)?;
    let result: Result<usize, Fat32Error> = entry.handle.write(buf);
    // Always restore handle position, even if write failed.
    let _ = entry.handle.seek(saved, file_seek::SEEK_SET);
    let n: usize = result?;
    Ok(n as c_size_t)
}

/// Changes file mode bits through the VFS.
///
/// FAT32 does not support POSIX permission bits, so the mode is accepted
/// but silently ignored. Returns `Err` if the path does not exist.
pub fn vfs_chmod(path: &str, _mode: ::sysapi::sys_types::mode_t) -> Result<(), Fat32Error> {
    crate::stat(path).map(|_| ())
}

/// Checks file accessibility through the VFS.
///
/// Returns `Ok(())` if the path exists, `Err` otherwise.
/// FAT32 does not have UNIX permissions, so only existence is checked.
pub fn vfs_access(path: &str) -> Result<(), Fat32Error> {
    crate::stat(path).map(|_| ())
}

/// Checks the accessibility of a file relative to a directory file descriptor.
///
/// The `path` is resolved against `dirfd` via [`vfs_resolve_path`] and the
/// resulting path is checked with [`vfs_access`].
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidArgument`] if `dirfd` cannot be resolved (for
/// example, when it is not a VFS directory file descriptor). The `dirfd` is
/// never silently ignored.
pub fn vfs_accessat(dirfd: c_int, path: &str) -> Result<(), Fat32Error> {
    let resolved: String = vfs_resolve_path(dirfd, path).ok_or(Fat32Error::InvalidArgument)?;
    vfs_access(&resolved)
}

/// File control operation on a VFS file descriptor.
///
/// Supports the file-descriptor flag commands (`F_GETFD`/`F_SETFD`, which have no effect because
/// the VFS implements no close-on-exec) and the file-status-flag commands (`F_GETFL`/`F_SETFL`).
/// `F_SETFL` stores the mutable status-flag subset (currently only `O_NONBLOCK`) on the open file
/// description; `F_GETFL` returns it. Other commands return [`Fat32Error::NotSupported`].
pub fn vfs_fcntl(fd: c_int, cmd: c_int, arg: c_int) -> Result<c_int, Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;

    match cmd {
        file_control_request::F_GETFD => Ok(0), // No FD flags (no close-on-exec for VFS).
        file_control_request::F_SETFD => Ok(0), // Accept but ignore (no close-on-exec).
        file_control_request::F_GETFL => Ok(file.lock().status_flags),
        file_control_request::F_SETFL => {
            // Persist only the mutable status-flag subset (currently `O_NONBLOCK`). The remaining
            // bits (access mode, creation flags) are not changeable via `F_SETFL` per POSIX.
            file.lock().status_flags = arg & file_status_flags::O_NONBLOCK;
            Ok(0)
        },
        _ => Err(Fat32Error::NotSupported), // Other commands not supported.
    }
}

/// Reads directory entries from a VFS directory file descriptor.
///
/// Returns entries as `posix_dent` structs suitable for the `getdents` syscall.
pub fn vfs_getdents(
    fd: c_int,
    count: usize,
) -> Result<Vec<::sysapi::dirent::posix_dent>, Fat32Error> {
    use ::sysapi::{
        dirent::{
            dirent_file_type,
            posix_dent,
        },
        limits::NAME_MAX,
    };

    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;

    let dir_handle: &mut DirectoryHandle = match &mut entry.handle {
        VfsFileHandle::Directory(dh) => dh,
        _ => return Err(Fat32Error::InvalidArgument),
    };

    let entries: Vec<crate::DirEntry> = dir_handle.read_entries(count)?;

    // FAT32 has no real inodes; use synthetic 1-based indices.
    let mut result: Vec<posix_dent> = Vec::new();
    for (i, de) in entries.iter().enumerate() {
        let mut dent: posix_dent = posix_dent {
            d_ino: (i + 1) as u64,
            d_reclen: core::mem::size_of::<posix_dent>() as u16,
            d_type: if de.is_dir() {
                dirent_file_type::DT_DIR
            } else {
                dirent_file_type::DT_REG
            },
            ..posix_dent::default()
        };
        let name_bytes: &[u8] = de.name().as_bytes();
        let copy_len: usize = name_bytes.len().min(NAME_MAX);
        dent.d_name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
        dent.d_name[copy_len] = 0;
        result.push(dent);
    }

    Ok(result)
}

/// Renames a file or directory relative to directory file descriptors through the VFS.
///
/// Both paths must resolve to the same VFS mount. The `olddirfd` and `newdirfd` parameters must
/// be `AT_FDCWD`; the VFS resolves all paths from the CWD and does not support dirfd-relative
/// resolution.
///
/// # Parameters
///
/// - `olddirfd`: Directory file descriptor for the old path (must be `AT_FDCWD`).
/// - `oldpath`: Current path of the file or directory.
/// - `newdirfd`: Directory file descriptor for the new path (must be `AT_FDCWD`).
/// - `newpath`: New path for the file or directory.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidArgument`] if a dirfd cannot be resolved.
/// Returns a [`Fat32Error`] if the paths are on different mounts, the old path does not exist,
/// or the new path already exists.
pub fn vfs_renameat(
    olddirfd: c_int,
    oldpath: &str,
    newdirfd: c_int,
    newpath: &str,
) -> Result<(), Fat32Error> {
    let old_resolved: String =
        vfs_resolve_path(olddirfd, oldpath).ok_or(Fat32Error::InvalidArgument)?;
    let new_resolved: String =
        vfs_resolve_path(newdirfd, newpath).ok_or(Fat32Error::InvalidArgument)?;
    crate::rename(&old_resolved, &new_resolved)
}

/// Unlinks a file or removes a directory relative to a directory file descriptor through the VFS.
///
/// When `AT_REMOVEDIR` is set in `flags`, the operation behaves like `rmdir()` and removes an
/// empty directory. Otherwise, it behaves like `unlink()` and removes a regular file.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor (must be `AT_FDCWD`).
/// - `path`: Path of the file or directory to remove.
/// - `flags`: If `AT_REMOVEDIR` (0x8) is set, remove a directory; otherwise remove a file.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidArgument`] if `dirfd` cannot be resolved.
/// Returns a [`Fat32Error`] if the path does not exist, the directory is not empty (when removing
/// a directory), or the path refers to a directory but `AT_REMOVEDIR` is not set.
pub fn vfs_unlinkat(dirfd: c_int, path: &str, flags: c_int) -> Result<(), Fat32Error> {
    use ::sysapi::fcntl::atflags::AT_REMOVEDIR;
    let resolved: String = vfs_resolve_path(dirfd, path).ok_or(Fat32Error::InvalidArgument)?;
    if flags & AT_REMOVEDIR != 0 {
        crate::rmdir(&resolved)
    } else {
        crate::unlink(&resolved)
    }
}

/// Attempts to create a hard link through the VFS.
///
/// FAT32 does not support hard links. This function always returns
/// [`Fat32Error::NotSupported`].
///
/// # Parameters
///
/// - `_olddirfd`: Directory file descriptor for the old path (ignored).
/// - `_oldpath`: Path to the existing file.
/// - `_newdirfd`: Directory file descriptor for the new path (ignored).
/// - `_newpath`: Path for the new link.
/// - `_flags`: Link flags (ignored).
///
/// # Errors
///
/// Always returns [`Fat32Error::NotSupported`].
pub fn vfs_linkat(
    _olddirfd: c_int,
    _oldpath: &str,
    _newdirfd: c_int,
    _newpath: &str,
    _flags: c_int,
) -> Result<(), Fat32Error> {
    Err(Fat32Error::NotSupported)
}

/// Attempts to create a symbolic link through the VFS.
///
/// FAT32 does not support symbolic links. This function always returns
/// [`Fat32Error::NotSupported`].
///
/// # Parameters
///
/// - `_target`: Path that the symbolic link should point to.
/// - `_dirfd`: Directory file descriptor for the link path (ignored).
/// - `_linkpath`: Path for the new symbolic link.
///
/// # Errors
///
/// Always returns [`Fat32Error::NotSupported`].
pub fn vfs_symlinkat(_target: &str, _dirfd: c_int, _linkpath: &str) -> Result<(), Fat32Error> {
    Err(Fat32Error::NotSupported)
}

/// Changes the mode of a file relative to a directory file descriptor through the VFS.
///
/// FAT32 does not support POSIX permission bits. This function validates
/// its arguments and returns success without modifying any permissions.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor for relative path resolution.
/// - `path`: Path to the target file.
/// - `_mode`: File mode bits (ignored on FAT32).
/// - `_flag`: Flags (ignored on FAT32).
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidArgument`] if the path cannot be resolved.
/// Returns [`Fat32Error::FileNotFound`] if the resolved path does not exist.
pub fn vfs_fchmodat(
    dirfd: c_int,
    path: &str,
    _mode: ::sysapi::sys_types::mode_t,
    _flag: c_int,
) -> Result<(), Fat32Error> {
    let resolved: String = vfs_resolve_path(dirfd, path).ok_or(Fat32Error::InvalidArgument)?;
    // Verify that the target exists using the VFS-level stat for consistent semantics.
    crate::stat(&resolved).map(|_| ())
}

/// Changes the owner and group of a file relative to a directory file descriptor through the VFS.
///
/// FAT32 does not support POSIX ownership. This function validates
/// its arguments and returns success without modifying any ownership.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor for relative path resolution.
/// - `path`: Path to the target file.
/// - `_owner`: Owner of the file (ignored on FAT32).
/// - `_group`: Group of the file (ignored on FAT32).
/// - `_flag`: Flags (ignored on FAT32).
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidArgument`] if the path cannot be resolved.
/// Returns [`Fat32Error::FileNotFound`] if the resolved path does not exist.
pub fn vfs_fchownat(
    dirfd: c_int,
    path: &str,
    _owner: uid_t,
    _group: gid_t,
    _flag: c_int,
) -> Result<(), Fat32Error> {
    let resolved: String = vfs_resolve_path(dirfd, path).ok_or(Fat32Error::InvalidArgument)?;
    // Verify that the target exists using the VFS-level stat for consistent semantics.
    crate::stat(&resolved).map(|_| ())
}

/// Sets file access and modification times through the VFS.
///
/// FAT32 does not support fine-grained POSIX timestamps. This function
/// validates its arguments and returns success without modifying any
/// timestamps.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor for relative path resolution.
/// - `pathname`: Path to the target file.
/// - `_times`: Access and modification times (ignored on FAT32).
/// - `flags`: Flags (must be zero; unsupported flags are rejected).
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidArgument`] if the path cannot be resolved.
/// Returns [`Fat32Error::FileNotFound`] if the resolved path does not exist.
pub fn vfs_utimensat(
    dirfd: c_int,
    pathname: &str,
    _times: &[timespec; 2],
    flags: c_int,
) -> Result<(), Fat32Error> {
    // Reject unsupported flags since FAT32 does not handle them.
    if flags != 0 {
        return Err(Fat32Error::InvalidArgument);
    }
    let path: String = vfs_resolve_path(dirfd, pathname).ok_or(Fat32Error::InvalidArgument)?;
    // Verify that the target exists using the VFS-level stat for consistent semantics.
    crate::stat(&path).map(|_| ())
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    // -- VfsStat tests -----------------------------------------------------------

    /// Tests VfsStat construction and accessors for a file.
    #[test]
    fn vfs_stat_file() {
        let s: VfsStat = VfsStat::new(1024, false);
        assert_eq!(s.size(), 1024, "file size should be 1024");
        assert!(!s.is_dir(), "should not be a directory");
    }

    /// Tests VfsStat construction and accessors for a directory.
    #[test]
    fn vfs_stat_directory() {
        let s: VfsStat = VfsStat::new(0, true);
        assert_eq!(s.size(), 0, "directory size should be 0");
        assert!(s.is_dir(), "should be a directory");
    }

    // -- DirectReadHandle tests --------------------------------------------------

    /// Tests reading from a direct read handle.
    #[test]
    fn direct_read_basic() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
        let mut buf: [u8; 3] = [0; 3];

        let n: usize = handle.read(&mut buf);
        assert_eq!(n, 3, "should read 3 bytes");
        assert_eq!(buf, [1, 2, 3], "first 3 bytes");
    }

    /// Tests reading until EOF.
    #[test]
    fn direct_read_to_eof() {
        let data: [u8; 3] = [10, 20, 30];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
        let mut buf: [u8; 10] = [0; 10];

        let n: usize = handle.read(&mut buf);
        assert_eq!(n, 3, "should read all 3 bytes");
        assert_eq!(&buf[..3], &[10, 20, 30]);

        let n2: usize = handle.read(&mut buf);
        assert_eq!(n2, 0, "should return 0 at EOF");
    }

    /// Tests reading with empty buffer.
    #[test]
    fn direct_read_empty_buffer() {
        let data: [u8; 3] = [1, 2, 3];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
        let mut buf: [u8; 0] = [];

        let n: usize = handle.read(&mut buf);
        assert_eq!(n, 0, "empty buffer should read 0 bytes");
    }

    /// Tests SEEK_SET on a direct read handle.
    #[test]
    fn direct_seek_set() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        // Read 2 bytes to advance position.
        let mut buf: [u8; 2] = [0; 2];
        handle.read(&mut buf);

        // Seek back to start.
        let pos: off_t = handle
            .seek(0, file_seek::SEEK_SET)
            .expect("SEEK_SET should succeed");
        assert_eq!(pos, 0, "position should be 0");

        // Read again.
        let n: usize = handle.read(&mut buf);
        assert_eq!(n, 2, "should read 2 bytes after seek");
        assert_eq!(buf, [1, 2], "should re-read first 2 bytes");
    }

    /// Tests SEEK_CUR on a direct read handle.
    #[test]
    fn direct_seek_cur() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        // Read 2 bytes to advance position.
        let mut buf: [u8; 2] = [0; 2];
        handle.read(&mut buf);

        // Seek forward by 1 (relative).
        let pos: off_t = handle
            .seek(1, file_seek::SEEK_CUR)
            .expect("SEEK_CUR should succeed");
        assert_eq!(pos, 3, "position should be 3");

        // Read next byte.
        let mut one: [u8; 1] = [0];
        let n: usize = handle.read(&mut one);
        assert_eq!(n, 1);
        assert_eq!(one[0], 4, "should be the 4th byte");
    }

    /// Tests SEEK_END on a direct read handle.
    #[test]
    fn direct_seek_end() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        let pos: off_t = handle
            .seek(0, file_seek::SEEK_END)
            .expect("SEEK_END should succeed");
        assert_eq!(pos, 5, "position should be at end");

        let pos2: off_t = handle
            .seek(-2, file_seek::SEEK_END)
            .expect("SEEK_END(-2) should succeed");
        assert_eq!(pos2, 3, "position should be 3");
    }

    /// Tests that seeking to a negative position fails.
    #[test]
    fn direct_seek_negative_fails() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        let result = handle.seek(-1, file_seek::SEEK_SET);
        assert!(result.is_err(), "negative SEEK_SET should fail");
    }

    /// Tests that seeking past end fails.
    #[test]
    fn direct_seek_past_end_fails() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        let result = handle.seek(6, file_seek::SEEK_SET);
        assert!(result.is_err(), "seeking past end should fail");
    }

    /// Tests that an invalid whence value fails.
    #[test]
    fn direct_seek_invalid_whence() {
        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let mut handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());

        let result = handle.seek(0, 99);
        assert!(result.is_err(), "invalid whence should fail");
    }

    /// Tests the size accessor.
    #[test]
    fn direct_read_size() {
        let data: [u8; 42] = [0; 42];
        let handle: DirectReadHandle = DirectReadHandle::new(data.as_ptr(), data.len());
        assert_eq!(handle.size(), 42, "size should match data length");
    }

    // -- VfsFileHandle::DirectRead dispatch tests --------------------------------

    /// Tests VfsFileHandle::DirectRead read dispatch.
    #[test]
    fn vfs_handle_direct_read() {
        let data: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];
        let mut handle: VfsFileHandle =
            VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

        let mut buf: [u8; 4] = [0; 4];
        let n: usize = handle.read(&mut buf).expect("read should succeed");
        assert_eq!(n, 4);
        assert_eq!(buf, data);
    }

    /// Tests VfsFileHandle::DirectRead write fails (read-only).
    #[test]
    fn vfs_handle_direct_write_fails() {
        let data: [u8; 4] = [0; 4];
        let mut handle: VfsFileHandle =
            VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

        let result = handle.write(&[1, 2, 3]);
        assert!(result.is_err(), "writing to DirectRead should fail");
    }

    /// Tests VfsFileHandle::DirectRead seek dispatch.
    #[test]
    fn vfs_handle_direct_seek() {
        let data: [u8; 10] = [0; 10];
        let mut handle: VfsFileHandle =
            VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

        let pos: off_t = handle
            .seek(5, file_seek::SEEK_SET)
            .expect("seek should succeed");
        assert_eq!(pos, 5);
    }

    /// Tests VfsFileHandle::DirectRead size dispatch.
    #[test]
    fn vfs_handle_direct_size() {
        let data: [u8; 100] = [0; 100];
        let mut handle: VfsFileHandle =
            VfsFileHandle::DirectRead(DirectReadHandle::new(data.as_ptr(), data.len()));

        let size: u64 = handle.size().expect("size should succeed");
        assert_eq!(size, 100);
    }

    // -- FD range tests ----------------------------------------------------------

    /// Tests that VFS FD base is outside linuxd range.
    #[test]
    fn vfs_fd_base_is_high() {
        assert!(VFS_FD_BASE >= 1024, "VFS FD base should be >= 1024 to avoid linuxd conflicts");
    }

    /// Tests is_vfs_fd with FDs in range.
    #[test]
    fn is_vfs_fd_in_range() {
        assert!(is_vfs_fd(VFS_FD_BASE), "base FD should be a VFS FD");
        assert!(
            is_vfs_fd(VFS_FD_BASE + VFS_MAX_OPEN_FILES as c_int - 1),
            "last FD should be a VFS FD"
        );
    }

    /// Tests is_vfs_fd with FDs out of range.
    #[test]
    fn is_vfs_fd_out_of_range() {
        assert!(!is_vfs_fd(0), "FD 0 should not be a VFS FD");
        assert!(!is_vfs_fd(VFS_FD_BASE - 1), "FD below base should not be a VFS FD");
        assert!(
            !is_vfs_fd(VFS_FD_BASE + VFS_MAX_OPEN_FILES as c_int),
            "FD past max should not be a VFS FD"
        );
    }

    // -- fork() per-process descriptor table tests -------------------------------

    /// Serializes the fork tests below. They mutate the process-global `CURRENT_PID` and the shared
    /// `PROCESSES` registry, so they must not run concurrently with one another.
    static FORK_TEST_GUARD: Mutex<()> = Mutex::new(());

    /// Returns the recorded working directory of a process, or `None` if it has no state.
    fn registry_cwd(pid: ProcessIdentifier) -> Option<String> {
        PROCESSES.lock().get(&pid).map(|state| state.cwd.clone())
    }

    /// Returns the number of open descriptor slots held by a process.
    fn registry_open_fd_count(pid: ProcessIdentifier) -> usize {
        PROCESSES
            .lock()
            .get(&pid)
            .map(|state| state.slots.iter().filter(|slot| slot.is_some()).count())
            .unwrap_or(0)
    }

    /// Reads the virtual position of a process's descriptor through its shared open file
    /// description.
    fn fd_virtual_pos(pid: ProcessIdentifier, fd: c_int) -> Option<off_t> {
        let procs = PROCESSES.lock();
        let state = procs.get(&pid)?;
        let idx: usize = (fd - VFS_FD_BASE) as usize;
        let entry: OpenFile = state.slots.get(idx)?.as_ref()?.file.clone();
        let pos: off_t = entry.lock().virtual_pos;
        Some(pos)
    }

    /// Writes the virtual position of a process's descriptor through its shared open file
    /// description.
    fn set_fd_virtual_pos(pid: ProcessIdentifier, fd: c_int, pos: off_t) {
        let procs = PROCESSES.lock();
        if let Some(state) = procs.get(&pid) {
            let idx: usize = (fd - VFS_FD_BASE) as usize;
            if let Some(Some(slot)) = state.slots.get(idx) {
                slot.file.lock().virtual_pos = pos;
            }
        }
    }

    /// Removes all recorded state for the given processes (test cleanup).
    fn forget_processes(pids: &[ProcessIdentifier]) {
        let mut procs = PROCESSES.lock();
        for pid in pids {
            procs.remove(pid);
        }
        CURRENT_PID.store(0, Ordering::Relaxed);
    }

    /// Tests that `fork()` gives the child a copy of the parent's open descriptors.
    #[test]
    fn fork_inherits_open_descriptors() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7001), ProcessIdentifier::from(0x7002));

        // Parent opens a host-backed descriptor.
        set_current_process(parent);
        let fd: c_int = vfs_alloc_hostfs(42, false, None).expect("alloc should succeed");
        assert_eq!(vfs_hostfs_remote_fd(fd), Some(42), "parent should see the remote fd");

        // Fork: the child inherits a copy of the descriptor table.
        vfs_fork_clone(parent, child).expect("fork clone should succeed");

        // The child observes the same descriptor referring to the same remote fd.
        set_current_process(child);
        assert_eq!(
            vfs_hostfs_remote_fd(fd),
            Some(42),
            "child should inherit the parent's descriptor"
        );

        forget_processes(&[parent, child]);
    }

    /// Tests that a forked parent and child share the same open file description (and offset).
    #[test]
    fn fork_shares_open_file_description() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7011), ProcessIdentifier::from(0x7012));

        // Parent opens a descriptor; it is the sole reference.
        set_current_process(parent);
        let fd: c_int = vfs_alloc_hostfs(43, false, None).expect("alloc should succeed");
        assert!(vfs_hostfs_is_last_ref(fd), "the only descriptor should be the last reference");

        // Fork: parent and child now share the open file description.
        vfs_fork_clone(parent, child).expect("fork clone should succeed");
        set_current_process(parent);
        assert!(!vfs_hostfs_is_last_ref(fd), "parent must not be the last reference after fork");
        set_current_process(child);
        assert!(!vfs_hostfs_is_last_ref(fd), "child must not be the last reference after fork");

        // A position advanced through the parent's descriptor is visible through the child's,
        // because both refer to the same open file description (POSIX offset sharing).
        set_fd_virtual_pos(parent, fd, 128);
        assert_eq!(
            fd_virtual_pos(child, fd),
            Some(128),
            "child should observe the parent's file offset"
        );

        forget_processes(&[parent, child]);
    }

    /// Tests that closing a descriptor in the child leaves the parent's descriptor intact, and that
    /// the parent then becomes the last reference (so a host close would be forwarded).
    #[test]
    fn fork_descriptor_close_is_isolated() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7021), ProcessIdentifier::from(0x7022));

        set_current_process(parent);
        let fd: c_int = vfs_alloc_hostfs(44, false, None).expect("alloc should succeed");
        vfs_fork_clone(parent, child).expect("fork clone should succeed");

        // The child closes its descriptor.
        set_current_process(child);
        vfs_close(fd).expect("child close should succeed");
        assert_eq!(vfs_hostfs_remote_fd(fd), None, "child's descriptor should be gone");

        // The parent's descriptor is untouched and is now the sole reference.
        set_current_process(parent);
        assert_eq!(vfs_hostfs_remote_fd(fd), Some(44), "parent's descriptor should remain open");
        assert!(
            vfs_hostfs_is_last_ref(fd),
            "parent should be the last reference once the child has closed"
        );

        forget_processes(&[parent, child]);
    }

    /// Tests that `fork()` deep-copies the parent's working directory into the child.
    #[test]
    fn fork_clones_working_directory() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7031), ProcessIdentifier::from(0x7032));

        // Parent sets a working directory, then forks.
        set_current_process(parent);
        set_current_cwd(String::from("/data"));
        vfs_fork_clone(parent, child).expect("fork clone should succeed");

        // The child inherits a copy of the parent's working directory.
        assert_eq!(registry_cwd(child), Some(String::from("/data")), "child should inherit cwd");

        // The copy is independent: changing the child's cwd does not affect the parent.
        set_current_process(child);
        set_current_cwd(String::from("/other"));
        assert_eq!(
            registry_cwd(parent),
            Some(String::from("/data")),
            "parent cwd must be unchanged"
        );
        assert_eq!(
            registry_cwd(child),
            Some(String::from("/other")),
            "child cwd should be updated"
        );

        forget_processes(&[parent, child]);
    }

    /// Tests that forking from a parent with no recorded state is rejected. `procd` registers every
    /// process before it can fork, so an unregistered parent is a contract violation.
    #[test]
    fn fork_without_parent_state_is_rejected() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7041), ProcessIdentifier::from(0x7042));

        // The parent has never been seen by the VFS.
        assert!(registry_cwd(parent).is_none(), "parent should have no state");

        let err: Fat32Error =
            vfs_fork_clone(parent, child).expect_err("fork from unregistered parent should fail");
        assert_eq!(err, Fat32Error::NotFound, "missing parent state must be rejected");

        // The child must not have been registered as a side effect of the failed fork.
        assert!(registry_cwd(child).is_none(), "child must not be registered on failure");
        assert_eq!(registry_open_fd_count(child), 0, "child should have no open descriptors");

        forget_processes(&[parent, child]);
    }

    /// Tests that forking into a child that already has recorded state is rejected, so its open
    /// descriptors are never silently dropped.
    #[test]
    fn fork_into_existing_child_is_rejected() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7051), ProcessIdentifier::from(0x7052));

        // The child already holds an open descriptor (e.g., PID reuse or a logic bug).
        set_current_process(child);
        let fd: c_int = vfs_alloc_hostfs(45, false, None).expect("alloc should succeed");

        // Forking onto it must fail rather than clobber the existing state.
        let err: Fat32Error =
            vfs_fork_clone(parent, child).expect_err("fork into existing child should fail");
        assert_eq!(err, Fat32Error::AlreadyExists, "existing child state must be rejected");

        // The child's descriptor is left intact.
        set_current_process(child);
        assert_eq!(
            vfs_hostfs_remote_fd(fd),
            Some(45),
            "child's existing descriptor must be preserved"
        );

        forget_processes(&[parent, child]);
    }

    /// Tests that forking onto a child whose only state is the empty placeholder inserted by
    /// [`set_current_process`] succeeds. The child's first request can race ahead of procd's
    /// fork-clone notification and lazily create that placeholder; because it holds no descriptors,
    /// the clone must overwrite it rather than fail, otherwise the child never inherits the
    /// parent's descriptors.
    #[test]
    fn fork_overwrites_empty_placeholder_child() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7061), ProcessIdentifier::from(0x7062));

        // Parent opens a host-backed descriptor.
        set_current_process(parent);
        let fd: c_int = vfs_alloc_hostfs(46, false, None).expect("alloc should succeed");

        // The child's first request reaches the VFS before the fork-clone notification, lazily
        // inserting an empty placeholder state for it.
        set_current_process(child);
        assert_eq!(registry_open_fd_count(child), 0, "placeholder must hold no descriptors");

        // Fork-clone must overwrite the empty placeholder and install the inherited descriptors.
        vfs_fork_clone(parent, child).expect("fork clone over empty placeholder should succeed");

        set_current_process(child);
        assert_eq!(
            vfs_hostfs_remote_fd(fd),
            Some(46),
            "child should inherit the parent's descriptor after the placeholder is overwritten"
        );

        forget_processes(&[parent, child]);
    }

    /// Tests that forking onto an empty placeholder preserves a working directory the child set
    /// before the fork-clone notification arrived. The child's first request can race ahead of
    /// procd's notification and `chdir` within the lazily created placeholder; that explicit update
    /// must survive the clone rather than reverting to the parent's directory.
    #[test]
    fn fork_preserves_placeholder_cwd() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7071), ProcessIdentifier::from(0x7072));

        // Parent establishes its own working directory.
        set_current_process(parent);
        set_current_cwd(String::from("/parent"));

        // The child's first request races ahead of the fork-clone notification, lazily creating a
        // placeholder, then the child changes its working directory before the notification lands.
        set_current_process(child);
        set_current_cwd(String::from("/child"));
        assert_eq!(registry_open_fd_count(child), 0, "placeholder must hold no descriptors");

        // Fork-clone overwrites the placeholder but must keep the child's own working directory.
        vfs_fork_clone(parent, child).expect("fork clone over empty placeholder should succeed");
        assert_eq!(
            registry_cwd(child),
            Some(String::from("/child")),
            "child's pre-existing working directory must be preserved"
        );

        forget_processes(&[parent, child]);
    }

    /// Tests that forking onto a pristine placeholder — one the child created but never `chdir`ed —
    /// lets the child inherit the parent's working directory. Only a directory the child actually
    /// changed is preserved; an untouched placeholder must not pin the child to the default root.
    #[test]
    fn fork_pristine_placeholder_inherits_parent_cwd() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7081), ProcessIdentifier::from(0x7082));

        // Parent establishes a non-default working directory.
        set_current_process(parent);
        set_current_cwd(String::from("/parent"));

        // The child's first request creates a placeholder but never changes its directory, leaving
        // it at the default root.
        set_current_process(child);
        assert_eq!(registry_cwd(child), Some(String::from("/")), "placeholder defaults to root");

        // Fork-clone must give the child a copy of the parent's directory, not the default root.
        vfs_fork_clone(parent, child).expect("fork clone over empty placeholder should succeed");
        assert_eq!(
            registry_cwd(child),
            Some(String::from("/parent")),
            "pristine placeholder must inherit the parent's working directory"
        );

        forget_processes(&[parent, child]);
    }

    // -- pipe descriptor tests ---------------------------------------------------

    /// Tests that `vfs_pipe` allocates two descriptors with correct directions and shared identity.
    #[test]
    fn pipe_allocates_read_and_write_ends() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7101);
        set_current_process(pid);

        let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
        let (rid, r_is_write): (u64, bool) =
            vfs_pipe_id(read_fd).expect("read fd should be a pipe");
        let (wid, w_is_write): (u64, bool) =
            vfs_pipe_id(write_fd).expect("write fd should be a pipe");
        assert_eq!(rid, wid, "both ends share one pipe identity");
        assert!(!r_is_write, "read_fd must be the read end");
        assert!(w_is_write, "write_fd must be the write end");

        forget_processes(&[pid]);
    }

    /// Tests the non-blocking read/write outcome matrix and direction enforcement on a pipe.
    #[test]
    fn pipe_read_write_outcomes() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7111);
        set_current_process(pid);

        let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");

        // Empty pipe with an open writer: a read would block.
        let mut buf: [u8; 8] = [0u8; 8];
        assert!(
            matches!(vfs_pipe_read(read_fd, &mut buf), Ok(PipeReadOutcome::WouldBlock)),
            "an empty pipe with a writer should block reads"
        );

        // Write then read back.
        assert!(
            matches!(vfs_pipe_write(write_fd, &[1, 2, 3]), Ok(PipeWriteOutcome::Wrote(3))),
            "a write into a pipe with space should succeed"
        );
        match vfs_pipe_read(read_fd, &mut buf) {
            Ok(PipeReadOutcome::Read(n)) => {
                assert_eq!(n, 3, "should read the 3 written bytes");
                assert_eq!(&buf[..3], &[1, 2, 3], "read bytes should match");
            },
            _ => panic!("expected a successful read"),
        }

        // Direction enforcement.
        assert!(vfs_pipe_read(write_fd, &mut buf).is_err(), "reading the write end is rejected");
        assert!(vfs_pipe_write(read_fd, &[0]).is_err(), "writing the read end is rejected");

        forget_processes(&[pid]);
    }

    /// Tests EOF after the write end closes and a broken pipe after the read end closes.
    #[test]
    fn pipe_eof_and_broken_pipe() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7121);
        set_current_process(pid);

        // EOF: close the write end, the read end then reports EOF.
        let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
        assert!(vfs_pipe_is_last_ref(write_fd), "the sole write descriptor is the last reference");
        vfs_close(write_fd).expect("closing the write end should succeed");
        let mut buf: [u8; 4] = [0u8; 4];
        assert!(
            matches!(vfs_pipe_read(read_fd, &mut buf), Ok(PipeReadOutcome::Eof)),
            "after the writer closes, the empty pipe reports EOF"
        );
        vfs_close(read_fd).expect("closing the read end should succeed");

        // Broken pipe: close the read end, a write then reports a broken pipe.
        let (read_fd2, write_fd2): (c_int, c_int) =
            vfs_pipe().expect("pipe creation should succeed");
        vfs_close(read_fd2).expect("closing the read end should succeed");
        assert!(
            matches!(vfs_pipe_write(write_fd2, &[9]), Ok(PipeWriteOutcome::BrokenPipe)),
            "writing with no readers reports a broken pipe"
        );
        vfs_close(write_fd2).expect("closing the write end should succeed");

        forget_processes(&[pid]);
    }

    /// Tests that `F_SETFL` stores `O_NONBLOCK` and `F_GETFL`/`vfs_get_status_flags` read it back.
    #[test]
    fn pipe_nonblock_status_flags_round_trip() {
        use ::sysapi::fcntl::{
            file_control_request,
            file_status_flags,
        };
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7131);
        set_current_process(pid);

        let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
        assert_eq!(vfs_get_status_flags(read_fd), 0, "flags default to zero");

        vfs_fcntl(read_fd, file_control_request::F_SETFL, file_status_flags::O_NONBLOCK)
            .expect("F_SETFL should succeed");
        assert_eq!(
            vfs_get_status_flags(read_fd),
            file_status_flags::O_NONBLOCK,
            "O_NONBLOCK should be stored"
        );
        assert_eq!(
            vfs_fcntl(read_fd, file_control_request::F_GETFL, 0).expect("F_GETFL should succeed"),
            file_status_flags::O_NONBLOCK,
            "F_GETFL returns the stored flags"
        );
        // Status flags are per open file description: the write end is unaffected.
        assert_eq!(vfs_get_status_flags(write_fd), 0, "the write end is unaffected");

        forget_processes(&[pid]);
    }

    /// Tests that process exit surfaces pipe count-to-zero transitions for the process's own ends.
    #[test]
    fn pipe_process_exit_reports_closures() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7141);
        set_current_process(pid);

        let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
        let (pipe_id, _): (u64, bool) = vfs_pipe_id(read_fd).expect("read fd should be a pipe");
        // Both ends stay open in this single process, so it holds the only reference to each.
        let _ = write_fd;

        let reclaim: ProcessExitReclaim = vfs_process_exit(pid);
        assert!(reclaim.orphaned_hostfs_fds.is_empty(), "no hostfs fds were opened");
        assert_eq!(reclaim.pipe_closures.len(), 2, "both ends are last references at exit");
        assert!(
            reclaim.pipe_closures.iter().all(|c| c.pipe_id == pipe_id),
            "closures must target our pipe"
        );
        assert!(
            reclaim.pipe_closures.iter().any(|c| c.was_write),
            "the write end closure is reported"
        );
        assert!(
            reclaim.pipe_closures.iter().any(|c| !c.was_write),
            "the read end closure is reported"
        );

        forget_processes(&[pid]);
    }

    // -- console/socket token and per-descriptor flag tests ----------------------

    /// Tests that [`FdFlags`] defaults to empty and that each flag can be set and cleared
    /// independently of the other.
    #[test]
    fn fd_flags_set_and_clear() {
        // A default descriptor carries no flags.
        let mut flags: FdFlags = FdFlags::default();
        assert!(!flags.close_on_exec(), "close-on-exec defaults to off");
        assert!(!flags.close_on_fork(), "close-on-fork defaults to off");

        // Setting one flag must not disturb the other.
        flags.set_close_on_exec(true);
        assert!(flags.close_on_exec(), "close-on-exec should be set");
        assert!(!flags.close_on_fork(), "close-on-fork must remain off");

        flags.set_close_on_fork(true);
        assert!(flags.close_on_exec(), "close-on-exec must remain set");
        assert!(flags.close_on_fork(), "close-on-fork should be set");

        // Clearing one flag must not disturb the other.
        flags.set_close_on_exec(false);
        assert!(!flags.close_on_exec(), "close-on-exec should be cleared");
        assert!(flags.close_on_fork(), "close-on-fork must remain set");
    }

    /// Tests that a [`ConsoleHandle`] reports the standard stream it was created for.
    #[test]
    fn console_handle_tracks_stream() {
        for stream in [
            ConsoleStream::Stdin,
            ConsoleStream::Stdout,
            ConsoleStream::Stderr,
        ] {
            let handle: ConsoleHandle = ConsoleHandle::new(stream);
            assert_eq!(handle.stream(), stream, "console handle should report its stream");
        }
    }

    /// Tests that a [`SocketHandle`] reports the `networkd` descriptor it was created for.
    #[test]
    fn socket_handle_tracks_remote_fd() {
        let handle: SocketHandle = SocketHandle::new(7);
        assert_eq!(handle.remote_fd(), 7, "socket handle should report its networkd descriptor");
    }

    /// Tests that `fork()` copies a slot's per-descriptor flags into the child and that the copies
    /// are independent, so changing the child's flags does not affect the parent's. This is the
    /// per-descriptor semantics the later close-on-exec/close-on-fork plans depend on.
    #[test]
    fn fork_copies_independent_descriptor_flags() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7201), ProcessIdentifier::from(0x7202));

        // Parent allocates a console-backed descriptor and marks it close-on-exec and close-on-fork.
        set_current_process(parent);
        let fd: c_int = alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("console alloc should succeed");
        assert_eq!(vfs_get_fd_flags(fd), Some(FdFlags::default()), "flags default to empty");
        let mut flags: FdFlags = FdFlags::default();
        flags.set_close_on_exec(true);
        flags.set_close_on_fork(true);
        vfs_set_fd_flags(fd, flags).expect("setting flags should succeed");

        // Fork: the child inherits a copy of the descriptor's flags.
        vfs_fork_clone(parent, child).expect("fork clone should succeed");
        set_current_process(child);
        let child_flags: FdFlags = vfs_get_fd_flags(fd).expect("child should inherit the slot");
        assert!(child_flags.close_on_exec(), "child inherits close-on-exec");
        assert!(child_flags.close_on_fork(), "child inherits close-on-fork");

        // The flags are per descriptor: clearing the child's must leave the parent's untouched.
        vfs_set_fd_flags(fd, FdFlags::default()).expect("clearing child flags should succeed");
        assert_eq!(vfs_get_fd_flags(fd), Some(FdFlags::default()), "child flags should be cleared");
        set_current_process(parent);
        let parent_flags: FdFlags = vfs_get_fd_flags(fd).expect("parent slot should remain");
        assert!(parent_flags.close_on_exec(), "parent close-on-exec must be unchanged");
        assert!(parent_flags.close_on_fork(), "parent close-on-fork must be unchanged");

        forget_processes(&[parent, child]);
    }

    /// Documents the deferred `F_SETFD`/`F_GETFD` round trip tracked by
    /// <https://github.com/nanvix/nanvix/issues/2604>.
    #[test]
    #[ignore = "TODO(#2604): wire vfs_fcntl(F_GETFD/F_SETFD) to FdFlags"]
    fn fcntl_fd_flags_round_trip() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7221);
        set_current_process(pid);

        let fd: c_int = alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("console alloc should succeed");
        let raw_flags: c_int = FD_CLOEXEC | FD_CLOFORK;

        vfs_fcntl(fd, file_control_request::F_SETFD, raw_flags)
            .expect("F_SETFD should store descriptor flags");
        assert_eq!(
            vfs_fcntl(fd, file_control_request::F_GETFD, 0).expect("F_GETFD should succeed"),
            raw_flags,
            "F_GETFD should return the flags stored by F_SETFD"
        );

        let stored_flags: FdFlags = vfs_get_fd_flags(fd).expect("fd flags should be recorded");
        assert!(stored_flags.close_on_exec(), "FD_CLOEXEC should be recorded");
        assert!(stored_flags.close_on_fork(), "FD_CLOFORK should be recorded");

        forget_processes(&[pid]);
    }

    /// Documents that `F_SETFD` must update only the addressed descriptor, as tracked by
    /// <https://github.com/nanvix/nanvix/issues/2604>.
    #[test]
    #[ignore = "TODO(#2604): wire vfs_fcntl(F_GETFD/F_SETFD) to FdFlags"]
    fn fcntl_fd_flags_are_per_descriptor_after_fork() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7231), ProcessIdentifier::from(0x7232));

        set_current_process(parent);
        let fd: c_int = alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("console alloc should succeed");
        vfs_fork_clone(parent, child).expect("fork clone should succeed");

        set_current_process(child);
        vfs_fcntl(fd, file_control_request::F_SETFD, FD_CLOEXEC)
            .expect("child F_SETFD should succeed");
        assert_eq!(
            vfs_fcntl(fd, file_control_request::F_GETFD, 0).expect("child F_GETFD should succeed"),
            FD_CLOEXEC,
            "child should see its own descriptor flag"
        );

        set_current_process(parent);
        assert_eq!(
            vfs_fcntl(fd, file_control_request::F_GETFD, 0).expect("parent F_GETFD should succeed"),
            0,
            "parent descriptor flags should remain unchanged"
        );

        forget_processes(&[parent, child]);
    }

    /// Tests that a lone console or socket token contributes nothing to [`ProcessExitReclaim`] at
    /// process exit: a console holds no external resource, and a socket's `networkd` descriptor is
    /// closed by a later plan rather than reclaimed here. Crucially, neither is mistaken for a
    /// hostfs or pipe handle.
    #[test]
    fn process_exit_reclaims_nothing_for_console_and_socket() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7211);
        set_current_process(pid);

        // A console token and a socket token, each the sole reference held by this process.
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stderr)))
            .expect("console alloc should succeed");
        alloc_fd(VfsFileHandle::Socket(SocketHandle::new(99)))
            .expect("socket alloc should succeed");

        // Exit must reclaim neither a hostfs descriptor nor a pipe end for these inert tokens.
        let reclaim: ProcessExitReclaim = vfs_process_exit(pid);
        assert!(
            reclaim.orphaned_hostfs_fds.is_empty(),
            "console and socket tokens own no hostfs descriptor"
        );
        assert!(
            reclaim.pipe_closures.is_empty(),
            "console and socket tokens trigger no pipe closures"
        );

        forget_processes(&[pid]);
    }
}
