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
    collections::{
        BTreeMap,
        BTreeSet,
    },
    string::String,
    sync::Arc,
    vec::Vec,
};
use ::core::sync::atomic::{
    AtomicBool,
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
    unistd::{
        file_seek,
        STDERR_FILENO,
        STDIN_FILENO,
        STDOUT_FILENO,
    },
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
    /// remote descriptor so vfsd can own the slot and its per-descriptor flags. vfsd closes the
    /// remote descriptor on `networkd` when the last reference to the slot is dropped.
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
/// vfsd own the descriptor slot and its per-descriptor flags. vfsd closes the remote descriptor on
/// `networkd` when the last reference to the slot is dropped.
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
    /// Builds the descriptor flags from a raw `fcntl(F_SETFD)` argument, rejecting any bit outside
    /// the recognized set (`FD_CLOEXEC`, `FD_CLOFORK`).
    ///
    /// POSIX requires `fcntl(F_SETFD)` to fail with `EINVAL` when an unsupported descriptor-flag bit
    /// is set rather than silently masking it. Rejecting here keeps this layer in agreement with the
    /// syscall-side validation in `syscall::safe::FileDescriptorFlags` and prevents a later
    /// `F_GETFD` from reporting flags the caller never set.
    ///
    /// # Errors
    ///
    /// Returns [`Fat32Error::InvalidArgument`] if `raw` sets any bit other than `FD_CLOEXEC` or
    /// `FD_CLOFORK`.
    pub fn try_from_bits(raw: c_int) -> Result<Self, Fat32Error> {
        if raw & !(FD_CLOEXEC | FD_CLOFORK) != 0 {
            return Err(Fat32Error::InvalidArgument);
        }
        Ok(Self(raw))
    }

    /// Returns the raw flag bits, as reported by `fcntl(F_GETFD)`.
    pub const fn bits(self) -> c_int {
        self.0
    }

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
///
/// The slot table is a flat map keyed by the raw descriptor number: it holds the low console
/// descriptors (`0`/`1`/`2`, seeded by [`vfs_seed_root_console`]) and the descriptors handed out by
/// [`alloc_fd`] in one structure. [`alloc_fd`] allocates the lowest free number across that whole
/// namespace, so the first `open` in a fresh process returns `3` and a freed number is reused
/// before any higher one. A descriptor number therefore no longer encodes its backend; vfsd answers
/// what each number is through [`vfs_resolve`].
struct ProcessState {
    /// File descriptor slots keyed by the raw file descriptor number.
    slots: BTreeMap<c_int, Slot>,
    /// Current working directory (always absolute, never ends with "/").
    cwd: String,
    /// Whether this state has been explicitly initialized rather than lazily conjured.
    ///
    /// A freshly created state is an uninitialized *placeholder*: [`set_current_process`] inserts
    /// one the first time it sees a pid, so that a forked child whose first request races ahead of
    /// its fork-clone notification has somewhere to record an early `chdir`. A state becomes
    /// initialized — *active* — when [`vfs_seed_root_console`] seeds the root, when [`fork`] clones
    /// a child, or when [`alloc_fd`] allocates a descriptor. [`vfs_fork_clone`] overwrites a child
    /// entry only while it is still a placeholder, replacing the descriptor-count test that the
    /// removed `is_empty` predicate used: a seeded state may legitimately hold console descriptors
    /// and an active state may have closed all of them, so "holds no descriptors" can no longer
    /// stand in for "safe to overwrite."
    ///
    /// [`fork`]: ProcessState::fork
    initialized: bool,
    /// Monotonic generation counter, bumped on every descriptor-table mutation (`open`, `close`,
    /// and per-descriptor flag changes).
    ///
    /// libposix's resolution cache records the generation each cached entry was learned at and uses
    /// it as a coherence epoch: once descriptor numbers stop encoding their backend (a later plan),
    /// an entry older than the table's current generation is treated as stale and re-resolved. The
    /// counter is plumbed and returned with descriptor responses now so the coherence substrate is
    /// in place; in this plan routing still follows the descriptor number, so the epoch never alters
    /// a routing decision.
    generation: u64,
}

impl ProcessState {
    /// Creates a new, uninitialized placeholder state whose working directory defaults to
    /// [`DEFAULT_CWD`] and whose descriptor table is empty.
    fn new() -> Self {
        Self {
            slots: BTreeMap::new(),
            cwd: String::from(DEFAULT_CWD),
            initialized: false,
            generation: 0,
        }
    }

    /// Creates a copy of this state for a freshly forked child.
    ///
    /// Each descriptor slot is cloned: the open file description is shared as a reference (so the
    /// parent and child share file offsets) while the per-descriptor flags are copied, giving the
    /// child its own independent `FD_CLOEXEC`/`FD_CLOFORK` settings. The working directory is
    /// deep-copied. The child is born *active*: a forked process is a real process, never an
    /// overwritable placeholder.
    fn fork(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            cwd: self.cwd.clone(),
            initialized: true,
            // The child's table starts as an exact copy of the parent's, so it inherits the
            // parent's generation: any entry libposix cached against the parent is equally coherent
            // for the child until the child mutates its own table.
            generation: self.generation,
        }
    }

    /// Advances the descriptor-table generation, marking every previously cached resolution as
    /// potentially stale. Called on each table-mutating operation.
    fn bump_generation(&mut self) {
        // Saturate rather than wrap so the generation stays monotonic: a wrap back to `0` could
        // make a stale cached epoch compare equal to a fresh one once coherence becomes
        // load-bearing. Saturating at `u64::MAX` is unreachable in practice.
        self.generation = self.generation.saturating_add(1);
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

/// Returns the identifier assigned to the root/init process.
///
/// The kernel creates the fixed daemon set first (`procd`, `memd`, then `vfsd`) and assigns the
/// boot workload the next pid. Keeping this derivation local avoids treating whichever process
/// happens to issue the first VFS request as the root, which would break the fork-race placeholder
/// path that [`vfs_fork_clone`] preserves.
///
/// TODO(#2610): replace this positional derivation with an authoritative root pid — a dedicated
/// `ProcessIdentifier::INIT` constant or a value supplied by `procd` — instead of hardcoding
/// `VFSD_RAW + 1`, which silently breaks if the daemon spawn order changes.
fn root_process_identifier() -> ProcessIdentifier {
    ProcessIdentifier::from(ProcessIdentifier::VFSD_RAW + 1)
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

/// Returns a shared reference to the open file description for `fd` in the current process.
///
/// Indexing is flat: the descriptor number is the slot key directly, so the low console slots
/// (`0`/`1`/`2`) and the descriptors handed out by [`alloc_fd`] are looked up the same way. A
/// descriptor with no slot in the current process is rejected. vfsd serves no I/O on console or
/// socket tokens; the I/O methods on [`VfsFileHandle`] reject those handles, so this accessor does
/// not need to special-case them.
fn entry_arc(fd: c_int) -> Result<OpenFile, Fat32Error> {
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    let state: &ProcessState = procs.get(&current_pid()).ok_or(Fat32Error::InvalidFd)?;
    state
        .slots
        .get(&fd)
        .map(|slot| slot.file.clone())
        .ok_or(Fat32Error::InvalidFd)
}

/// Upper bound (exclusive) of the flat per-process descriptor namespace.
///
/// The allocator hands out the lowest free descriptor in `[0, MAX_OPEN_FDS)`. Every object —
/// sockets, regular files, pipes, and console streams — draws from this single range, so there is
/// no carved-out sub-range. Exhausting it yields [`Fat32Error::TooManyOpenFiles`].
const MAX_OPEN_FDS: c_int = 2048;

/// Allocates a new file descriptor for the given handle in the current process.
///
/// Allocation is flat and lowest-free: the descriptor is the smallest non-negative number not
/// already present in the process's slot table. Console descriptors `0`/`1`/`2` occupy low keys in
/// that same table, so the first `open` in a fresh process returns `3`, and a number freed by
/// `close` is reused before any higher one — matching POSIX.
///
/// The search spans the whole flat namespace `[0, MAX_OPEN_FDS)`: sockets, files, pipes, and
/// console streams all draw from this single range, with no carved-out sub-range. A socket's
/// application-visible number is therefore the lowest free flat descriptor like any other object,
/// while the `networkd` descriptor that backs it lives in `networkd`'s own space and is invisible
/// here.
fn alloc_fd(handle: VfsFileHandle) -> Result<c_int, Fat32Error> {
    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    let state: &mut ProcessState = procs.entry(current_pid()).or_insert_with(ProcessState::new);
    // Lowest free descriptor across the whole flat namespace.
    let fd: c_int = (0..MAX_OPEN_FDS)
        .find(|candidate| !state.slots.contains_key(candidate))
        .ok_or(Fat32Error::TooManyOpenFiles)?;
    let file: OpenFile = Arc::new(Mutex::new(VfsEntry {
        handle,
        virtual_pos: 0,
        status_flags: 0,
    }));
    state.slots.insert(fd, Slot::new(file));
    // Allocating a descriptor graduates a lazily-inserted placeholder into an active state: a
    // process holding a real descriptor must never be overwritten by a later fork-clone.
    state.initialized = true;
    // Allocating a descriptor mutates the table; advance the coherence generation.
    state.bump_generation();
    Ok(fd)
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
/// of the parent's current working directory. The child's console descriptors (`0`/`1`/`2`) are
/// cloned like any other slot, so the standard streams the root process was seeded with flow down
/// every fork. A descriptor flagged `FD_CLOFORK` is the one exception: it is dropped in the child
/// rather than cloned, per POSIX close-on-fork semantics.
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
///   examined before the `parent`, so an *active* `child` yields [`Fat32Error::AlreadyExists`]
///   even when `parent` is missing as well.
/// - [`Fat32Error::AlreadyExists`] if `child` already has an *active* recorded state. A forked
///   child must be a fresh process; overwriting an active table would drop its open file
///   descriptions and leak any host-backed remote handles they hold. The caller must reclaim that
///   state first (e.g., via [`vfs_process_exit`]). Activeness — not a descriptor count — is the
///   test: a state is active once it has been seeded, cloned, or has allocated a descriptor, so a
///   process that closed all of its descriptors is still protected. An uninitialized placeholder
///   state — which [`set_current_process`] inserts lazily when the child's first request races
///   ahead of this fork-clone notification — is overwritten in place; any working directory the
///   child set via `chdir` before the notification arrived is preserved rather than reverted to
///   the parent's.
pub fn vfs_fork_clone(
    parent: ProcessIdentifier,
    child: ProcessIdentifier,
) -> Result<(), Fat32Error> {
    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    // The child's first request can reach the VFS before procd's fork-clone notification, in which
    // case `set_current_process` has already inserted an uninitialized placeholder state for it.
    // Such a placeholder is safe to overwrite. Refuse only when the existing state is active, since
    // clobbering a real table would orphan any host-backed remote handles its descriptors
    // reference. Activeness — rather than "holds no descriptors" — is the guard, so a seeded state
    // holding only console descriptors and an active state that has closed all of its descriptors
    // are each classified correctly.
    //
    // The placeholder can still carry a working directory: the racing child may have issued a
    // `chdir` before this notification arrived. Capture a directory that differs from the default
    // so the clone below keeps the child's own cwd instead of reverting it to the parent's.
    let mut child_cwd: Option<String> = None;
    if let Some(existing) = procs.get(&child) {
        if existing.initialized {
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
    // Honor close-on-fork: a descriptor flagged `FD_CLOFORK` in the parent is not inherited, so
    // drop it from the freshly cloned child table. The standard streams `0`/`1`/`2` are not flagged
    // by default, so they survive the fork.
    let pre_filter_len: usize = child_state.slots.len();
    child_state
        .slots
        .retain(|_fd, slot| !slot.fd_flags.close_on_fork());
    // `retain()` is the only table mutation here, so bump the generation at most once and only when
    // it actually dropped a descriptor.
    if child_state.slots.len() != pre_filter_len {
        child_state.bump_generation();
    }
    // Honor a working directory the child established before the fork-clone notification arrived.
    if let Some(cwd) = child_cwd {
        child_state.cwd = cwd;
    }
    procs.insert(child, child_state);
    Ok(())
}

/// Tracks whether the root process's console descriptors have been seeded.
///
/// Seeding the root is a one-time event: [`vfs_seed_root_console`] reads and sets this only after
/// verifying that the target pid is the root. Non-root calls are no-ops and do not consume the
/// latch, which is what keeps a racing child placeholder overwritable until its fork-clone arrives.
/// It is process-global, so any unit test that drives seeding must serialize on `FORK_TEST_GUARD`
/// and reset it on cleanup.
static ROOT_CONSOLE_SEEDED: AtomicBool = AtomicBool::new(false);

/// Seeds the root process's standard console descriptors (`0`/`1`/`2`) and marks its state active.
///
/// The root process is the one process not born from a fork: the kernel spawns it directly, so it
/// never receives its standard streams through [`vfs_fork_clone`] the way every other process does.
/// This installs a [`ConsoleHandle`] for stdin, stdout, and stderr into `pid`'s descriptor table so
/// that vfsd becomes the authoritative bookkeeper of those slots and forks propagate them onward.
///
/// The console slots are inert routing tokens: vfsd serves no I/O on them, but it is the authority
/// that tells libposix to route console `read`/`write` to the kernel. They occupy descriptors
/// `0`/`1`/`2` in the flat table, so the lowest-free [`alloc_fd`] hands the first `open` descriptor
/// `3`.
///
/// The call is idempotent and root-only. vfsd may invoke it both on regular syscall dispatch and
/// when a fork-clone notification names a parent; non-root pids are ignored without consuming the
/// one-shot latch, so a forked child whose first request races ahead of its clone remains an
/// overwritable placeholder. A child receives `0`/`1`/`2` only through [`vfs_fork_clone`].
pub fn vfs_seed_root_console(pid: ProcessIdentifier) {
    if pid != root_process_identifier() {
        return;
    }
    // One-shot: only the root's first call has any effect. Every later root call observes the flag
    // already set and returns immediately.
    if ROOT_CONSOLE_SEEDED.swap(true, Ordering::Relaxed) {
        return;
    }
    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    let state: &mut ProcessState = procs.entry(pid).or_insert_with(ProcessState::new);
    for (fd, stream) in [
        (STDIN_FILENO, ConsoleStream::Stdin),
        (STDOUT_FILENO, ConsoleStream::Stdout),
        (STDERR_FILENO, ConsoleStream::Stderr),
    ] {
        let file: OpenFile = Arc::new(Mutex::new(VfsEntry {
            handle: VfsFileHandle::Console(ConsoleHandle::new(stream)),
            virtual_pos: 0,
            status_flags: 0,
        }));
        state.slots.insert(fd, Slot::new(file));
    }
    // The root is now a real, active process: its state must never be mistaken for an overwritable
    // placeholder. (The root is never the target of a fork-clone, but marking it keeps the
    // placeholder/active invariant uniform across every code path.)
    state.initialized = true;
    state.bump_generation();
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
    /// `networkd` descriptors of socket slots for which the process held the final reference. Each
    /// must be closed on `networkd` or the socket endpoint leaks.
    pub orphaned_socket_fds: Vec<i32>,
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
/// because the process is gone and can no longer close them itself), the `networkd` descriptors of
/// any socket slots for which `pid` held the final reference (which it must close on `networkd` for
/// the same reason), and the pipe ends whose reference count reached zero (so the daemon can fire
/// EOF/`EPIPE` wakeups for any suspended counterparts). Descriptions still shared with a surviving
/// process are not returned.
#[must_use = "the returned hostfs and socket fds must be closed and pipe closures must trigger \
              wakeups"]
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
            orphaned_socket_fds: Vec::new(),
            pipe_closures: Vec::new(),
        };
    };
    // The process has been removed from the registry, so an `Arc` strong count of one means no
    // surviving descriptor — in this or any other process — still shares the open file description.
    // A host-backed handle must therefore be closed on hostfsd, and a pipe end's count drops to
    // zero (which the dropped end's `Drop` applies just below). As the sole owner, the lock is
    // uncontended.
    let removed_slots: Vec<Slot> = state.slots.into_values().collect();
    let mut removed_ref_counts: BTreeMap<*const Mutex<VfsEntry>, usize> = BTreeMap::new();
    for slot in &removed_slots {
        let file_id: *const Mutex<VfsEntry> = Arc::as_ptr(&slot.file);
        *removed_ref_counts.entry(file_id).or_insert(0) += 1;
    }
    let mut orphaned: Vec<i32> = Vec::new();
    let mut orphaned_sockets: Vec<i32> = Vec::new();
    let mut seen_files: BTreeSet<*const Mutex<VfsEntry>> = BTreeSet::new();
    let mut pipe_closures: Vec<PipeClosure> = Vec::new();
    for slot in removed_slots {
        let file_id: *const Mutex<VfsEntry> = Arc::as_ptr(&slot.file);
        let removed_refs: usize = *removed_ref_counts.get(&file_id).unwrap_or(&0);
        if !seen_files.insert(file_id) || Arc::strong_count(&slot.file) != removed_refs {
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
            // A socket token holds the `networkd` descriptor: as the final reference, the endpoint
            // must be closed on `networkd` so it does not leak. This arm must stay distinct so a
            // socket is never mistaken for a hostfs or pipe handle.
            VfsFileHandle::Socket(h) => orphaned_sockets.push(h.remote_fd()),
            _ => {},
        }
        // `slot` — and the open file description it holds — is dropped here, after the registry lock
        // has been released; the pipe end's `Drop` decrements its reader/writer count as part of
        // that.
    }
    ProcessExitReclaim {
        orphaned_hostfs_fds: orphaned,
        orphaned_socket_fds: orphaned_sockets,
        pipe_closures,
    }
}

/// Applies close-on-exec to a process's descriptor table when it replaces its image.
///
/// Walks `pid`'s slot table and drops every descriptor whose per-descriptor flags carry
/// `FD_CLOEXEC`, leaving the surviving descriptors at their original numbers. Each dropped slot
/// runs the same last-reference accounting as [`vfs_process_exit`]: a host-backed description for
/// which `pid` held the final reference is surfaced for closing on hostfsd, a socket slot for which
/// `pid` held the final reference is surfaced for closing on `networkd`, and a pipe end whose
/// reference count reaches zero is surfaced so the daemon can fire the EOF/`EPIPE` wakeup for any
/// suspended counterpart. Descriptions still shared with a surviving descriptor — in this process
/// (for example a non-`FD_CLOEXEC` `dup` sibling) or another (a forked relative) — are not
/// reclaimed.
///
/// The table generation is bumped once when at least one descriptor is dropped, so the new image's
/// resolution cache (rebuilt after this returns) observes the post-close-on-exec table rather than
/// the pre-exec one. Unlike [`vfs_process_exit`], the process itself is retained: only its
/// close-on-exec descriptors are removed. Applying close-on-exec to an unknown `pid`, or to one
/// with no `FD_CLOEXEC` descriptor, is a no-op that reclaims nothing.
///
/// The barrier in the process manager daemon guarantees this runs before the new image issues any
/// descriptor operation, so a descriptor flagged `FD_CLOEXEC` is provably gone before the new image
/// can observe it.
#[must_use = "the returned hostfs and socket fds must be closed and pipe closures must trigger \
              wakeups"]
pub fn vfs_exec_cloexec(pid: ProcessIdentifier) -> ProcessExitReclaim {
    // Detach the close-on-exec slots while holding the registry lock, but defer dropping the open
    // file descriptions they hold until the lock is released. Dropping a description may run
    // backend teardown; deferring keeps the registry lock from ever nesting with backend locks,
    // matching `vfs_close` and `vfs_process_exit`.
    let removed_slots: Vec<Slot> = {
        let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
            PROCESSES.lock();
        let Some(state) = procs.get_mut(&pid) else {
            return ProcessExitReclaim {
                orphaned_hostfs_fds: Vec::new(),
                orphaned_socket_fds: Vec::new(),
                pipe_closures: Vec::new(),
            };
        };
        // Identify the close-on-exec descriptors first, then remove them. Collecting the numbers up
        // front avoids mutating the map while iterating it.
        let cloexec_fds: Vec<c_int> = state
            .slots
            .iter()
            .filter(|(_, slot)| slot.fd_flags.close_on_exec())
            .map(|(fd, _)| *fd)
            .collect();
        if cloexec_fds.is_empty() {
            return ProcessExitReclaim {
                orphaned_hostfs_fds: Vec::new(),
                orphaned_socket_fds: Vec::new(),
                pipe_closures: Vec::new(),
            };
        }
        let mut removed: Vec<Slot> = Vec::with_capacity(cloexec_fds.len());
        for fd in cloexec_fds {
            if let Some(slot) = state.slots.remove(&fd) {
                removed.push(slot);
            }
        }
        // The removals are the only table mutation here, so bump the generation exactly once now
        // that at least one descriptor was dropped.
        state.bump_generation();
        removed
    };
    // The registry lock has been released and the removed slots are owned here, so an `Arc` strong
    // count of one means no surviving descriptor — in this or any other process — still shares the
    // open file description, exactly as in `vfs_process_exit`. A host-backed handle must therefore
    // be closed on hostfsd, and a pipe end's count drops to zero (which the dropped end's `Drop`
    // applies as each slot leaves scope below).
    let mut removed_ref_counts: BTreeMap<*const Mutex<VfsEntry>, usize> = BTreeMap::new();
    for slot in &removed_slots {
        let file_id: *const Mutex<VfsEntry> = Arc::as_ptr(&slot.file);
        *removed_ref_counts.entry(file_id).or_insert(0) += 1;
    }
    let mut orphaned: Vec<i32> = Vec::new();
    let mut orphaned_sockets: Vec<i32> = Vec::new();
    let mut seen_files: BTreeSet<*const Mutex<VfsEntry>> = BTreeSet::new();
    let mut pipe_closures: Vec<PipeClosure> = Vec::new();
    for slot in removed_slots {
        let file_id: *const Mutex<VfsEntry> = Arc::as_ptr(&slot.file);
        let removed_refs: usize = *removed_ref_counts.get(&file_id).unwrap_or(&0);
        if !seen_files.insert(file_id) || Arc::strong_count(&slot.file) != removed_refs {
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
            // A socket token holds the `networkd` descriptor: as the final reference, the endpoint
            // must be closed on `networkd` so it does not leak. This arm must stay distinct so a
            // socket is never mistaken for a hostfs or pipe handle.
            VfsFileHandle::Socket(h) => orphaned_sockets.push(h.remote_fd()),
            _ => {},
        }
        // `slot` — and the open file description it holds — is dropped here, after the registry lock
        // has been released; a pipe end's `Drop` decrements its reader/writer count as part of that.
    }
    ProcessExitReclaim {
        orphaned_hostfs_fds: orphaned,
        orphaned_socket_fds: orphaned_sockets,
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
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    let Some(state) = procs.get(&current_pid()) else {
        return false;
    };
    match state.slots.get(&fd) {
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
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    let Some(state) = procs.get(&current_pid()) else {
        return false;
    };
    match state.slots.get(&fd) {
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
// Socket FD Helpers
//==================================================================================================

/// Allocates a flat descriptor slot for a socket endpoint that `networkd` already created.
///
/// This is the second step of socket creation: once `networkd` returns the endpoint's remote
/// descriptor, libposix asks vfsd to bind it to a flat slot via this function. The returned
/// descriptor is the lowest free flat number — the same allocation every other object goes through
/// — and the slot holds a [`SocketHandle`] so vfsd owns the socket's per-descriptor flags and its
/// place in `fork`/`exec`/`close` accounting. Socket I/O still flows directly to `networkd`, keyed
/// off `remote_fd`.
pub fn vfs_register_socket(remote_fd: i32) -> Result<c_int, Fat32Error> {
    alloc_fd(VfsFileHandle::Socket(SocketHandle::new(remote_fd)))
}

/// Returns the `networkd` descriptor backing a socket file descriptor, or `None` if `fd` is not a
/// socket slot in the current process.
///
/// vfsd uses this to forward a last-reference socket close to `networkd`, the socket analogue of
/// [`vfs_hostfs_remote_fd`].
pub fn vfs_socket_remote_fd(fd: c_int) -> Option<i32> {
    let file: OpenFile = entry_arc(fd).ok()?;
    let guard = file.lock();
    match &guard.handle {
        VfsFileHandle::Socket(h) => Some(h.remote_fd()),
        _ => None,
    }
}

/// Reports whether closing `fd` would drop the final reference to a socket endpoint.
///
/// Returns `true` when `fd` refers to a socket slot and the current process holds the only
/// descriptor for that endpoint's open file description, so closing it (or the owning process
/// exiting) must close the `networkd` endpoint. Returns `false` when other descriptors still share
/// it (for example in a forked child), when `fd` is not a socket, or when `fd` is invalid. This
/// does not modify the descriptor table.
pub fn vfs_socket_is_last_ref(fd: c_int) -> bool {
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    let Some(state) = procs.get(&current_pid()) else {
        return false;
    };
    match state.slots.get(&fd) {
        Some(slot) => {
            // Must be a socket and the sole reference for closing to release the endpoint.
            Arc::strong_count(&slot.file) == 1
                && matches!(&slot.file.lock().handle, VfsFileHandle::Socket(_))
        },
        None => false,
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
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    let state: &ProcessState = procs.get(&current_pid())?;
    Some(state.slots.get(&fd)?.fd_flags)
}

/// Sets the per-descriptor flags (`FD_CLOEXEC`/`FD_CLOFORK`) for `fd` in the current process.
///
/// Returns [`Fat32Error::InvalidFd`] if `fd` is invalid or refers to no open descriptor. Because the
/// flags are stored per descriptor, updating one descriptor does not affect another that shares the
/// same open file description.
pub fn vfs_set_fd_flags(fd: c_int, flags: FdFlags) -> Result<(), Fat32Error> {
    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    let state: &mut ProcessState = procs.get_mut(&current_pid()).ok_or(Fat32Error::InvalidFd)?;
    {
        let slot: &mut Slot = state.slots.get_mut(&fd).ok_or(Fat32Error::InvalidFd)?;
        slot.fd_flags = flags;
    }
    // Changing a descriptor's flags mutates the table; advance the coherence generation.
    state.bump_generation();
    Ok(())
}

//==================================================================================================
// Coherence Generation
//==================================================================================================

/// Returns the current descriptor-table generation of the process the VFS is operating on behalf of,
/// or `0` if it has no recorded state yet.
///
/// vfsd returns this value with descriptor-allocating responses (e.g. `openat`) so that libposix can
/// stamp each cache entry with the generation it was learned at. The counter is advanced by every
/// table-mutating operation, so a stale entry can later be recognized by comparing its stored
/// generation against this one.
pub fn vfs_current_generation() -> u64 {
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    procs
        .get(&current_pid())
        .map(|state| state.generation)
        .unwrap_or(0)
}

//==================================================================================================
// Path Routing
//==================================================================================================

/// Returns `true` if the given path is handled by the VFS.
pub fn is_vfs_path(path: &str) -> bool {
    fat32_backend::exists(path)
}

/// The backend a descriptor's slot is bound to, as reported by [`vfs_resolve`].
///
/// vfsd is the routing authority once descriptor numbers no longer encode their backend: it answers
/// what a flat number like `4` actually is — a console stream, a vfsd-served object, or a socket —
/// from its authoritative slot table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VfsRoute {
    /// A console stream; the `backend_fd` is the standard stream number (`0`/`1`/`2`).
    Console,
    /// A vfsd-served object (regular file, directory, host file, or pipe end).
    Vfs,
    /// A socket; the `backend_fd` is the descriptor `networkd` assigned.
    Socket,
}

/// Resolves a flat descriptor to its backend route and the descriptor that backend expects.
///
/// This is the authoritative answer libposix consults on a resolution-cache miss once numbers stop
/// encoding their backend. The route follows the slot's handle: a console token reports
/// [`VfsRoute::Console`] and the stream number it stands for, a socket token reports
/// [`VfsRoute::Socket`] and its `networkd` descriptor, and every vfsd-served handle reports
/// [`VfsRoute::Vfs`] addressed by the raw descriptor. Returns `None` if the current process holds no
/// slot for `fd`.
pub fn vfs_resolve(fd: c_int) -> Option<(VfsRoute, c_int)> {
    // Clone the shared open file description out from under the registry lock, then inspect its
    // handle without nesting the registry lock under the per-entry lock.
    let file: OpenFile = {
        let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
            PROCESSES.lock();
        procs.get(&current_pid())?.slots.get(&fd)?.file.clone()
    };
    let guard = file.lock();
    let resolution: (VfsRoute, c_int) = match &guard.handle {
        VfsFileHandle::Console(h) => (
            VfsRoute::Console,
            match h.stream() {
                ConsoleStream::Stdin => STDIN_FILENO,
                ConsoleStream::Stdout => STDOUT_FILENO,
                ConsoleStream::Stderr => STDERR_FILENO,
            },
        ),
        VfsFileHandle::Socket(h) => (VfsRoute::Socket, h.remote_fd()),
        // Every other handle is served by vfsd, addressed by the raw descriptor.
        VfsFileHandle::Fat32(_)
        | VfsFileHandle::DirectRead(_)
        | VfsFileHandle::Directory(_)
        | VfsFileHandle::HostFs(_)
        | VfsFileHandle::Pipe(_) => (VfsRoute::Vfs, fd),
    };
    Some(resolution)
}

/// Resolves a `dirfd` + `path` pair into an absolute VFS path.
///
/// If `path` is absolute, it is returned as-is (dirfd is ignored per POSIX).
/// If `dirfd` is `AT_FDCWD`, the path is resolved against the VFS current
/// working directory. If `dirfd` is a directory descriptor, the path is resolved
/// relative to that directory's path.
///
/// Returns `None` if `dirfd` is neither `AT_FDCWD` nor a directory descriptor of the current
/// process, indicating that VFS cannot handle this request.
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

    // Relative path with a directory descriptor: resolve against that directory. Validity is the
    // slot's handle type, not the descriptor number — a non-directory or absent descriptor yields
    // `None` below rather than being pre-screened by a number range.
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

/// Populates stat fields for a console-backed descriptor (a character device).
///
/// A console stream reports as a character device (`S_IFCHR`) with a stable identity: a synthetic
/// console `st_dev` distinct from the VFS file device (1) and the pipefs device (2), and an `st_ino`
/// derived from the stream so the three standard streams have distinct, stable inodes. Because the
/// inode follows the stream rather than the slot, a `dup`'d console descriptor reports the same
/// `(st_dev, st_ino)` as its source, matching POSIX shared-identity expectations. A console carries
/// no size or on-disk blocks, so both are zero.
fn populate_console_stat_fields(buf: &mut ::sysapi::sys_stat::stat, stream: ConsoleStream) {
    // Fixed epoch timestamp: 2024-01-01T00:00:00Z (1704067200).
    const FIXED_EPOCH: i64 = 1_704_067_200;

    // Stable per-stream inode: stdin/stdout/stderr get distinct values that a duplicate inherits.
    let ino: u64 = match stream {
        ConsoleStream::Stdin => 1,
        ConsoleStream::Stdout => 2,
        ConsoleStream::Stderr => 3,
    };
    buf.st_size = 0;
    buf.st_nlink = 1;
    buf.st_dev = 3; // Synthetic console device, distinct from VFS file (1) and pipefs (2).
    buf.st_ino = ino;
    buf.st_mode = file_type::S_IFCHR | file_mode::S_IRUSR | file_mode::S_IWUSR;
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
    let console_stream: Option<ConsoleStream> = match &entry.handle {
        VfsFileHandle::Console(h) => Some(h.stream()),
        _ => None,
    };
    let is_dir: bool = matches!(&entry.handle, VfsFileHandle::Directory(_));
    let size: u64 = entry.handle.size()?;

    // Zero-initialize the stat buffer.
    unsafe {
        ::core::ptr::write_bytes(buf as *mut ::sysapi::sys_stat::stat, 0, 1);
    }

    if let Some(stream) = console_stream {
        // A console descriptor is a character device with a stable, dup-shared identity; the kernel
        // does not serve `fstat` on the console, so vfsd synthesizes it from the slot it owns.
        populate_console_stat_fields(buf, stream);
    } else if let Some(pipe_id) = pipe_id {
        populate_pipe_stat_fields(buf, pipe_id);
    } else {
        populate_stat_fields(buf, size, is_dir);
    }

    Ok(())
}

/// Closes a descriptor held in the process's flat slot table.
///
/// In the flat namespace any descriptor present in the process's slot table can be closed by its
/// raw number, including a low console slot — freeing it for the lowest-free allocator to reuse.
pub fn vfs_close(fd: c_int) -> Result<(), Fat32Error> {
    // Detach the descriptor while holding the registry lock, but defer dropping the open file
    // description until the lock is released. Its drop may run backend teardown (e.g. `File::drop`,
    // which takes a global VFS lock); deferring keeps the registry lock from ever nesting with
    // backend locks.
    let removed: Option<Slot> = {
        let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
            PROCESSES.lock();
        let state: &mut ProcessState =
            procs.get_mut(&current_pid()).ok_or(Fat32Error::InvalidFd)?;
        let removed: Option<Slot> = state.slots.remove(&fd);
        // Removing a descriptor mutates the table; advance the coherence generation.
        if removed.is_some() {
            state.bump_generation();
        }
        removed
    };
    match removed {
        // The slot — and the open file description it holds — is dropped here, after the registry
        // lock has been released.
        Some(_slot) => Ok(()),
        None => Err(Fat32Error::InvalidFd),
    }
}

/// Duplicates a descriptor into the lowest free slot at or above `min_fd`.
///
/// The new descriptor refers to the *same* open file description as `oldfd` — the shared [`Arc`] is
/// cloned, so the two descriptors share one file offset and status flags, exactly as POSIX `dup`
/// and `fcntl(F_DUPFD)` require. The duplicate carries its own per-descriptor flags with
/// `FD_CLOEXEC` cleared, because POSIX mandates that a duplicate start with close-on-exec off; the
/// source descriptor's flags are untouched. Allocation is lowest-free within the flat namespace, so
/// the returned number is the smallest free one that is at least `min_fd`.
///
/// This is the single primitive behind both `dup` (`min_fd == 0`) and `fcntl(F_DUPFD, arg)`
/// (`min_fd == arg`).
///
/// # Errors
///
/// - [`Fat32Error::InvalidFd`] if `oldfd` refers to no open descriptor in the current process.
/// - [`Fat32Error::InvalidArgument`] if `min_fd` is negative.
/// - [`Fat32Error::TooManyOpenFiles`] if no free descriptor at or above `min_fd` exists within the
///   flat namespace.
pub fn vfs_dup_from(oldfd: c_int, min_fd: c_int) -> Result<c_int, Fat32Error> {
    if min_fd < 0 {
        return Err(Fat32Error::InvalidArgument);
    }

    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    let state: &mut ProcessState = procs.get_mut(&current_pid()).ok_or(Fat32Error::InvalidFd)?;
    // Clone the source's shared open file description; the duplicate aliases it (shared offset).
    let file: OpenFile = state
        .slots
        .get(&oldfd)
        .map(|slot| slot.file.clone())
        .ok_or(Fat32Error::InvalidFd)?;
    // Lowest free descriptor at or above `min_fd` within the flat namespace.
    let fd: c_int = (min_fd..MAX_OPEN_FDS)
        .find(|candidate| !state.slots.contains_key(candidate))
        .ok_or(Fat32Error::TooManyOpenFiles)?;
    // `Slot::new` starts from default (empty) per-descriptor flags, so `FD_CLOEXEC` is cleared on
    // the duplicate while the source slot's flags are left untouched.
    state.slots.insert(fd, Slot::new(file));
    // Duplicating a descriptor graduates a lazily-inserted placeholder into an active state and
    // mutates the table; advance the coherence generation.
    state.initialized = true;
    state.bump_generation();
    Ok(fd)
}

/// Duplicates `oldfd` into the lowest free descriptor (POSIX `dup`).
///
/// Equivalent to [`vfs_dup_from`] with a minimum of `0`: the new descriptor is the lowest free
/// number in the flat namespace and aliases `oldfd`'s open file description with `FD_CLOEXEC`
/// cleared.
pub fn vfs_dup(oldfd: c_int) -> Result<c_int, Fat32Error> {
    vfs_dup_from(oldfd, 0)
}

/// Re-points `newfd` at `oldfd`'s open file description (POSIX `dup2`).
///
/// After a successful call `newfd` aliases the same open file description as `oldfd` — sharing its
/// offset and status flags — and carries its own per-descriptor flags with `FD_CLOEXEC` cleared.
/// `dup2(fd, fd)` is a no-op that returns `fd`, provided `fd` is open. When `newfd` was already
/// open, its previous slot is dropped here, but the caller must first capture any last-reference
/// reclaim it owes (a host-backed remote close, or a pipe EOF/`EPIPE` wakeup) exactly as it would
/// for [`vfs_close`]; this routine performs only the table mutation.
///
/// The displaced open file description is dropped *after* the registry lock is released, mirroring
/// [`vfs_close`], so backend teardown never nests under the registry lock.
///
/// # Errors
///
/// - [`Fat32Error::InvalidFd`] if `oldfd` refers to no open descriptor, or if `newfd` is negative
///   or outside the flat namespace.
pub fn vfs_dup2(oldfd: c_int, newfd: c_int) -> Result<c_int, Fat32Error> {
    // `newfd` must be a legal descriptor number within the flat namespace.
    if !(0..MAX_OPEN_FDS).contains(&newfd) {
        return Err(Fat32Error::InvalidFd);
    }
    // Re-point the slot while holding the registry lock, but defer dropping the displaced open file
    // description until the lock is released: its drop may run backend teardown (e.g. `File::drop`,
    // which takes a global VFS lock), and deferring keeps the registry lock from nesting with
    // backend locks — the same discipline as `vfs_close`.
    let displaced: Option<Slot> = {
        let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
            PROCESSES.lock();
        let state: &mut ProcessState =
            procs.get_mut(&current_pid()).ok_or(Fat32Error::InvalidFd)?;
        // `oldfd` must be open.
        let file: OpenFile = state
            .slots
            .get(&oldfd)
            .map(|slot| slot.file.clone())
            .ok_or(Fat32Error::InvalidFd)?;
        // `dup2(fd, fd)` returns `fd` without disturbing the slot or the generation.
        if oldfd == newfd {
            return Ok(newfd);
        }
        // Re-point `newfd` at `oldfd`'s description with fresh per-descriptor flags (`FD_CLOEXEC`
        // cleared); the previous occupant, if any, is returned for deferred drop.
        let displaced: Option<Slot> = state.slots.insert(newfd, Slot::new(file));
        state.initialized = true;
        state.bump_generation();
        displaced
    };
    // The displaced slot — and the open file description it held — is dropped here, after the
    // registry lock has been released.
    drop(displaced);
    Ok(newfd)
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
/// Serves the per-descriptor flag commands and the open-file status-flag commands directly from the
/// state vfsd owns:
///
/// - `F_GETFD`/`F_SETFD` read and write the per-descriptor flags (`FD_CLOEXEC`, `FD_CLOFORK`) stored
///   on the slot. Because these flags live per descriptor, setting them on one descriptor never
///   affects another that shares the same open file description through `dup` or `fork`.
/// - `F_GETFL`/`F_SETFL` read and write the open-file status flags. `F_SETFL` stores only the
///   mutable subset (currently `O_NONBLOCK`); the access-mode and creation bits are not changeable
///   per POSIX.
///
/// The duplication command `F_DUPFD` is *not* handled here: it is a slot-table allocation that the
/// daemon routes to [`vfs_dup_from`]. Any other command returns [`Fat32Error::NotSupported`].
pub fn vfs_fcntl(fd: c_int, cmd: c_int, arg: c_int) -> Result<c_int, Fat32Error> {
    match cmd {
        // Per-descriptor flags live on the slot, so duplicates keep independent close-on-exec /
        // close-on-fork settings.
        file_control_request::F_GETFD => {
            Ok(vfs_get_fd_flags(fd).ok_or(Fat32Error::InvalidFd)?.bits())
        },
        file_control_request::F_SETFD => {
            vfs_set_fd_flags(fd, FdFlags::try_from_bits(arg)?)?;
            Ok(0)
        },
        file_control_request::F_GETFL => Ok(entry_arc(fd)?.lock().status_flags),
        file_control_request::F_SETFL => {
            // Persist only the mutable status-flag subset (currently `O_NONBLOCK`). The remaining
            // bits (access mode, creation flags) are not changeable via `F_SETFL` per POSIX.
            entry_arc(fd)?.lock().status_flags = arg & file_status_flags::O_NONBLOCK;
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

    // -- flat allocation & resolution tests --------------------------------------

    /// Tests that allocation is flat and lowest-free: descriptors are handed out from `0` upward and
    /// a freed number is reused before any higher one. Every object draws from one flat range, so an
    /// allocated descriptor is simply the smallest free number.
    #[test]
    fn alloc_is_lowest_free() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7401);
        set_current_process(pid);

        // A fresh process with no console seeded allocates from 0 upward.
        let a: c_int = vfs_alloc_hostfs(10, false, None).expect("alloc a");
        let b: c_int = vfs_alloc_hostfs(11, false, None).expect("alloc b");
        let c: c_int = vfs_alloc_hostfs(12, false, None).expect("alloc c");
        assert_eq!((a, b, c), (0, 1, 2), "flat allocation hands out the lowest free numbers");
        assert!(
            [a, b, c].iter().all(|fd| (0..MAX_OPEN_FDS).contains(fd)),
            "an allocated descriptor must fall within the flat namespace"
        );

        // Freeing the middle descriptor makes it the lowest free, so the next alloc reuses it.
        vfs_close(b).expect("close b");
        let d: c_int = vfs_alloc_hostfs(13, false, None).expect("alloc d");
        assert_eq!(d, b, "a freed number is reused before any higher one");

        forget_processes(&[pid]);
    }

    /// Tests that the first `open` in a process whose console slots are seeded returns `3`, because
    /// `0`/`1`/`2` are occupied by the standard streams — the POSIX lowest-free expectation.
    #[test]
    fn first_open_after_console_seed_returns_three() {
        let _guard = FORK_TEST_GUARD.lock();
        let root: ProcessIdentifier = root_process_identifier();

        vfs_seed_root_console(root);
        set_current_process(root);
        let fd: c_int = vfs_alloc_hostfs(20, false, None).expect("alloc after seed");
        assert_eq!(fd, 3, "console occupies 0/1/2, so the first open returns 3");

        // Closing stdin frees slot 0, which the lowest-free allocator then reuses ahead of 4.
        vfs_close(STDIN_FILENO).expect("close stdin");
        let reused: c_int = vfs_alloc_hostfs(21, false, None).expect("alloc after closing stdin");
        assert_eq!(reused, STDIN_FILENO, "closing stdin makes 0 the lowest free number");

        forget_processes(&[root]);
    }

    /// Tests that [`vfs_resolve`] answers a descriptor's backend from its slot handle: a console
    /// token reports its stream number, a socket token its networkd descriptor, and every
    /// vfsd-served handle the raw descriptor. An absent descriptor resolves to `None`.
    #[test]
    fn vfs_resolve_reports_backend_by_handle() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7411);
        set_current_process(pid);

        let console_fd: c_int =
            alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
                .expect("console alloc");
        let socket_fd: c_int =
            alloc_fd(VfsFileHandle::Socket(SocketHandle::new(77))).expect("socket alloc");
        let file_fd: c_int = vfs_alloc_hostfs(30, false, None).expect("hostfs alloc");

        // The console token's backend is its stream number (stdout = 1), not its slot number.
        assert_eq!(
            vfs_resolve(console_fd),
            Some((VfsRoute::Console, STDOUT_FILENO)),
            "a console token resolves to its stream number"
        );
        assert_eq!(
            vfs_resolve(socket_fd),
            Some((VfsRoute::Socket, 77)),
            "a socket token resolves to its networkd descriptor"
        );
        assert_eq!(
            vfs_resolve(file_fd),
            Some((VfsRoute::Vfs, file_fd)),
            "a vfsd-served handle resolves to its raw descriptor"
        );
        assert_eq!(vfs_resolve(4096), None, "an absent descriptor resolves to None");

        forget_processes(&[pid]);
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
            .map(|state| state.slots.len())
            .unwrap_or(0)
    }

    /// Returns the descriptor-table generation recorded for a process, or `None` if it has no state.
    fn registry_generation(pid: ProcessIdentifier) -> Option<u64> {
        PROCESSES.lock().get(&pid).map(|state| state.generation)
    }

    /// Reads the virtual position of a process's descriptor through its shared open file
    /// description.
    fn fd_virtual_pos(pid: ProcessIdentifier, fd: c_int) -> Option<off_t> {
        let procs = PROCESSES.lock();
        let state = procs.get(&pid)?;
        let entry: OpenFile = state.slots.get(&fd)?.file.clone();
        let pos: off_t = entry.lock().virtual_pos;
        Some(pos)
    }

    /// Writes the virtual position of a process's descriptor through its shared open file
    /// description.
    fn set_fd_virtual_pos(pid: ProcessIdentifier, fd: c_int, pos: off_t) {
        let procs = PROCESSES.lock();
        if let Some(state) = procs.get(&pid) {
            if let Some(slot) = state.slots.get(&fd) {
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
        // Clear the one-shot root-seeding latch so a later test starts from a pristine state.
        ROOT_CONSOLE_SEEDED.store(false, Ordering::Relaxed);
    }

    /// Tests that the descriptor-table generation advances on every table mutation (open, flag
    /// change, close) and is inherited by a forked child.
    #[test]
    fn generation_advances_on_table_mutations() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7101), ProcessIdentifier::from(0x7102));

        set_current_process(parent);
        let start: u64 = vfs_current_generation();

        // Allocating a descriptor (open) advances the generation.
        let fd: c_int = vfs_alloc_hostfs(50, false, None).expect("alloc should succeed");
        let after_open: u64 = vfs_current_generation();
        assert!(after_open > start, "open must advance the generation");

        // A per-descriptor flag change advances the generation.
        let mut flags: FdFlags = FdFlags::default();
        flags.set_close_on_exec(true);
        vfs_set_fd_flags(fd, flags).expect("set fd flags should succeed");
        let after_flags: u64 = vfs_current_generation();
        assert!(after_flags > after_open, "a flag change must advance the generation");

        // A forked child inherits the parent's current generation.
        vfs_fork_clone(parent, child).expect("fork clone should succeed");
        assert_eq!(
            registry_generation(child),
            Some(after_flags),
            "the child must inherit the parent's generation"
        );

        // Closing a descriptor advances the generation. Closing through the child (the parent still
        // holds the shared open file description) exercises the bump without last-reference
        // teardown.
        set_current_process(child);
        vfs_close(fd).expect("close should succeed");
        assert!(vfs_current_generation() > after_flags, "close must advance the generation");

        forget_processes(&[parent, child]);
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

    /// Tests that [`FdFlags::try_from_bits`] accepts the recognized descriptor-flag bits and rejects
    /// any bit outside `FD_CLOEXEC`/`FD_CLOFORK` with [`Fat32Error::InvalidArgument`], matching POSIX
    /// `fcntl(F_SETFD)` `EINVAL` semantics rather than silently masking unknown bits.
    #[test]
    fn fd_flags_try_from_bits_validates() {
        use ::sysapi::fcntl::file_descriptor_flags::{
            FD_CLOEXEC,
            FD_CLOFORK,
        };

        // Recognized bits — alone and combined — are accepted and round-trip exactly.
        assert_eq!(FdFlags::try_from_bits(0).map(FdFlags::bits), Ok(0));
        assert_eq!(FdFlags::try_from_bits(FD_CLOEXEC).map(FdFlags::bits), Ok(FD_CLOEXEC));
        assert_eq!(FdFlags::try_from_bits(FD_CLOFORK).map(FdFlags::bits), Ok(FD_CLOFORK));
        assert_eq!(
            FdFlags::try_from_bits(FD_CLOEXEC | FD_CLOFORK).map(FdFlags::bits),
            Ok(FD_CLOEXEC | FD_CLOFORK),
        );

        // Any bit outside the recognized set is rejected, even alongside a recognized bit.
        assert_eq!(FdFlags::try_from_bits(0x4), Err(Fat32Error::InvalidArgument));
        assert_eq!(FdFlags::try_from_bits(FD_CLOEXEC | 0x4), Err(Fat32Error::InvalidArgument));
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

    /// Tests that [`vfs_register_socket`] allocates a flat slot bound to a socket token, so the
    /// descriptor resolves to the `networkd` descriptor it routes to, is reported as a socket, and
    /// is the last reference when held alone.
    #[test]
    fn register_socket_allocates_flat_slot() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7220);
        set_current_process(pid);

        let fd: c_int = vfs_register_socket(2050).expect("register socket should succeed");
        // The first allocation in a fresh process is the lowest free flat number.
        assert_eq!(fd, 0, "a socket is allocated the lowest free flat descriptor");
        assert_eq!(
            vfs_resolve(fd),
            Some((VfsRoute::Socket, 2050)),
            "a socket slot resolves to its networkd descriptor"
        );
        assert_eq!(vfs_socket_remote_fd(fd), Some(2050), "the socket reports its remote fd");
        assert!(vfs_socket_is_last_ref(fd), "the sole reference is the last reference");
        // A non-socket descriptor is not reported as a socket.
        let file_fd: c_int = vfs_alloc_hostfs(70, false, None).expect("alloc file");
        assert_eq!(vfs_socket_remote_fd(file_fd), None, "a hostfs descriptor is not a socket");
        assert!(!vfs_socket_is_last_ref(file_fd), "a non-socket is never a socket last reference");

        forget_processes(&[pid]);
    }

    /// Tests that a close-on-exec socket is released at exec: its `networkd` descriptor is surfaced
    /// in [`ProcessExitReclaim::orphaned_socket_fds`] so the daemon closes the endpoint, while a
    /// non-close-on-exec socket survives at its original number.
    #[test]
    fn exec_cloexec_reclaims_socket() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7221);
        set_current_process(pid);

        // A close-on-exec socket and a surviving socket.
        let cloexec_fd: c_int = vfs_register_socket(2060).expect("register cloexec socket");
        let mut flags: FdFlags = FdFlags::default();
        flags.set_close_on_exec(true);
        vfs_set_fd_flags(cloexec_fd, flags).expect("set close-on-exec");
        let survivor_fd: c_int = vfs_register_socket(2061).expect("register surviving socket");

        let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
        assert_eq!(reclaim.orphaned_socket_fds.len(), 1, "only the cloexec socket is reclaimed");
        assert_eq!(
            reclaim.orphaned_socket_fds[0], 2060,
            "the cloexec socket's networkd descriptor is surfaced for closing"
        );
        // The surviving socket is still resolvable at its original number.
        assert_eq!(
            vfs_resolve(survivor_fd),
            Some((VfsRoute::Socket, 2061)),
            "a non-cloexec socket survives exec"
        );

        forget_processes(&[pid]);
    }

    /// Tests that close-on-exec reports a duplicated socket endpoint only once when every alias is
    /// flagged close-on-exec. The daemon sends one close per reported `networkd` descriptor, so a
    /// shared socket open-file description must not surface duplicate reclaim records.
    #[test]
    fn exec_cloexec_reclaims_duplicated_socket_once() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7222);
        set_current_process(pid);

        let fd: c_int = vfs_register_socket(2062).expect("register socket");
        let dup_fd: c_int = vfs_dup(fd).expect("dup socket");
        let mut cloexec: FdFlags = FdFlags::default();
        cloexec.set_close_on_exec(true);
        vfs_set_fd_flags(fd, cloexec).expect("set close-on-exec on original");
        vfs_set_fd_flags(dup_fd, cloexec).expect("set close-on-exec on duplicate");

        let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
        assert_eq!(
            reclaim.orphaned_socket_fds,
            [2062],
            "a duplicated socket endpoint must be reclaimed exactly once"
        );
        assert_eq!(vfs_get_fd_flags(fd), None, "the original alias is dropped");
        assert_eq!(vfs_get_fd_flags(dup_fd), None, "the duplicate alias is dropped");

        forget_processes(&[pid]);
    }

    /// Tests that `fork()` copies a slot's per-descriptor flags into the child and that the copies
    /// are independent, so changing the child's flags does not affect the parent's. This is the
    /// per-descriptor semantics the later close-on-exec/close-on-fork plans depend on.
    ///
    /// Close-on-exec is used here rather than close-on-fork precisely because a close-on-fork slot
    /// is now *dropped* by the clone (see [`fork_drops_close_on_fork_slots`]), so it would not
    /// survive to have its flags inspected in the child.
    #[test]
    fn fork_copies_independent_descriptor_flags() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7201), ProcessIdentifier::from(0x7202));

        // Parent allocates a console-backed descriptor and marks it close-on-exec.
        set_current_process(parent);
        let fd: c_int = alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("console alloc should succeed");
        assert_eq!(vfs_get_fd_flags(fd), Some(FdFlags::default()), "flags default to empty");
        let mut flags: FdFlags = FdFlags::default();
        flags.set_close_on_exec(true);
        vfs_set_fd_flags(fd, flags).expect("setting flags should succeed");

        // Fork: the child inherits a copy of the descriptor's flags.
        vfs_fork_clone(parent, child).expect("fork clone should succeed");
        set_current_process(child);
        let child_flags: FdFlags = vfs_get_fd_flags(fd).expect("child should inherit the slot");
        assert!(child_flags.close_on_exec(), "child inherits close-on-exec");
        assert!(!child_flags.close_on_fork(), "child must not gain close-on-fork");

        // The flags are per descriptor: clearing the child's must leave the parent's untouched.
        vfs_set_fd_flags(fd, FdFlags::default()).expect("clearing child flags should succeed");
        assert_eq!(vfs_get_fd_flags(fd), Some(FdFlags::default()), "child flags should be cleared");
        set_current_process(parent);
        let parent_flags: FdFlags = vfs_get_fd_flags(fd).expect("parent slot should remain");
        assert!(parent_flags.close_on_exec(), "parent close-on-exec must be unchanged");

        forget_processes(&[parent, child]);
    }

    /// Tests that `fork()` drops descriptors flagged close-on-fork (`FD_CLOFORK`) from the child
    /// while cloning every other slot. This is the close-on-fork semantics this plan wires in: a
    /// flagged slot is not inherited, but an unflagged one — including the standard streams — is.
    #[test]
    fn fork_drops_close_on_fork_slots() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7241), ProcessIdentifier::from(0x7242));

        set_current_process(parent);
        // One descriptor flagged close-on-fork, one left unflagged.
        let clofork_fd: c_int =
            alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
                .expect("console alloc should succeed");
        let kept_fd: c_int =
            vfs_alloc_hostfs(70, false, None).expect("hostfs alloc should succeed");
        let mut clofork: FdFlags = FdFlags::default();
        clofork.set_close_on_fork(true);
        vfs_set_fd_flags(clofork_fd, clofork).expect("setting close-on-fork should succeed");
        let parent_generation: u64 =
            registry_generation(parent).expect("parent generation should be recorded");

        // Fork: the flagged descriptor is dropped in the child; the unflagged one is inherited.
        vfs_fork_clone(parent, child).expect("fork clone should succeed");
        set_current_process(child);
        assert_eq!(vfs_get_fd_flags(clofork_fd), None, "close-on-fork descriptor must be dropped");
        assert!(
            registry_generation(child).expect("child generation should be recorded")
                > parent_generation,
            "dropping a close-on-fork descriptor must advance the child's generation"
        );
        assert_eq!(
            vfs_hostfs_remote_fd(kept_fd),
            Some(70),
            "an unflagged descriptor must still be inherited"
        );

        // The parent keeps both descriptors: close-on-fork affects only the child.
        set_current_process(parent);
        assert!(
            vfs_get_fd_flags(clofork_fd).is_some(),
            "parent retains its close-on-fork descriptor"
        );

        forget_processes(&[parent, child]);
    }

    /// Tests that applying close-on-exec drops only the descriptors flagged `FD_CLOEXEC`, leaves
    /// every other descriptor at its number, and advances the table generation so the new image's
    /// cache is rebuilt against the post-close-on-exec table.
    #[test]
    fn exec_cloexec_drops_flagged_and_keeps_others() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7261);
        set_current_process(pid);

        // One descriptor flagged close-on-exec, one left unflagged.
        let cloexec_fd: c_int =
            alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
                .expect("console alloc should succeed");
        let kept_fd: c_int =
            alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stderr)))
                .expect("console alloc should succeed");
        let mut cloexec: FdFlags = FdFlags::default();
        cloexec.set_close_on_exec(true);
        vfs_set_fd_flags(cloexec_fd, cloexec).expect("setting close-on-exec should succeed");
        let generation_before: u64 =
            registry_generation(pid).expect("generation should be recorded");

        // A console token holds no external resource, so nothing is reclaimed.
        let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
        assert!(reclaim.orphaned_hostfs_fds.is_empty(), "console drop orphans no hostfs fd");
        assert!(reclaim.pipe_closures.is_empty(), "console drop closes no pipe end");

        // The flagged descriptor is gone; the unflagged one survives at its number.
        assert_eq!(vfs_get_fd_flags(cloexec_fd), None, "close-on-exec descriptor must be dropped");
        assert!(vfs_resolve(cloexec_fd).is_none(), "the dropped descriptor must not resolve");
        assert!(vfs_get_fd_flags(kept_fd).is_some(), "an unflagged descriptor must survive");
        assert!(vfs_resolve(kept_fd).is_some(), "the surviving descriptor must still resolve");
        assert!(
            registry_generation(pid).expect("generation should be recorded") > generation_before,
            "dropping a close-on-exec descriptor must advance the generation"
        );

        forget_processes(&[pid]);
    }

    /// Tests that applying close-on-exec runs the same last-reference accounting as `close`: a
    /// host-backed descriptor for which the process held the final reference is surfaced for
    /// closing on hostfsd, and a pipe end whose count reaches zero is surfaced so the daemon can
    /// wake its counterpart.
    #[test]
    fn exec_cloexec_reports_pipe_and_hostfs_reclaim() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7262);
        set_current_process(pid);

        // A pipe whose write end is flagged close-on-exec; the read end stays unflagged.
        let (read_fd, write_fd): (c_int, c_int) = vfs_pipe().expect("pipe creation should succeed");
        let (pipe_id, _): (u64, bool) = vfs_pipe_id(write_fd).expect("write fd should be a pipe");
        let mut cloexec: FdFlags = FdFlags::default();
        cloexec.set_close_on_exec(true);
        vfs_set_fd_flags(write_fd, cloexec).expect("setting close-on-exec should succeed");
        // A host-backed descriptor, also flagged close-on-exec, for which this process is the sole
        // reference.
        let hostfs_fd: c_int =
            vfs_alloc_hostfs(70, false, None).expect("hostfs alloc should succeed");
        vfs_set_fd_flags(hostfs_fd, cloexec).expect("setting close-on-exec should succeed");

        let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
        assert_eq!(
            reclaim.orphaned_hostfs_fds.len(),
            1,
            "the host-backed descriptor's remote fd must be reclaimed"
        );
        assert_eq!(reclaim.orphaned_hostfs_fds[0], 70, "the reclaimed remote fd must be 70");
        assert_eq!(reclaim.pipe_closures.len(), 1, "exactly the write end is reclaimed");
        assert_eq!(reclaim.pipe_closures[0].pipe_id, pipe_id, "the closure targets our pipe");
        assert!(reclaim.pipe_closures[0].was_write, "the reclaimed end is the write end");

        // The read end survives because it was not flagged.
        assert!(vfs_resolve(read_fd).is_some(), "the unflagged read end must survive");
        assert_eq!(vfs_get_fd_flags(write_fd), None, "the flagged write end must be dropped");
        assert_eq!(
            vfs_get_fd_flags(hostfs_fd),
            None,
            "the flagged hostfs descriptor must be dropped"
        );

        forget_processes(&[pid]);
    }

    /// Tests that applying close-on-exec to a process with no flagged descriptor is a no-op: every
    /// descriptor survives, nothing is reclaimed, and the generation is left unchanged.
    #[test]
    fn exec_cloexec_without_flagged_is_noop() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7263);
        set_current_process(pid);

        let fd: c_int = alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
            .expect("console alloc should succeed");
        let generation_before: u64 =
            registry_generation(pid).expect("generation should be recorded");

        let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
        assert!(reclaim.orphaned_hostfs_fds.is_empty(), "nothing is reclaimed");
        assert!(reclaim.pipe_closures.is_empty(), "nothing is reclaimed");
        assert!(vfs_resolve(fd).is_some(), "the unflagged descriptor survives");
        assert_eq!(
            registry_generation(pid).expect("generation should be recorded"),
            generation_before,
            "a no-op must not advance the generation"
        );

        forget_processes(&[pid]);
    }

    /// Tests that applying close-on-exec to an unregistered process reclaims nothing and does not
    /// panic.
    #[test]
    fn exec_cloexec_unknown_pid_is_noop() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7264);
        // Deliberately leave the process unregistered.
        let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
        assert!(reclaim.orphaned_hostfs_fds.is_empty(), "an unknown process reclaims nothing");
        assert!(reclaim.pipe_closures.is_empty(), "an unknown process reclaims nothing");
    }

    /// Tests that close-on-exec last-reference accounting respects shared open file descriptions: a
    /// host-backed description still referenced by a surviving `dup` sibling is not reclaimed when
    /// its close-on-exec alias is dropped.
    #[test]
    fn exec_cloexec_shared_description_not_reclaimed() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7265);
        set_current_process(pid);

        // A host-backed descriptor duplicated onto a second number: the two share one description.
        let hostfs_fd: c_int =
            vfs_alloc_hostfs(71, false, None).expect("hostfs alloc should succeed");
        let dup_fd: c_int = vfs_dup(hostfs_fd).expect("dup should succeed");
        // Flag only the original close-on-exec; the dup sibling stays unflagged.
        let mut cloexec: FdFlags = FdFlags::default();
        cloexec.set_close_on_exec(true);
        vfs_set_fd_flags(hostfs_fd, cloexec).expect("setting close-on-exec should succeed");

        let reclaim: ProcessExitReclaim = vfs_exec_cloexec(pid);
        assert!(
            reclaim.orphaned_hostfs_fds.is_empty(),
            "a description still held by a dup sibling must not be reclaimed"
        );
        assert_eq!(vfs_get_fd_flags(hostfs_fd), None, "the flagged alias must be dropped");
        assert_eq!(
            vfs_hostfs_remote_fd(dup_fd),
            Some(71),
            "the surviving dup sibling must keep the description"
        );

        forget_processes(&[pid]);
    }

    /// Tests that `F_SETFD` stores the per-descriptor flags and `F_GETFD` reads them back
    /// (<https://github.com/nanvix/nanvix/issues/2604>).
    #[test]
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

    /// Tests that `F_SETFD` updates only the addressed descriptor, so a forked child and its parent
    /// keep independent flags (<https://github.com/nanvix/nanvix/issues/2604>).
    #[test]
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

    /// Tests that at process exit a lone socket token surfaces its `networkd` descriptor in
    /// [`ProcessExitReclaim::orphaned_socket_fds`] (so the daemon closes the endpoint), while a lone
    /// console token contributes nothing. Crucially, neither is mistaken for a hostfs or pipe
    /// handle.
    #[test]
    fn process_exit_reclaims_socket_and_nothing_for_console() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7211);
        set_current_process(pid);

        // A console token and a socket token, each the sole reference held by this process.
        alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stderr)))
            .expect("console alloc should succeed");
        alloc_fd(VfsFileHandle::Socket(SocketHandle::new(99)))
            .expect("socket alloc should succeed");

        // Exit reclaims the socket's networkd descriptor as the sole reference, and nothing for the
        // inert console token or for hostfs/pipe handles.
        let reclaim: ProcessExitReclaim = vfs_process_exit(pid);
        assert!(
            reclaim.orphaned_hostfs_fds.is_empty(),
            "console and socket tokens own no hostfs descriptor"
        );
        assert!(
            reclaim.pipe_closures.is_empty(),
            "console and socket tokens trigger no pipe closures"
        );
        assert_eq!(
            reclaim.orphaned_socket_fds.len(),
            1,
            "exactly one socket descriptor is reclaimed"
        );
        assert_eq!(
            reclaim.orphaned_socket_fds[0], 99,
            "a sole-reference socket surfaces its networkd descriptor for closing"
        );

        forget_processes(&[pid]);
    }

    /// Tests that process exit reports a duplicated socket endpoint only once. A `dup` sibling
    /// shares the same open-file description, so vfsd must not ask `networkd` to close the same
    /// remote endpoint once per descriptor slot when the process exits with both aliases live.
    #[test]
    fn process_exit_reclaims_duplicated_socket_once() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7212);
        set_current_process(pid);

        let fd: c_int = vfs_register_socket(100).expect("register socket");
        let dup_fd: c_int = vfs_dup(fd).expect("dup socket");

        let reclaim: ProcessExitReclaim = vfs_process_exit(pid);
        assert_eq!(
            reclaim.orphaned_socket_fds,
            [100],
            "a duplicated socket endpoint must be reclaimed exactly once"
        );
        assert!(reclaim.orphaned_hostfs_fds.is_empty(), "no hostfs fds were opened");
        assert!(reclaim.pipe_closures.is_empty(), "no pipe ends were opened");
        let _ = dup_fd;

        forget_processes(&[pid]);
    }

    /// Tests that a socket inherited across `fork()` is reference-counted, not shared: a child
    /// closing its inherited copy must not tear down the parent's socket endpoint. This is the core
    /// regression guard for `nanvix/nanvix#2609`. Before the child closes, neither process is the
    /// last reference (so vfsd would not forward the close to `networkd`); once the child has
    /// closed, the parent is the sole, last reference and its socket remains fully resolvable.
    #[test]
    fn fork_socket_close_is_isolated() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7231), ProcessIdentifier::from(0x7232));

        // Parent creates a socket; it is the sole reference before the fork.
        set_current_process(parent);
        let fd: c_int = vfs_register_socket(2070).expect("register socket should succeed");
        assert!(vfs_socket_is_last_ref(fd), "the only descriptor is the last reference");

        // Fork: parent and child now share the same socket open-file description.
        vfs_fork_clone(parent, child).expect("fork clone should succeed");
        set_current_process(parent);
        assert!(!vfs_socket_is_last_ref(fd), "parent must not be the last reference after fork");
        set_current_process(child);
        assert!(!vfs_socket_is_last_ref(fd), "child must not be the last reference after fork");

        // The child closes its inherited copy. Because it is not the last reference, vfsd would not
        // forward the endpoint close to `networkd`, so the parent's socket survives.
        vfs_close(fd).expect("child close should succeed");
        assert_eq!(vfs_socket_remote_fd(fd), None, "the child's socket descriptor is gone");

        // The parent's socket is untouched and is now the sole, last reference.
        set_current_process(parent);
        assert_eq!(
            vfs_socket_remote_fd(fd),
            Some(2070),
            "the parent's socket must survive the child's close (nanvix/nanvix#2609)"
        );
        assert!(
            vfs_socket_is_last_ref(fd),
            "the parent becomes the last reference once the child has closed"
        );

        forget_processes(&[parent, child]);
    }

    /// Tests that a child exiting while it still holds an inherited socket does not close the shared
    /// endpoint: `vfs_process_exit` surfaces no orphaned socket because the parent still references
    /// it. Only when the last holder (the parent) exits is the endpoint reclaimed — exactly once —
    /// so there is neither a premature close (`nanvix/nanvix#2609`) nor a leak. This covers the
    /// issue's "implicitly when the child exits" case, which the explicit-close acceptance test does
    /// not exercise.
    #[test]
    fn fork_socket_child_exit_preserves_parent() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7241), ProcessIdentifier::from(0x7242));

        // Parent creates a socket and forks; the child inherits a shared reference.
        set_current_process(parent);
        let fd: c_int = vfs_register_socket(2071).expect("register socket should succeed");
        vfs_fork_clone(parent, child).expect("fork clone should succeed");

        // The child exits without closing its inherited socket. The endpoint must not be orphaned,
        // because the parent still holds a reference to it.
        let child_reclaim: ProcessExitReclaim = vfs_process_exit(child);
        assert!(
            child_reclaim.orphaned_socket_fds.is_empty(),
            "a child exiting while the parent holds the socket must not close the endpoint \
             (nanvix/nanvix#2609)"
        );

        // The parent's socket is intact and is now the sole, last reference.
        set_current_process(parent);
        assert_eq!(
            vfs_socket_remote_fd(fd),
            Some(2071),
            "the parent's socket must survive the child's exit"
        );
        assert!(
            vfs_socket_is_last_ref(fd),
            "the parent is the last reference after the child exits"
        );

        // When the last holder exits, the endpoint is reclaimed exactly once so `networkd` closes
        // it — no leak.
        let parent_reclaim: ProcessExitReclaim = vfs_process_exit(parent);
        assert_eq!(
            parent_reclaim.orphaned_socket_fds,
            [2071],
            "the last holder's exit closes the endpoint exactly once (no leak)"
        );

        forget_processes(&[parent, child]);
    }

    // -- root console seeding tests ----------------------------------------------

    /// Tests that seeding the root installs console tokens at descriptors `0`/`1`/`2` carrying the
    /// matching stream identity, and marks the root's state active. This is what makes vfsd the
    /// authoritative bookkeeper of the standard streams.
    #[test]
    fn seed_root_console_installs_stdio() {
        let _guard = FORK_TEST_GUARD.lock();
        let root: ProcessIdentifier = root_process_identifier();

        vfs_seed_root_console(root);

        let procs = PROCESSES.lock();
        let state = procs
            .get(&root)
            .expect("root state should exist after seeding");
        assert!(state.initialized, "a seeded root must be active");
        assert_eq!(state.slots.len(), 3, "seeding installs exactly the three standard streams");
        for (fd, expected) in [
            (STDIN_FILENO, ConsoleStream::Stdin),
            (STDOUT_FILENO, ConsoleStream::Stdout),
            (STDERR_FILENO, ConsoleStream::Stderr),
        ] {
            let slot = state
                .slots
                .get(&fd)
                .expect("console slot should be installed");
            match &slot.file.lock().handle {
                VfsFileHandle::Console(h) => {
                    assert_eq!(
                        h.stream(),
                        expected,
                        "console slot should carry its stream identity"
                    )
                },
                _ => panic!("descriptor {fd} should be a console token"),
            }
            assert_eq!(slot.fd_flags, FdFlags::default(), "console slots are not close-on-fork");
        }
        drop(procs);

        forget_processes(&[root]);
    }

    /// Tests that root-console seeding runs exactly once: the one-shot guard makes a second call —
    /// even for a different pid — a no-op that registers nothing. This is what guarantees a forked
    /// child, which reaches vfsd only after the root, is never seeded directly.
    #[test]
    fn seed_runs_once() {
        let _guard = FORK_TEST_GUARD.lock();
        let root: ProcessIdentifier = root_process_identifier();
        let other: ProcessIdentifier = ProcessIdentifier::from(ProcessIdentifier::VFSD_RAW + 2);

        vfs_seed_root_console(root);
        // A second seeding attempt (here for a different pid) must do nothing at all.
        vfs_seed_root_console(other);

        assert_eq!(registry_open_fd_count(root), 3, "root keeps its three console descriptors");
        assert!(
            PROCESSES.lock().get(&other).is_none(),
            "a second seeding attempt must not register or seed another process"
        );

        forget_processes(&[root, other]);
    }

    /// Tests that a forked child whose first request races ahead of the fork-clone notification is
    /// not seeded directly. The non-root seeding attempt must leave the child as an uninitialized
    /// placeholder and must not consume the root's one-shot latch; once the root is seeded, the
    /// fork-clone overwrites the placeholder and installs the shared console slots in the child.
    #[test]
    fn racing_child_seed_attempt_remains_placeholder() {
        let _guard = FORK_TEST_GUARD.lock();
        let root: ProcessIdentifier = root_process_identifier();
        let child: ProcessIdentifier = ProcessIdentifier::from(ProcessIdentifier::VFSD_RAW + 2);

        // This mirrors vfsd's syscall path for a child request that arrives before procd's
        // fork-clone notification: bind the child, then try the root-seeding hook.
        set_current_process(child);
        vfs_seed_root_console(child);
        {
            let procs = PROCESSES.lock();
            let child_state = procs.get(&child).expect("child placeholder should exist");
            assert!(
                !child_state.initialized,
                "a non-root seeding attempt must not activate the racing child"
            );
            assert!(child_state.slots.is_empty(), "racing child must not receive console slots");
            assert!(
                procs.get(&root).is_none(),
                "non-root seeding must not create the root state as a side effect"
            );
        }

        // Now vfsd learns the root pid (e.g., through the fork-clone parent) and seeds it. The
        // clone must overwrite the still-uninitialized child placeholder.
        vfs_seed_root_console(root);
        vfs_fork_clone(root, child).expect("fork clone over racing placeholder should succeed");

        let procs = PROCESSES.lock();
        let root_state = procs.get(&root).expect("root state should exist");
        let child_state = procs.get(&child).expect("child state should exist");
        assert!(child_state.initialized, "fork-cloned child should be active");
        for fd in [STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO] {
            let root_slot = root_state.slots.get(&fd).expect("root console slot");
            let child_slot = child_state.slots.get(&fd).expect("child console slot");
            assert!(
                Arc::ptr_eq(&root_slot.file, &child_slot.file),
                "child descriptor {fd} must inherit the root's console slot through fork-clone"
            );
        }
        drop(procs);

        forget_processes(&[root, child]);
    }

    /// Tests that a forked child's console descriptors are the very same open file descriptions as
    /// the root's — `Arc`-shared, not freshly created — so the standard streams keep POSIX shared
    /// offsets across a fork. Root-only seeding plus cloning is the sole path by which a child
    /// receives `0`/`1`/`2`.
    #[test]
    fn fork_shares_console_slots_with_parent() {
        let _guard = FORK_TEST_GUARD.lock();
        let root: ProcessIdentifier = root_process_identifier();
        let child: ProcessIdentifier = ProcessIdentifier::from(ProcessIdentifier::VFSD_RAW + 2);

        vfs_seed_root_console(root);
        vfs_fork_clone(root, child).expect("fork clone should succeed");

        let procs = PROCESSES.lock();
        let root_state = procs.get(&root).expect("root state should exist");
        let child_state = procs.get(&child).expect("child state should exist");
        for fd in [STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO] {
            let root_slot = root_state.slots.get(&fd).expect("root console slot");
            let child_slot = child_state.slots.get(&fd).expect("child console slot");
            assert!(
                Arc::ptr_eq(&root_slot.file, &child_slot.file),
                "child descriptor {fd} must share the root's open file description, not a fresh \
                 one"
            );
        }
        drop(procs);

        forget_processes(&[root, child]);
    }

    /// Tests that an active child that has closed every descriptor is still not overwritten by a
    /// fork-clone. The explicit active marker — not a descriptor count — guards the table, so an
    /// initialized-but-empty state is correctly refused where the old `is_empty` predicate would
    /// have clobbered it.
    #[test]
    fn fork_into_active_child_with_no_descriptors_is_rejected() {
        let _guard = FORK_TEST_GUARD.lock();
        let (parent, child): (ProcessIdentifier, ProcessIdentifier) =
            (ProcessIdentifier::from(0x7331), ProcessIdentifier::from(0x7332));

        // The child becomes active by allocating a descriptor, then closes it: its table is empty
        // but it remains a real, initialized process.
        set_current_process(child);
        let fd: c_int = vfs_alloc_hostfs(71, false, None).expect("alloc should succeed");
        vfs_close(fd).expect("close should succeed");
        assert_eq!(registry_open_fd_count(child), 0, "child holds no descriptors after closing");

        // Forking onto it must still be refused: an active state is never an overwritable
        // placeholder, even with an empty table.
        let err: Fat32Error =
            vfs_fork_clone(parent, child).expect_err("fork into active child should fail");
        assert_eq!(err, Fat32Error::AlreadyExists, "active child must be rejected even when empty");

        forget_processes(&[parent, child]);
    }

    // -- dup / dup2 / F_DUPFD slot-table tests ------------------------------------

    /// Tests that `vfs_dup` aliases the source's open file description into the lowest free slot and
    /// that the duplicate starts with `FD_CLOEXEC` cleared without disturbing the source's flags.
    #[test]
    fn dup_aliases_description_and_clears_cloexec() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7501);
        set_current_process(pid);

        let fd: c_int = vfs_alloc_hostfs(60, false, None).expect("alloc");
        // Mark the source close-on-exec so we can prove the duplicate does not inherit it.
        let mut flags: FdFlags = FdFlags::default();
        flags.set_close_on_exec(true);
        vfs_set_fd_flags(fd, flags).expect("set source flags");

        let dup_fd: c_int = vfs_dup(fd).expect("dup");
        assert_eq!(dup_fd, fd + 1, "dup returns the lowest free descriptor");

        // The duplicate shares the source's open file description (same offset).
        {
            let procs = PROCESSES.lock();
            let state = procs.get(&pid).expect("state");
            assert!(
                Arc::ptr_eq(&state.slots[&fd].file, &state.slots[&dup_fd].file),
                "dup must alias the same open file description"
            );
        }
        // POSIX: the duplicate has close-on-exec cleared; the source keeps its own flag.
        assert!(
            !vfs_get_fd_flags(dup_fd).expect("dup flags").close_on_exec(),
            "a dup'd descriptor must start with FD_CLOEXEC cleared"
        );
        assert!(
            vfs_get_fd_flags(fd).expect("source flags").close_on_exec(),
            "the source descriptor's flags must be unaffected by dup"
        );

        forget_processes(&[pid]);
    }

    /// Tests that `vfs_dup_from` (the `fcntl(F_DUPFD, arg)` primitive) allocates the lowest free
    /// descriptor at or above the requested minimum.
    #[test]
    fn dup_from_allocates_at_or_above_min() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7502);
        set_current_process(pid);

        let fd: c_int = vfs_alloc_hostfs(61, false, None).expect("alloc");
        // Request a duplicate no lower than 10; nothing is allocated there yet, so it lands on 10.
        let high: c_int = vfs_dup_from(fd, 10).expect("dup_from");
        assert_eq!(high, 10, "F_DUPFD must return the lowest free fd >= arg");
        // A second request with the same floor skips the now-occupied 10 and lands on 11.
        let higher: c_int = vfs_dup_from(fd, 10).expect("dup_from again");
        assert_eq!(higher, 11, "the next duplicate skips the occupied floor");
        assert!(
            Arc::ptr_eq(&entry_arc(fd).unwrap(), &entry_arc(high).unwrap()),
            "F_DUPFD must alias the same open file description"
        );

        assert_eq!(
            vfs_dup_from(fd, -1).unwrap_err(),
            Fat32Error::InvalidArgument,
            "F_DUPFD with a negative floor must be rejected"
        );

        forget_processes(&[pid]);
    }

    /// Tests that `vfs_dup2` re-points `newfd` at `oldfd`'s description across backends and that the
    /// previous occupant of `newfd` is released.
    #[test]
    fn dup2_repoints_across_backends() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7503);
        set_current_process(pid);

        // A console descriptor and an independent host-backed file descriptor.
        let console_fd: c_int =
            alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
                .expect("console alloc");
        let file_fd: c_int = vfs_alloc_hostfs(62, false, None).expect("file alloc");
        // The file is the sole reference before dup2.
        assert!(vfs_hostfs_is_last_ref(file_fd), "file should be the last reference pre-dup2");

        // dup2(file_fd, console_fd): the console slot now aliases the file's description — the
        // cross-backend redirection the old split model could not express.
        let ret: c_int = vfs_dup2(file_fd, console_fd).expect("dup2");
        assert_eq!(ret, console_fd, "dup2 returns newfd");
        assert!(
            Arc::ptr_eq(&entry_arc(file_fd).unwrap(), &entry_arc(console_fd).unwrap()),
            "newfd must alias oldfd's open file description after dup2"
        );
        // Resolution now reports the former console descriptor as a vfsd-served file.
        assert_eq!(
            vfs_resolve(console_fd),
            Some((VfsRoute::Vfs, console_fd)),
            "the redirected descriptor must resolve to the file backend"
        );

        forget_processes(&[pid]);
    }

    /// Tests that `dup2(fd, fd)` is a no-op that returns `fd` and leaves the table generation
    /// unchanged, while `dup2` with an invalid `oldfd` or an out-of-range `newfd` is rejected.
    #[test]
    fn dup2_noop_and_invalid_arguments() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7504);
        set_current_process(pid);

        let fd: c_int = vfs_alloc_hostfs(63, false, None).expect("alloc");
        let before: u64 = vfs_current_generation();
        assert_eq!(vfs_dup2(fd, fd).expect("dup2 self"), fd, "dup2(fd, fd) returns fd");
        assert_eq!(
            vfs_current_generation(),
            before,
            "a no-op dup2 must not advance the generation"
        );

        // An invalid source descriptor is rejected.
        assert_eq!(vfs_dup2(4096, fd).unwrap_err(), Fat32Error::InvalidFd, "bad oldfd rejected");
        // A newfd outside the flat namespace is rejected.
        assert_eq!(
            vfs_dup2(fd, MAX_OPEN_FDS).unwrap_err(),
            Fat32Error::InvalidFd,
            "newfd outside the flat namespace is rejected"
        );

        forget_processes(&[pid]);
    }

    /// Tests that when `dup2` displaces the last reference to a host-backed description, that
    /// reference is dropped so the caller (vfsd) becomes responsible for reclaiming the remote
    /// handle. The replaced slot must no longer reference the old description.
    #[test]
    fn dup2_releases_displaced_last_reference() {
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7505);
        set_current_process(pid);

        let src_fd: c_int = vfs_alloc_hostfs(64, false, None).expect("src alloc");
        let victim_fd: c_int = vfs_alloc_hostfs(65, false, None).expect("victim alloc");
        // The victim is the only reference to remote fd 65, so the daemon would inspect it for
        // reclaim before calling dup2.
        assert!(vfs_hostfs_is_last_ref(victim_fd), "victim is the last reference pre-dup2");
        assert_eq!(vfs_hostfs_remote_fd(victim_fd), Some(65));

        vfs_dup2(src_fd, victim_fd).expect("dup2");
        // The victim slot now points at the source's remote fd, and the displaced description is
        // gone (its Arc was dropped by dup2).
        assert_eq!(
            vfs_hostfs_remote_fd(victim_fd),
            Some(64),
            "the displaced slot must now alias the source's remote handle"
        );

        forget_processes(&[pid]);
    }

    // -- console fstat tests ------------------------------------------------------

    /// Tests that `vfs_fstat` reports a console descriptor as a character device with a stable
    /// identity, and that a `dup`'d console descriptor shares that identity.
    #[test]
    fn fstat_console_reports_stable_character_device() {
        use ::sysapi::sys_stat::file_type;
        let _guard = FORK_TEST_GUARD.lock();
        let pid: ProcessIdentifier = ProcessIdentifier::from(0x7507);
        set_current_process(pid);

        let console_fd: c_int =
            alloc_fd(VfsFileHandle::Console(ConsoleHandle::new(ConsoleStream::Stdout)))
                .expect("console alloc");

        let mut st: ::sysapi::sys_stat::stat = Default::default();
        vfs_fstat(console_fd, &mut st).expect("fstat console");
        assert_eq!(
            st.st_mode & file_type::S_IFMT,
            file_type::S_IFCHR,
            "a console descriptor must report as a character device"
        );
        assert_eq!(st.st_size, 0, "a console has no size");
        let (dev, ino): (_, _) = (st.st_dev, st.st_ino);

        // fstat is stable across calls.
        let mut st2: ::sysapi::sys_stat::stat = Default::default();
        vfs_fstat(console_fd, &mut st2).expect("fstat console again");
        assert_eq!((st2.st_dev, st2.st_ino), (dev, ino), "console identity must be stable");

        // A dup'd console descriptor shares the same identity.
        let dup_fd: c_int = vfs_dup(console_fd).expect("dup console");
        let mut st3: ::sysapi::sys_stat::stat = Default::default();
        vfs_fstat(dup_fd, &mut st3).expect("fstat dup'd console");
        assert_eq!(
            (st3.st_dev, st3.st_ino),
            (dev, ino),
            "a dup'd console descriptor must share its source's identity"
        );

        forget_processes(&[pid]);
    }
}
