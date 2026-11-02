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
//! 3. Update the descriptor open adapter (or create a new backend module).

//==================================================================================================
// Modules
//==================================================================================================

mod open;

//==================================================================================================
// Imports
//==================================================================================================

use self::open as open_adapter;
use crate::{
    devfs::{
        self,
        DevicePath,
    },
    filesystem,
    identifiers::{
        ConsoleInodeId,
        FilesystemDeviceId,
        HostFsInodeId,
        VfsInodeId,
    },
    line_discipline::{
        ConsoleReadOutcome,
        LineDiscipline,
        TerminalSignal,
    },
    path::{
        AnchoredPath,
        ResolvedPath,
    },
    process::{
        current_cwd,
        current_pid,
        root_process_identifier,
        set_current_cwd,
        OpenFile,
        ProcessState,
        Slot,
        VfsEntry,
        DEFAULT_CWD,
        PROCESSES,
        ROOT_CONSOLE_SEEDED,
    },
};
use ::alloc::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    string::String,
    sync::Arc,
    vec::Vec,
};
use ::core::sync::atomic::Ordering;
use ::fat32::{
    Fat32Error,
    FAT_EPOCH_SECS,
};
use ::spin::Mutex;
use ::sys::pm::{
    GroupIdentifier,
    ProcessIdentifier,
    UserIdentifier,
};
use ::sysapi::{
    fcntl::{
        atflags::{
            AT_FDCWD,
            AT_SYMLINK_NOFOLLOW,
        },
        file_access_mode,
        file_control_request,
        file_creation_flags,
        file_status_flags,
    },
    ffi::{
        c_int,
        c_short,
    },
    poll::poll_flags::{
        POLLIN,
        POLLOUT,
        POLLRDNORM,
        POLLWRNORM,
    },
    sys_ioctl::Winsize,
    sys_stat::{
        file_mode,
        file_type,
        UTIME_NOW,
        UTIME_OMIT,
    },
    sys_types::{
        c_size_t,
        dev_t,
        gid_t,
        ino_t,
        mode_t,
        off_t,
        uid_t,
    },
    termios::Termios,
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

/// Last Unix second representable by FAT (2107-12-31T23:59:59Z).
const MAX_FAT_TIMESTAMP: i64 = 4_354_819_199;

/// Default file mode creation mask.
const DEFAULT_FILE_CREATION_MASK: mode_t = 0;

/// File permission bits affected by the file mode creation mask.
const FILE_PERMISSION_BITS: mode_t = file_mode::S_IRWXU | file_mode::S_IRWXG | file_mode::S_IRWXO;

//==================================================================================================
// Global State
//==================================================================================================

/// Shared console device retained independently of process descriptor tables.
static CONSOLE_TERMINAL: Mutex<Option<Arc<Mutex<LineDiscipline>>>> = Mutex::new(None);

/// Assigns the single Nanvix identity to a POSIX stat result.
fn set_root_ownership(buf: &mut ::sysapi::sys_stat::stat) {
    buf.st_uid = UserIdentifier::ROOT.as_usize() as uid_t;
    buf.st_gid = GroupIdentifier::ROOT.as_usize() as gid_t;
}

/// Returns the process-global console terminal, creating it on first use.
fn console_device() -> Arc<Mutex<LineDiscipline>> {
    let mut terminal: spin::MutexGuard<'_, Option<Arc<Mutex<LineDiscipline>>>> =
        CONSOLE_TERMINAL.lock();
    terminal
        .get_or_insert_with(|| Arc::new(Mutex::new(LineDiscipline::default())))
        .clone()
}

/// Access and line-discipline state of a terminal descriptor.
struct TerminalState {
    /// Standard stream identity, if this is a seeded console descriptor.
    stream: Option<ConsoleStream>,
    /// Shared console line discipline.
    terminal: Arc<Mutex<LineDiscipline>>,
    /// Whether reads are permitted.
    readable: bool,
    /// Whether writes are permitted.
    writable: bool,
}

//==================================================================================================
// Public Re-Exports
//==================================================================================================

pub use crate::{
    descriptor::{
        AccessMode,
        ConsoleHandle,
        ConsoleStream,
        DirectReadHandle,
        DirectoryHandle,
        FdFlags,
        HostFsHandle,
        NullHandle,
        PipeClosure,
        ProcessExitReclaim,
        SocketHandle,
        TerminalDevice,
        TerminalHandle,
        TtyError,
        VfsFileHandle,
        VfsRoute,
    },
    process::set_current_process,
};

//==================================================================================================
// File Descriptor Table
//==================================================================================================

/// Returns a shared reference to the open file description for `fd` in the current process.
///
/// Indexing is flat: the descriptor number is the slot key directly, so the low console slots
/// (`0`/`1`/`2`) and the descriptors handed out by [`alloc_fd`] are looked up the same way. A
/// descriptor with no slot in the current process is rejected. vfsd serves no I/O on console or
/// socket tokens; the I/O methods on [`VfsFileHandle`] reject those handles, so this accessor does
/// not need to special-case them.
pub(crate) fn entry_arc(fd: c_int) -> Result<OpenFile, Fat32Error> {
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
    alloc_fd_with_status(handle, 0)
}

/// Allocates a descriptor with initial open-file status flags.
fn alloc_fd_with_status(handle: VfsFileHandle, status_flags: c_int) -> Result<c_int, Fat32Error> {
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
        status_flags,
    }));
    state.slots.insert(fd, Slot::new(file));
    // Allocating a descriptor graduates a lazily-inserted placeholder into an active state: a
    // process holding a real descriptor must never be overwritten by a later fork-clone.
    state.initialized = true;
    // Allocating a descriptor mutates the table; advance the coherence generation.
    state.bump_generation();
    Ok(fd)
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
    // so the clone below keeps the child's own cwd instead of reverting it to the parent's. An
    // explicit file creation mask is preserved for the same reason, including `umask(0)`.
    let mut child_cwd: Option<String> = None;
    let mut child_file_creation_mask: Option<mode_t> = None;
    if let Some(existing) = procs.get(&child) {
        if existing.initialized {
            return Err(Fat32Error::AlreadyExists);
        }
        if existing.cwd != DEFAULT_CWD {
            child_cwd = Some(existing.cwd.clone());
        }
        child_file_creation_mask = existing.file_creation_mask;
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
    if child_file_creation_mask.is_some() {
        child_state.file_creation_mask = child_file_creation_mask;
    }
    procs.insert(child, child_state);
    Ok(())
}

/// Sets the current process's file mode creation mask and returns the previous mask.
pub fn vfs_umask(mask: mode_t) -> mode_t {
    let mut procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
        PROCESSES.lock();
    let state: &mut ProcessState = procs.entry(current_pid()).or_insert_with(ProcessState::new);
    let previous: mode_t = state
        .file_creation_mask
        .unwrap_or(DEFAULT_FILE_CREATION_MASK);
    state.file_creation_mask = Some(mask & FILE_PERMISSION_BITS);
    previous
}

/// Applies the current process's file mode creation mask to `mode`.
pub fn vfs_apply_umask(mode: mode_t) -> mode_t {
    let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> = PROCESSES.lock();
    let mask: mode_t = procs
        .get(&current_pid())
        .and_then(|state| state.file_creation_mask)
        .unwrap_or(DEFAULT_FILE_CREATION_MASK);
    mode & !mask
}

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
    state.has_controlling_terminal = true;
    // The three standard streams share one terminal device, so they reference a single shared
    // terminal-state object. A `tcsetattr` through any of them is therefore visible through the
    // others, and the shared object flows down every `fork`/`dup` with the descriptor slots.
    let terminal: Arc<Mutex<LineDiscipline>> = console_device();
    for (fd, stream) in [
        (STDIN_FILENO, ConsoleStream::Stdin),
        (STDOUT_FILENO, ConsoleStream::Stdout),
        (STDERR_FILENO, ConsoleStream::Stderr),
    ] {
        let file: OpenFile = Arc::new(Mutex::new(VfsEntry {
            handle: VfsFileHandle::Console(ConsoleHandle::with_terminal(stream, terminal.clone())),
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

/// Detaches a process from the system controlling terminal.
pub fn vfs_detach_controlling_terminal(pid: ProcessIdentifier) {
    if let Some(state) = PROCESSES.lock().get_mut(&pid) {
        state.has_controlling_terminal = false;
    }
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

/// Non-destructively copies bytes from a pipe read end into `buf`.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidFd`] if `fd` is not a pipe or if it is the write end.
pub fn vfs_pipe_peek(fd: c_int, buf: &mut [u8]) -> Result<PipeReadOutcome, Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let guard = file.lock();
    let end: &crate::pipe::PipeEnd = guard.handle.pipe_end().ok_or(Fat32Error::InvalidFd)?;
    end.peek(buf).map_err(|_| Fat32Error::InvalidFd)
}

/// Removes up to `count` bytes from the front of a pipe read end.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidFd`] if `fd` is not a pipe or if it is the write end.
pub fn vfs_pipe_consume(fd: c_int, count: usize) -> Result<usize, Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let guard = file.lock();
    let end: &crate::pipe::PipeEnd = guard.handle.pipe_end().ok_or(Fat32Error::InvalidFd)?;
    end.consume(count).map_err(|_| Fat32Error::InvalidFd)
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
    let path = match AnchoredPath::new(AT_FDCWD, String::from(path)) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let is_devfs: bool = devfs::owns(path.clone()).unwrap_or(false);
    is_devfs || open_adapter::exists(path)
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
        VfsFileHandle::Terminal(_) => (VfsRoute::Terminal, fd),
        // Every other handle is served by vfsd, addressed by the raw descriptor.
        VfsFileHandle::Fat32(_)
        | VfsFileHandle::DirectRead(_)
        | VfsFileHandle::Directory(_)
        | VfsFileHandle::HostFs(_)
        | VfsFileHandle::Null(_)
        | VfsFileHandle::Pipe(_) => (VfsRoute::Vfs, fd),
    };
    Some(resolution)
}

/// Returns the events that can complete immediately on `fd` without performing I/O.
pub fn vfs_poll(fd: c_int, events: c_short) -> Result<c_short, Fat32Error> {
    const READ_EVENTS: c_short = POLLIN | POLLRDNORM;
    const WRITE_EVENTS: c_short = POLLOUT | POLLWRNORM;

    let file: OpenFile = entry_arc(fd)?;
    let (stream, terminal): (ConsoleStream, Arc<Mutex<LineDiscipline>>) = {
        let guard = file.lock();
        match &guard.handle {
            VfsFileHandle::Console(handle) => (handle.stream(), handle.terminal().clone()),
            VfsFileHandle::Fat32(_) | VfsFileHandle::DirectRead(_) | VfsFileHandle::HostFs(_) => {
                return Ok(events & (READ_EVENTS | WRITE_EVENTS))
            },
            VfsFileHandle::Null(handle) => return Ok(handle.poll(events)),
            VfsFileHandle::Terminal(handle) => {
                let mut ready: c_short = 0;
                if handle.readable()
                    && events & READ_EVENTS != 0
                    && handle.terminal().lock().is_readable()
                {
                    ready |= events & READ_EVENTS;
                }
                if handle.writable() {
                    ready |= events & WRITE_EVENTS;
                }
                return Ok(ready);
            },
            VfsFileHandle::Directory(_) => return Ok(events & READ_EVENTS),
            VfsFileHandle::Pipe(end) => return Ok(end.poll(events)),
            VfsFileHandle::Socket(_) => return Err(Fat32Error::NotSupported),
        }
    };

    let mut revents: c_short = 0;
    if stream == ConsoleStream::Stdin && events & READ_EVENTS != 0 && terminal.lock().is_readable()
    {
        revents |= events & READ_EVENTS;
    }
    if matches!(stream, ConsoleStream::Stdout | ConsoleStream::Stderr) {
        revents |= events & WRITE_EVENTS;
    }
    Ok(revents)
}

//==================================================================================================
// POSIX-Compatible Operations
//==================================================================================================

/// Opens a file through the VFS and allocates a system-wide FD.
pub fn vfs_open(path: &str, flags: c_int) -> Result<c_int, Fat32Error> {
    vfs_open_anchored(AnchoredPath::new(AT_FDCWD, String::from(path))?, flags)
}

/// Opens a path whose descriptor provenance has already been captured.
fn vfs_open_anchored(path: AnchoredPath, flags: c_int) -> Result<c_int, Fat32Error> {
    let path = filesystem::VfsResolvedPath::new(path)?;
    if matches!(path.device(), Some(DevicePath::Directory)) {
        if flags & file_creation_flags::O_CREAT != 0 && flags & file_creation_flags::O_EXCL != 0 {
            return Err(Fat32Error::AlreadyExists);
        }
        let access_mode: c_int = flags & file_access_mode::O_ACCMODE;
        if access_mode != file_access_mode::O_RDONLY || flags & file_creation_flags::O_TRUNC != 0 {
            return Err(Fat32Error::NotAFile);
        }
    }
    // O_DIRECTORY and trailing slashes require a directory.
    if flags & file_creation_flags::O_DIRECTORY != 0 || path.has_trailing_separator() {
        let info = filesystem::stat(&path)?;
        if !info.is_dir() {
            return Err(Fat32Error::NotADirectory);
        }
        let normalized = String::from(path.normalized()?);
        let handle = VfsFileHandle::Directory(DirectoryHandle::new(normalized));
        let status_flags: c_int =
            flags & (file_access_mode::O_ACCMODE | file_status_flags::O_NONBLOCK);
        return alloc_fd_with_status(handle, status_flags);
    }

    if let Some(device) = match path.device() {
        Some(DevicePath::Tty) => Some(TerminalDevice::Tty),
        Some(DevicePath::Console) => Some(TerminalDevice::Console),
        _ => None,
    } {
        if flags & file_creation_flags::O_CREAT != 0 && flags & file_creation_flags::O_EXCL != 0 {
            return Err(Fat32Error::AlreadyExists);
        }
        if flags & file_access_mode::O_EXEC != 0 {
            return Err(Fat32Error::PermissionDenied);
        }
        if device == TerminalDevice::Tty {
            let processes: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
                PROCESSES.lock();
            if !processes
                .get(&current_pid())
                .map(|state| state.has_controlling_terminal)
                .unwrap_or(false)
            {
                return Err(Fat32Error::NoDevice);
            }
        }
        let access_mode: AccessMode = match flags & file_access_mode::O_ACCMODE {
            file_access_mode::O_RDONLY => AccessMode::ReadOnly,
            file_access_mode::O_WRONLY => AccessMode::WriteOnly,
            file_access_mode::O_RDWR => AccessMode::ReadWrite,
            _ => return Err(Fat32Error::InvalidArgument),
        };
        let handle: VfsFileHandle =
            VfsFileHandle::Terminal(TerminalHandle::new(device, access_mode, console_device()));
        let status_flags: c_int =
            flags & (file_access_mode::O_ACCMODE | file_status_flags::O_NONBLOCK);
        return alloc_fd_with_status(handle, status_flags);
    }

    let handle: VfsFileHandle = open_adapter::open(&path, flags)?;
    let status_flags: c_int = flags & (file_access_mode::O_ACCMODE | file_status_flags::O_NONBLOCK);
    alloc_fd_with_status(handle, status_flags)
}

/// Opens a file relative to a directory file descriptor through the VFS.
///
/// The raw descriptor and path are kept together until local VFS resolution consumes them.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidFd`] (POSIX `EBADF`) if `dirfd` is not an open
/// descriptor, or [`Fat32Error::NotADirectory`] (POSIX `ENOTDIR`) if it is not a
/// directory. An empty `path` returns [`Fat32Error::NotFound`] (POSIX `ENOENT`).
pub fn vfs_openat(dirfd: c_int, path: &str, flags: c_int) -> Result<c_int, Fat32Error> {
    vfs_open_anchored(AnchoredPath::new(dirfd, String::from(path))?, flags)
}

/// Opens an already-resolved path and allocates a system-wide FD.
///
/// Skips path anchoring: the caller already supplied an absolute path.
pub fn vfs_open_resolved(path: ResolvedPath, flags: c_int) -> Result<c_int, Fat32Error> {
    vfs_open_anchored(AnchoredPath::new(AT_FDCWD, path.into_string())?, flags)
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

    if let VfsFileHandle::Null(handle) = &entry.handle {
        return handle.read().map(|count| count as c_size_t);
    }

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

    if let VfsFileHandle::Null(handle) = &entry.handle {
        return handle.write(buf).map(|count| count as c_size_t);
    }

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

    if let VfsFileHandle::Null(handle) = &entry.handle {
        return handle.seek(whence);
    }

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

/// Updates a POSIX stat buffer from VFS metadata.
trait StatExt {
    fn update_from_vfs(&mut self, info: &filesystem::Stat);
    fn update_from_hostfs(&mut self, info: &filesystem::Stat);
    fn update_from_synthetic(&mut self, info: &filesystem::Stat, device: dev_t, inode: ino_t);
}

impl StatExt for ::sysapi::sys_stat::stat {
    fn update_from_vfs(&mut self, info: &filesystem::Stat) {
        self.update_from_synthetic(
            info,
            FilesystemDeviceId::Vfs.into(),
            VfsInodeId::Fallback.into(),
        );
    }

    fn update_from_hostfs(&mut self, info: &filesystem::Stat) {
        self.update_from_synthetic(
            info,
            FilesystemDeviceId::HostFs.into(),
            HostFsInodeId::Fallback.into(),
        );
    }

    fn update_from_synthetic(&mut self, info: &filesystem::Stat, device: dev_t, inode: ino_t) {
        self.st_size = info.size() as off_t;
        self.st_nlink = if info.is_dir() { 2 } else { 1 };
        self.st_dev = device;
        self.st_ino = inode;
        self.st_mode = if info.is_dir() {
            file_type::S_IFDIR | file_mode::S_IRWXU
        } else {
            file_type::S_IFREG | file_mode::S_IRUSR | file_mode::S_IWUSR
        };
        self.st_blksize = STAT_BLOCK_SIZE;
        self.st_blocks = info.size().div_ceil(STAT_SECTOR_SIZE) as off_t;
        self.st_atim = timespec {
            tv_sec: info.atime(),
            tv_nsec: 0,
        };
        self.st_mtim = timespec {
            tv_sec: info.mtime(),
            tv_nsec: 0,
        };
        self.st_ctim = timespec {
            tv_sec: info.ctime(),
            tv_nsec: 0,
        };
    }
}

/// Populates stat fields for a pipe (FIFO) descriptor.
///
/// A pipe has no name, size, or on-disk blocks: `st_mode` carries `S_IFIFO` with owner read/write
/// permissions and `st_size` is `0`, matching POSIX expectations for an unnamed pipe. `st_ino`
/// carries the pipe's unique identity, and `st_dev` a synthetic pipefs device distinct from the
/// VFS file device, so distinct pipes report distinct `(st_dev, st_ino)` pairs that never collide
/// with regular files — mirroring how a real pipefs assigns one inode per pipe.
fn populate_pipe_stat_fields(buf: &mut ::sysapi::sys_stat::stat, pipe_id: u64, created: i64) {
    buf.st_size = 0;
    buf.st_nlink = 1;
    buf.st_dev = FilesystemDeviceId::PipeFs.into();
    buf.st_ino = pipe_id;
    buf.st_mode = file_type::S_IFIFO | file_mode::S_IRUSR | file_mode::S_IWUSR;
    buf.st_blksize = STAT_BLOCK_SIZE;
    buf.st_blocks = 0;
    // A pipe's times are all its creation instant: never modified on disk, no atime tracking.
    buf.st_atim = timespec {
        tv_sec: created,
        tv_nsec: 0,
    };
    buf.st_mtim = timespec {
        tv_sec: created,
        tv_nsec: 0,
    };
    buf.st_ctim = timespec {
        tv_sec: created,
        tv_nsec: 0,
    };
}

/// Populates stat fields for a console-backed descriptor (a character device).
///
/// A console stream reports as a character device (`S_IFCHR`) with a stable identity: a synthetic
/// console `st_dev` distinct from other filesystems, and an `st_ino` derived from the stream so the
/// three standard streams have distinct, stable inodes. Because the
/// inode follows the stream rather than the slot, a `dup`'d console descriptor reports the same
/// `(st_dev, st_ino)` as its source, matching POSIX shared-identity expectations. A console carries
/// no size or on-disk blocks, so both are zero.
fn populate_console_stat_fields(
    buf: &mut ::sysapi::sys_stat::stat,
    stream: ConsoleStream,
    created: i64,
) {
    // Stable per-stream inode: stdin/stdout/stderr get distinct values that a duplicate inherits.
    let inode: ino_t = match stream {
        ConsoleStream::Stdin => ConsoleInodeId::Stdin.into(),
        ConsoleStream::Stdout => ConsoleInodeId::Stdout.into(),
        ConsoleStream::Stderr => ConsoleInodeId::Stderr.into(),
    };
    buf.st_size = 0;
    buf.st_nlink = 1;
    buf.st_dev = FilesystemDeviceId::Console.into();
    buf.st_ino = inode;
    buf.st_mode = file_type::S_IFCHR | file_mode::S_IRUSR | file_mode::S_IWUSR;
    buf.st_blksize = STAT_BLOCK_SIZE;
    buf.st_blocks = 0;
    // The console has no on-disk metadata; report its creation instant for all three times.
    buf.st_atim = timespec {
        tv_sec: created,
        tv_nsec: 0,
    };
    buf.st_mtim = timespec {
        tv_sec: created,
        tv_nsec: 0,
    };
    buf.st_ctim = timespec {
        tv_sec: created,
        tv_nsec: 0,
    };
}

/// Gets file status for a VFS file descriptor.
pub fn vfs_fstat(fd: c_int, buf: &mut ::sysapi::sys_stat::stat) -> Result<(), Fat32Error> {
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;

    // Zero-initialize the stat buffer.
    unsafe {
        ::core::ptr::write_bytes(buf as *mut ::sysapi::sys_stat::stat, 0, 1);
    }

    match &mut entry.handle {
        VfsFileHandle::Directory(directory) => {
            let path = AnchoredPath::new(AT_FDCWD, String::from(directory.path()))?;
            if let Some(device) = devfs::resolve_anchored(path)? {
                *buf = devfs::posix_stat(device)?;
            } else {
                let info: filesystem::Stat =
                    filesystem::Stat::new(0, true, FAT_EPOCH_SECS, FAT_EPOCH_SECS, FAT_EPOCH_SECS);
                buf.update_from_vfs(&info);
            }
        },
        handle @ (VfsFileHandle::Fat32(_) | VfsFileHandle::DirectRead(_)) => {
            // upstream exposes timestamps through directory entries, not open
            // files. Re-querying by path can identify a different file after
            // rename or unlink, so retain the epoch
            // fallback until the backend exposes metadata through open handles.
            let size: u64 = handle.size()?;
            let info: filesystem::Stat = filesystem::Stat::new(
                size,
                handle.is_dir(),
                FAT_EPOCH_SECS,
                FAT_EPOCH_SECS,
                FAT_EPOCH_SECS,
            );
            buf.update_from_vfs(&info);
        },
        VfsFileHandle::HostFs(handle) => {
            let info: filesystem::Stat = filesystem::Stat::new(
                0,
                handle.is_dir(),
                FAT_EPOCH_SECS,
                FAT_EPOCH_SECS,
                FAT_EPOCH_SECS,
            );
            buf.update_from_hostfs(&info);
        },
        VfsFileHandle::Null(_) => {
            *buf = devfs::null_posix_stat();
        },
        VfsFileHandle::Pipe(end) => {
            populate_pipe_stat_fields(buf, end.pipe_id(), end.created());
        },
        VfsFileHandle::Console(handle) => {
            populate_console_stat_fields(buf, handle.stream(), handle.created());
        },
        VfsFileHandle::Terminal(handle) => match handle.device() {
            TerminalDevice::Tty => *buf = devfs::tty_posix_stat(),
            TerminalDevice::Console => *buf = devfs::console_posix_stat(),
        },
        VfsFileHandle::Socket(_) => {
            // TODO: Report socket metadata instead of treating the descriptor as a regular file.
            let info: filesystem::Stat =
                filesystem::Stat::new(0, false, FAT_EPOCH_SECS, FAT_EPOCH_SECS, FAT_EPOCH_SECS);
            buf.update_from_vfs(&info);
        },
    }
    set_root_ownership(buf);

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
    vfs_stat_anchored(AnchoredPath::new(AT_FDCWD, String::from(path))?, buf)
}

/// Gets file status for a path whose descriptor provenance has already been captured.
fn vfs_stat_anchored(
    path: AnchoredPath,
    buf: &mut ::sysapi::sys_stat::stat,
) -> Result<(), Fat32Error> {
    let path = filesystem::VfsResolvedPath::new(path)?;
    let info = filesystem::stat(&path)?;
    if let Some(device) = path.device() {
        *buf = devfs::posix_stat(device)?;
        return Ok(());
    }

    // Zero-initialize the stat buffer.
    unsafe {
        ::core::ptr::write_bytes(buf as *mut ::sysapi::sys_stat::stat, 0, 1);
    }
    buf.update_from_vfs(&info);
    set_root_ownership(buf);

    Ok(())
}

/// Gets file status for a path relative to a directory file descriptor through the VFS.
///
/// The raw descriptor and path are kept together until local VFS resolution consumes them.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidFd`] (POSIX `EBADF`) if `dirfd` is not an open
/// descriptor, or [`Fat32Error::NotADirectory`] (POSIX `ENOTDIR`) if it is not a
/// directory. An empty `path` returns [`Fat32Error::NotFound`] (POSIX `ENOENT`).
pub fn vfs_fstatat(
    dirfd: c_int,
    path: &str,
    buf: &mut ::sysapi::sys_stat::stat,
) -> Result<(), Fat32Error> {
    vfs_stat_anchored(AnchoredPath::new(dirfd, String::from(path))?, buf)
}

/// Gets file status for an already-resolved path.
pub fn vfs_stat_resolved(
    path: ResolvedPath,
    buf: &mut ::sysapi::sys_stat::stat,
) -> Result<(), Fat32Error> {
    vfs_stat_anchored(AnchoredPath::new(AT_FDCWD, path.into_string())?, buf)
}

/// Renames a file or directory through the VFS.
///
/// Both paths must be on the same VFS mount.
pub fn vfs_rename(old_path: &str, new_path: &str) -> Result<(), Fat32Error> {
    filesystem::rename(
        AnchoredPath::new(AT_FDCWD, String::from(old_path))?,
        AnchoredPath::new(AT_FDCWD, String::from(new_path))?,
    )
}

/// Deletes a file through the VFS.
pub fn vfs_unlink(path: &str) -> Result<(), Fat32Error> {
    filesystem::unlink(AnchoredPath::new(AT_FDCWD, String::from(path))?)
}

/// Creates a directory through the VFS.
pub fn vfs_mkdir(path: &str) -> Result<(), Fat32Error> {
    filesystem::mkdir(AnchoredPath::new(AT_FDCWD, String::from(path))?)
}

/// Creates a directory relative to a directory file descriptor through the VFS.
///
/// The raw descriptor and path are kept together until local VFS resolution consumes them.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidFd`] (POSIX `EBADF`) if `dirfd` is not an open
/// descriptor, or [`Fat32Error::NotADirectory`] (POSIX `ENOTDIR`) if it is not a
/// directory. An empty `path` returns [`Fat32Error::NotFound`] (POSIX `ENOENT`).
pub fn vfs_mkdirat(dirfd: c_int, path: &str) -> Result<(), Fat32Error> {
    filesystem::mkdir(AnchoredPath::new(dirfd, String::from(path))?)
}

/// Creates a directory at an already-resolved path.
pub fn vfs_mkdir_resolved(path: ResolvedPath) -> Result<(), Fat32Error> {
    filesystem::mkdir(AnchoredPath::new(AT_FDCWD, path.into_string())?)
}

/// Removes an empty directory through the VFS.
pub fn vfs_rmdir(path: &str) -> Result<(), Fat32Error> {
    filesystem::rmdir(AnchoredPath::new(AT_FDCWD, String::from(path))?)
}

/// Changes the VFS current working directory.
pub fn vfs_chdir(path: &str) -> Result<(), Fat32Error> {
    let path = AnchoredPath::new(AT_FDCWD, String::from(path))?;
    let normalized = filesystem::change_directory(path)?;
    set_current_cwd(normalized);
    Ok(())
}

/// Commits the current working directory to a hostfs path, bypassing local
/// mount-table validation.
///
/// Unlike [`vfs_chdir`], this performs NO existence/type check: vfsd uses it to
/// finalize a chdir onto a hostfs path after hostfsd has confirmed the target is
/// a directory (hostfs paths live outside the local FAT mount table, so
/// [`vfs_chdir`] would reject them). Taking a [`ResolvedPath`] guarantees an absolute guest path.
/// Preserve its spelling: collapsing `symlink/..` here can select a different directory from the
/// one hostfsd validated. Later relative operations must leave those components for the host.
///
/// Hostfs `getcwd` returns this preserved guest spelling, not a physically canonicalized path;
/// the host's directory check does not return a canonical path over the existing protocol.
pub fn vfs_set_cwd(path: ResolvedPath) {
    set_current_cwd(path.into_string());
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
            VfsFileHandle::Directory(dir) => String::from(dir.path()),
            _ => return Err(Fat32Error::NotADirectory),
        }
    };
    vfs_chdir(&path)
}

/// Gets the VFS current working directory.
///
/// Hostfs paths retain the guest spelling committed by [`vfs_set_cwd`], including symlinks and
/// dot components. Local paths are normalized by [`vfs_chdir`].
pub fn vfs_getcwd() -> Result<alloc::string::String, Fat32Error> {
    if !crate::state::is_initialized() {
        return Err(Fat32Error::NotInitialized);
    }
    Ok(current_cwd())
}

/// Lists directory contents through the VFS.
///
/// Returns a vector of directory entries.
pub fn vfs_readdir(path: &str) -> Result<alloc::vec::Vec<filesystem::DirEntry>, Fat32Error> {
    let path = AnchoredPath::new(AT_FDCWD, String::from(path))?;
    filesystem::read_dir(path)
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
        VfsFileHandle::Null(_) | VfsFileHandle::Terminal(_) => Err(Fat32Error::InvalidArgument),
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
        | VfsFileHandle::Null(_)
        | VfsFileHandle::Pipe(_)
        | VfsFileHandle::Console(_)
        | VfsFileHandle::Terminal(_)
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
        VfsFileHandle::Null(_) | VfsFileHandle::Terminal(_) => Err(Fat32Error::InvalidArgument),
        VfsFileHandle::DirectRead(_)
        | VfsFileHandle::Directory(_)
        | VfsFileHandle::HostFs(_)
        | VfsFileHandle::Pipe(_)
        | VfsFileHandle::Console(_)
        | VfsFileHandle::Socket(_) => Ok(()),
    }
}

/// Returns the shared terminal device state of a seeded console or opened named terminal.
///
/// The shared [`Arc`] is cloned out from under the registry lock and the per-entry lock is released
/// before it is returned, so the caller may lock the terminal without nesting registry or entry
/// locks. A descriptor with no slot yields [`TtyError::BadFd`]; a non-terminal descriptor yields
/// [`TtyError::NotTty`].
fn terminal_handle(fd: c_int) -> Result<TerminalState, TtyError> {
    let file: OpenFile = {
        let procs: spin::MutexGuard<'_, BTreeMap<ProcessIdentifier, ProcessState>> =
            PROCESSES.lock();
        procs
            .get(&current_pid())
            .ok_or(TtyError::BadFd)?
            .slots
            .get(&fd)
            .ok_or(TtyError::BadFd)?
            .file
            .clone()
    };
    let guard = file.lock();
    match &guard.handle {
        VfsFileHandle::Console(handle) => {
            let stream: ConsoleStream = handle.stream();
            Ok(TerminalState {
                stream: Some(stream),
                terminal: handle.terminal().clone(),
                readable: stream == ConsoleStream::Stdin,
                writable: matches!(stream, ConsoleStream::Stdout | ConsoleStream::Stderr),
            })
        },
        VfsFileHandle::Terminal(handle) => Ok(TerminalState {
            stream: None,
            terminal: handle.terminal().clone(),
            readable: handle.readable(),
            writable: handle.writable(),
        }),
        _ => Err(TtyError::NotTty),
    }
}

/// Returns the console stream represented by descriptor `fd`.
pub fn vfs_console_stream(fd: c_int) -> Result<ConsoleStream, TtyError> {
    terminal_handle(fd).and_then(|state| state.stream.ok_or(TtyError::NotTty))
}

/// Returns the read and write access permitted by a terminal descriptor.
pub fn vfs_terminal_access(fd: c_int) -> Result<(bool, bool), TtyError> {
    terminal_handle(fd).map(|state| (state.readable, state.writable))
}

fn console_terminal(fd: c_int) -> Result<Arc<Mutex<LineDiscipline>>, TtyError> {
    terminal_handle(fd).map(|state| state.terminal)
}

/// Reads the terminal attributes of the console descriptor `fd` (`TCGETS`/`tcgetattr`).
pub fn vfs_tty_get_termios(fd: c_int) -> Result<Termios, TtyError> {
    let terminal: Arc<Mutex<LineDiscipline>> = console_terminal(fd)?;
    let termios: Termios = terminal.lock().termios;
    Ok(termios)
}

/// Replaces the terminal attributes of the console descriptor `fd` (`TCSETS`/`tcsetattr`).
///
/// The new attributes are stored on the shared terminal object, so the change is observed by every
/// console descriptor that references it, including descriptors in forked children.
pub fn vfs_tty_set_termios(fd: c_int, termios: Termios) -> Result<(), TtyError> {
    let terminal: Arc<Mutex<LineDiscipline>> = console_terminal(fd)?;
    terminal.lock().set_termios(termios);
    Ok(())
}

/// Reads the window size of the console descriptor `fd` (`TIOCGWINSZ`).
pub fn vfs_tty_get_winsize(fd: c_int) -> Result<Winsize, TtyError> {
    let terminal: Arc<Mutex<LineDiscipline>> = console_terminal(fd)?;
    let winsize: Winsize = terminal.lock().winsize;
    Ok(winsize)
}

/// Replaces the window size of the console descriptor `fd` (`TIOCSWINSZ`).
pub fn vfs_tty_set_winsize(fd: c_int, winsize: Winsize) -> Result<(), TtyError> {
    let terminal: Arc<Mutex<LineDiscipline>> = console_terminal(fd)?;
    terminal.lock().winsize = winsize;
    Ok(())
}

/// Feeds raw input bytes into the console line discipline and returns bytes to echo.
pub fn vfs_console_push_input(fd: c_int, input: &[u8]) -> Result<Vec<u8>, TtyError> {
    let terminal: Arc<Mutex<LineDiscipline>> = console_terminal(fd)?;
    let echo: Vec<u8> = terminal.lock().push_input(input);
    Ok(echo)
}

/// Feeds raw bytes directly into the shared console device.
pub fn vfs_console_device_push_input(input: &[u8]) -> Vec<u8> {
    console_device().lock().push_input(input)
}

/// Drains the terminal-generated signals (`^C`/`^\`/`^Z`) recognized by the console line discipline
/// since the last drain.
///
/// Called right after [`vfs_console_push_input`] so the console daemon can forward each recognized
/// signal to the foreground process group. The returned vector is empty when no signal character was
/// seen in the fed input.
pub fn vfs_console_take_signals(fd: c_int) -> Result<Vec<TerminalSignal>, TtyError> {
    let terminal: Arc<Mutex<LineDiscipline>> = console_terminal(fd)?;
    let signals: Vec<TerminalSignal> = terminal.lock().take_signals();
    Ok(signals)
}

/// Drains terminal-generated signals directly from the shared console device.
pub fn vfs_console_device_take_signals() -> Vec<TerminalSignal> {
    console_device().lock().take_signals()
}

/// Signals end-of-input on the raw console stream.
pub fn vfs_console_push_eof(fd: c_int) -> Result<(), TtyError> {
    let terminal: Arc<Mutex<LineDiscipline>> = console_terminal(fd)?;
    terminal.lock().push_eof();
    Ok(())
}

/// Signals hard end-of-input directly on the shared console device.
pub fn vfs_console_device_push_eof() {
    console_device().lock().push_eof();
}

/// Attempts a non-blocking read from the console line discipline.
pub fn vfs_console_read(fd: c_int, buf: &mut [u8]) -> Result<ConsoleReadOutcome, TtyError> {
    let terminal: Arc<Mutex<LineDiscipline>> = console_terminal(fd)?;
    let outcome: ConsoleReadOutcome = terminal.lock().read(buf);
    Ok(outcome)
}

/// Peeks at an immediate console read without consuming line-discipline state.
pub fn vfs_console_peek(fd: c_int, buf: &mut [u8]) -> Result<ConsoleReadOutcome, TtyError> {
    let terminal: Arc<Mutex<LineDiscipline>> = console_terminal(fd)?;
    let outcome: ConsoleReadOutcome = terminal.lock().peek(buf);
    Ok(outcome)
}

/// Reads from a VFS file descriptor at a given offset without changing position.
///
/// POSIX semantics: reading past EOF returns 0 bytes (not an error).
pub fn vfs_pread(fd: c_int, buf: &mut [u8], offset: off_t) -> Result<c_size_t, Fat32Error> {
    if offset < 0 {
        return Err(Fat32Error::InvalidArgument);
    }
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;

    if let VfsFileHandle::Null(handle) = &entry.handle {
        return handle.read().map(|count| count as c_size_t);
    }

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
    if offset < 0 {
        return Err(Fat32Error::InvalidArgument);
    }
    let file: OpenFile = entry_arc(fd)?;
    let mut guard = file.lock();
    let entry: &mut VfsEntry = &mut guard;

    if let VfsFileHandle::Null(handle) = &entry.handle {
        return handle.write(buf).map(|count| count as c_size_t);
    }

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

/// Checks file accessibility through the VFS.
///
/// Returns `Ok(())` if the path exists, `Err` otherwise.
/// FAT32 does not have UNIX permissions, so only existence is checked.
pub fn vfs_access(path: &str) -> Result<(), Fat32Error> {
    vfs_access_anchored(AnchoredPath::new(AT_FDCWD, String::from(path))?)
}

/// Checks a path whose descriptor provenance has already been captured.
fn vfs_access_anchored(path: AnchoredPath) -> Result<(), Fat32Error> {
    let path = filesystem::VfsResolvedPath::new(path)?;
    filesystem::stat(&path).map(|_| ())
}

/// Checks the accessibility of a file relative to a directory file descriptor.
///
/// The raw descriptor and path are kept together until local VFS resolution consumes them.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidFd`] (POSIX `EBADF`) if `dirfd` is not an open
/// descriptor, or [`Fat32Error::NotADirectory`] (POSIX `ENOTDIR`) if it is not a
/// directory. An empty `path` returns [`Fat32Error::NotFound`] (POSIX `ENOENT`).
pub fn vfs_accessat(dirfd: c_int, path: &str) -> Result<(), Fat32Error> {
    vfs_access_anchored(AnchoredPath::new(dirfd, String::from(path))?)
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
            let file: OpenFile = entry_arc(fd)?;
            let mut entry = file.lock();
            entry.status_flags = (entry.status_flags & file_access_mode::O_ACCMODE)
                | (arg & file_status_flags::O_NONBLOCK);
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

    let entries: Vec<filesystem::DirEntry> = dir_handle.read_entries(count)?;

    let mut result: Vec<posix_dent> = Vec::new();
    for de in &entries {
        let mut dent: posix_dent = posix_dent {
            d_ino: de.inode(),
            d_reclen: core::mem::size_of::<posix_dent>() as u16,
            d_type: if de.is_dir() {
                dirent_file_type::DT_DIR
            } else if de.is_character_device() {
                dirent_file_type::DT_CHR
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
/// Both paths must resolve to the same VFS mount. Relative paths are resolved from their respective
/// directory descriptors; `AT_FDCWD` selects the process working directory, while absolute paths
/// ignore the corresponding descriptor.
///
/// # Parameters
///
/// - `olddirfd`: Directory descriptor used when `oldpath` is relative.
/// - `oldpath`: Current path of the file or directory.
/// - `newdirfd`: Directory descriptor used when `newpath` is relative.
/// - `newpath`: New path for the file or directory.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidFd`] (POSIX `EBADF`) if a dirfd is not an open
/// descriptor, or [`Fat32Error::NotADirectory`] (POSIX `ENOTDIR`) if it is not a
/// directory. An empty path returns [`Fat32Error::NotFound`] (POSIX `ENOENT`).
/// Returns a [`Fat32Error`] if the paths are on different mounts, the old path does not exist,
/// or the new path already exists.
pub fn vfs_renameat(
    olddirfd: c_int,
    oldpath: &str,
    newdirfd: c_int,
    newpath: &str,
) -> Result<(), Fat32Error> {
    filesystem::rename(
        AnchoredPath::new(olddirfd, String::from(oldpath))?,
        AnchoredPath::new(newdirfd, String::from(newpath))?,
    )
}

/// Renames between two already-resolved paths.
pub fn vfs_rename_resolved(
    old_path: ResolvedPath,
    new_path: ResolvedPath,
) -> Result<(), Fat32Error> {
    filesystem::rename(
        AnchoredPath::new(AT_FDCWD, old_path.into_string())?,
        AnchoredPath::new(AT_FDCWD, new_path.into_string())?,
    )
}

/// Unlinks a file or removes a directory relative to a directory file descriptor through the VFS.
///
/// When `AT_REMOVEDIR` is set in `flags`, the operation behaves like `rmdir()` and removes an
/// empty directory. Otherwise, it behaves like `unlink()` and removes a regular file.
///
/// # Parameters
///
/// - `dirfd`: Directory descriptor used when `path` is relative; `AT_FDCWD` selects the process
///   working directory, while an absolute `path` ignores it.
/// - `path`: Path of the file or directory to remove.
/// - `flags`: If `AT_REMOVEDIR` (0x8) is set, remove a directory; otherwise remove a file.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidFd`] (POSIX `EBADF`) if `dirfd` is not an open
/// descriptor, or [`Fat32Error::NotADirectory`] (POSIX `ENOTDIR`) if it is not a
/// directory. An empty `path` returns [`Fat32Error::NotFound`] (POSIX `ENOENT`).
/// Returns a [`Fat32Error`] if the path does not exist, the directory is not empty (when removing
/// a directory), or the path refers to a directory but `AT_REMOVEDIR` is not set.
pub fn vfs_unlinkat(dirfd: c_int, path: &str, flags: c_int) -> Result<(), Fat32Error> {
    vfs_unlink_anchored(AnchoredPath::new(dirfd, String::from(path))?, flags)
}

/// Unlinks a file or removes a directory at an already-resolved path.
pub fn vfs_unlink_resolved(path: ResolvedPath, flags: c_int) -> Result<(), Fat32Error> {
    vfs_unlink_anchored(AnchoredPath::new(AT_FDCWD, path.into_string())?, flags)
}

/// Applies unlink semantics to a path whose descriptor provenance has already been captured.
fn vfs_unlink_anchored(path: AnchoredPath, flags: c_int) -> Result<(), Fat32Error> {
    use ::sysapi::fcntl::atflags::AT_REMOVEDIR;
    if flags & AT_REMOVEDIR != 0 {
        filesystem::rmdir(path)
    } else {
        filesystem::unlink(path)
    }
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
    // Verify that the target exists using the VFS-level stat for consistent semantics.
    vfs_access_anchored(AnchoredPath::new(dirfd, String::from(path))?)
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
    // Verify that the target exists using the VFS-level stat for consistent semantics.
    vfs_access_anchored(AnchoredPath::new(dirfd, String::from(path))?)
}

/// Sets file access and modification times through the VFS.
///
/// POSIX timestamp rules:
/// <https://pubs.opengroup.org/onlinepubs/9799919799/functions/utimensat.html>.
///
/// Resolves `UTIME_NOW` against the wall clock and honors `UTIME_OMIT`, then
/// applies the result to the backing file. FAT stores access as a date only, so
/// sub-day access precision is dropped.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor for relative path resolution.
/// - `pathname`: Path to the target file.
/// - `times`: Access (`[0]`) and modification (`[1]`) times.
/// - `flags`: Accepts `AT_SYMLINK_NOFOLLOW`; other flags are rejected.
///
/// # Errors
///
/// Returns [`Fat32Error::InvalidArgument`] if the path cannot be resolved or a
/// timestamp has an invalid nanosecond value or is outside the FAT range.
/// Returns [`Fat32Error::FileNotFound`] if the resolved path does not exist.
pub fn vfs_utimensat(
    dirfd: c_int,
    pathname: &str,
    times: &[timespec; 2],
    flags: c_int,
) -> Result<(), Fat32Error> {
    if flags & !AT_SYMLINK_NOFOLLOW != 0 {
        return Err(Fat32Error::InvalidArgument);
    }
    // FAT has no symlinks, so no-follow is equivalent to ordinary lookup.
    // POSIX requires no ownership or permission checks when both fields are omitted.
    if times.iter().all(|ts| ts.tv_nsec == UTIME_OMIT) {
        return Ok(());
    }
    let path = filesystem::VfsResolvedPath::new(AnchoredPath::new(dirfd, String::from(pathname))?)?;

    // POSIX permission check (owner / write access / privilege) is not enforced:
    // Nanvix is single-user so it always passes. Tracked for the multiuser model.

    // POSIX rejects non-special nanoseconds outside [0, 1000 million).
    let resolve = |ts: &timespec, now: i64| -> Result<Option<i64>, Fat32Error> {
        match ts.tv_nsec {
            n if n == UTIME_OMIT => Ok(None),
            n if n == UTIME_NOW => Ok(Some(now)),
            0..1_000_000_000 if (FAT_EPOCH_SECS..=MAX_FAT_TIMESTAMP).contains(&ts.tv_sec) => {
                Ok(Some(ts.tv_sec))
            },
            _ => Err(Fat32Error::InvalidArgument),
        }
    };
    let now: i64 = crate::time::wall_clock_secs();
    let atime: Option<i64> = resolve(&times[0], now)?;
    let mtime: Option<i64> = resolve(&times[1], now)?;

    filesystem::set_times(&path, atime, mtime)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
