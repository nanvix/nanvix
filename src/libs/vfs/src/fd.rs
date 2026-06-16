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
        }
    }

    /// Writes data to the file.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.write(buf),
            VfsFileHandle::DirectRead(_) => Err(Fat32Error::ReadOnly),
            VfsFileHandle::Directory(_) => Err(Fat32Error::NotSupported),
            VfsFileHandle::HostFs(_) => Err(Fat32Error::NotSupported),
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
        }
    }

    /// Returns the file size in bytes.
    pub fn size(&mut self) -> Result<u64, Fat32Error> {
        match self {
            VfsFileHandle::Fat32(file) => file.size(),
            VfsFileHandle::DirectRead(handle) => Ok(handle.size() as u64),
            VfsFileHandle::Directory(_) => Ok(0),
            VfsFileHandle::HostFs(_) => Ok(0),
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

/// Per-process VFS state: the open file descriptor table and the current working directory.
///
/// Each process is given its own descriptor table so that closing a descriptor in one process does
/// not affect another, and its own working directory so that `chdir()` is process-local. `fork()`
/// gives the child a copy of this state: the descriptor slots are cloned as shared references to
/// the parent's open file descriptions, while the working directory is deep-copied.
struct ProcessState {
    /// File descriptor slots indexed by `(fd - VFS_FD_BASE)`.
    slots: Vec<Option<OpenFile>>,
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
    /// The descriptor slots are cloned as shared references to the same open file descriptions,
    /// so the parent and child share file offsets. The working directory is deep-copied.
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
        .and_then(|slot| slot.clone())
        .ok_or(Fat32Error::InvalidFd)
}

/// Allocates a new file descriptor for the given handle in the current process.
fn alloc_fd(handle: VfsFileHandle) -> Result<c_int, Fat32Error> {
    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    let state: &mut ProcessState = procs.entry(current_pid()).or_insert_with(ProcessState::new);
    for i in 0..VFS_MAX_OPEN_FILES {
        if state.slots[i].is_none() {
            state.slots[i] = Some(Arc::new(Mutex::new(VfsEntry {
                handle,
                virtual_pos: 0,
            })));
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

/// Reclaims the per-process filesystem state of a terminated process.
///
/// Drops `pid`'s recorded state, releasing its references to the open file descriptions it held.
/// This keeps the last-reference accounting correct for surviving siblings that still share those
/// descriptions, and prevents the per-process table from growing without bound. Reclaiming an
/// unknown `pid` is a no-op.
///
/// Returns the remote file descriptors of any host-backed open file descriptions for which `pid`
/// held the final reference. The caller (vfsd) must forward a close for each of these to hostfsd:
/// because the process is gone it can no longer close them itself, and the VFS has no other way to
/// release a remote handle. Descriptions still shared with a surviving process are not returned.
#[must_use = "the returned remote fds must be closed on hostfsd or they leak"]
pub fn vfs_process_exit(pid: ProcessIdentifier) -> Vec<i32> {
    // Detach the process's state while holding the registry lock, but defer dropping it until the
    // lock is released. Dropping an open file description may run backend teardown; deferring keeps
    // the registry lock from ever nesting with backend locks, matching `vfs_close`.
    let removed: Option<ProcessState> = {
        let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
            PROCESSES.lock();
        procs.remove(&pid)
    };
    let Some(state) = removed else {
        return Vec::new();
    };
    // The process has been removed from the registry, so an `Arc` strong count of one means no
    // surviving descriptor — in this or any other process — still shares the open file description.
    // Its remote handle must therefore be closed on hostfsd. As the sole owner, the lock below is
    // uncontended.
    let mut orphaned: Vec<i32> = Vec::new();
    for open_file in state.slots.into_iter().flatten() {
        if Arc::strong_count(&open_file) != 1 {
            continue;
        }
        if let VfsFileHandle::HostFs(h) = &open_file.lock().handle {
            orphaned.push(h.remote_fd());
        }
        // `open_file` is dropped here, after the registry lock has been released.
    }
    orphaned
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
        Some(entry) => Arc::strong_count(entry) == 1,
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

/// Gets file status for a VFS file descriptor.
pub fn vfs_fstat(fd: c_int, buf: &mut ::sysapi::sys_stat::stat) -> Result<(), Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;
    let is_dir: bool = matches!(&entry.handle, VfsFileHandle::Directory(_));
    let size: u64 = entry.handle.size()?;

    // Zero-initialize the stat buffer.
    unsafe {
        ::core::ptr::write_bytes(buf as *mut ::sysapi::sys_stat::stat, 0, 1);
    }

    populate_stat_fields(buf, size, is_dir);

    Ok(())
}

/// Closes a VFS file descriptor.
pub fn vfs_close(fd: c_int) -> Result<(), Fat32Error> {
    let idx: usize = fd_index(fd)?;
    // Detach the descriptor while holding the registry lock, but defer dropping the open file
    // description until the lock is released. Its drop may run backend teardown (e.g. `File::drop`,
    // which takes a global VFS lock); deferring keeps the registry lock from ever nesting with
    // backend locks.
    let removed: Option<OpenFile> = {
        let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
            PROCESSES.lock();
        let state: &mut ProcessState =
            procs.get_mut(&current_pid()).ok_or(Fat32Error::InvalidFd)?;
        let slot: &mut Option<OpenFile> = state.slots.get_mut(idx).ok_or(Fat32Error::InvalidFd)?;
        slot.take()
    };
    match removed {
        // The open file description is dropped here, after the registry lock has been released.
        Some(_open_file) => Ok(()),
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
        VfsFileHandle::Directory(_) | VfsFileHandle::HostFs(_) => Err(Fat32Error::NotSupported),
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
        VfsFileHandle::Directory(_) | VfsFileHandle::HostFs(_) => Err(Fat32Error::NotSupported),
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
        VfsFileHandle::DirectRead(_) | VfsFileHandle::Directory(_) | VfsFileHandle::HostFs(_) => {
            Ok(())
        },
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
/// Only `F_GETFL` and `F_SETFL` are supported (as no-ops for FAT32).
/// Other commands return `NotSupported`.
pub fn vfs_fcntl(fd: c_int, cmd: c_int) -> Result<c_int, Fat32Error> {
    // Verify the fd is valid.
    let _file: OpenFile = entry_arc(fd)?;

    match cmd {
        file_control_request::F_GETFD => Ok(0), // No FD flags (no close-on-exec for VFS).
        file_control_request::F_SETFD => Ok(0), // Accept but ignore (no close-on-exec).
        file_control_request::F_GETFL => Ok(0), // No meaningful flags for FAT32.
        file_control_request::F_SETFL => Ok(0), // Accept but ignore (no O_NONBLOCK etc.).
        _ => Err(Fat32Error::NotSupported),     // Other commands not supported.
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
        let entry: OpenFile = state.slots.get(idx)?.clone()?;
        let pos: off_t = entry.lock().virtual_pos;
        Some(pos)
    }

    /// Writes the virtual position of a process's descriptor through its shared open file
    /// description.
    fn set_fd_virtual_pos(pid: ProcessIdentifier, fd: c_int, pos: off_t) {
        let procs = PROCESSES.lock();
        if let Some(state) = procs.get(&pid) {
            let idx: usize = (fd - VFS_FD_BASE) as usize;
            if let Some(Some(entry)) = state.slots.get(idx) {
                entry.lock().virtual_pos = pos;
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
}
