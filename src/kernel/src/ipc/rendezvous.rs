// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::VirtualAddress,
    pm::{
        clock,
        InterruptReason,
        ProcessManager,
        SleepError,
    },
};
use ::alloc::{
    sync::Arc,
    vec::Vec,
};
use ::core::{
    cell::UnsafeCell,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
    time::Duration,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Timeout,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    time::SystemTime,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Ordering used for all atomic operations. Relaxed is safe because Nanvix is a single-core system
/// and the kernel runs with interrupts disabled.
const ORDER: Ordering = Ordering::Relaxed;

//==================================================================================================
// Rendezvous Timeout
//==================================================================================================

///
/// # Description
///
/// Resolved timeout for a rendezvous operation, derived from the caller-supplied [`Timeout`]
/// descriptor. It expresses the three-point spectrum — poll, bounded wait, block — that bounds a
/// blocking rendezvous.
///
#[derive(Debug, Clone, Copy)]
pub enum RendezvousTimeout {
    /// Block until the counterpart arrives.
    Infinite,
    /// Return immediately if no counterpart is ready, without registering a pending entry or
    /// sleeping.
    NonBlocking,
    /// Block until the given absolute deadline, then report a timeout.
    Deadline(SystemTime),
}

impl RendezvousTimeout {
    ///
    /// # Description
    ///
    /// Resolves a wire [`Timeout`] descriptor into a [`RendezvousTimeout`], computing the absolute
    /// deadline of a finite, non-zero timeout from the monotonic clock.
    ///
    /// # Parameters
    ///
    /// - `timeout`: The timeout descriptor supplied by the caller.
    ///
    /// # Returns
    ///
    /// On success, the resolved timeout. On failure (a deadline that overflows the system clock),
    /// an error is returned instead.
    ///
    pub fn resolve(timeout: Timeout) -> Result<Self, Error> {
        match timeout.as_finite()? {
            // Infinite: block until the counterpart arrives.
            None => Ok(Self::Infinite),
            // Finite, zero duration: non-blocking probe.
            Some((0, 0)) => Ok(Self::NonBlocking),
            // Finite, non-zero duration: compute an absolute deadline from the monotonic clock.
            Some((secs, nanos)) => {
                let duration: Duration = Duration::new(secs as u64, nanos);
                let now: SystemTime = clock::now();
                match now.checked_add_duration(&duration) {
                    Some(deadline) => Ok(Self::Deadline(deadline)),
                    None => Err(Error::new(
                        ErrorCode::InvalidArgument,
                        "rendezvous timeout overflows the system clock",
                    )),
                }
            },
        }
    }

    ///
    /// # Description
    ///
    /// Returns the absolute deadline to sleep until: [`None`] for an infinite wait, or [`Some`]
    /// deadline for a bounded or non-blocking wait. A non-blocking timeout yields the current time,
    /// so the wait expires immediately.
    ///
    /// This is used by the guest-to-host bulk pull path, which — unlike the intra-guest rendezvous
    /// — cannot short-circuit a non-blocking request before emitting it, and so treats it as an
    /// immediate deadline.
    ///
    /// # Returns
    ///
    /// The optional sleep deadline.
    ///
    pub fn deadline(&self) -> Option<SystemTime> {
        match self {
            Self::Infinite => None,
            Self::NonBlocking => Some(clock::now()),
            Self::Deadline(deadline) => Some(*deadline),
        }
    }
}

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A pending push request: a thread that is sleeping while waiting for a matching pull.
///
struct PendingPush {
    /// Process identifier of the pushing thread.
    pid: ProcessIdentifier,
    /// Thread identifier of the pushing thread.
    tid: ThreadIdentifier,
    /// Destination process identifier.
    dst_pid: ProcessIdentifier,
    /// Destination thread identifier.
    dst_tid: ThreadIdentifier,
    /// Buffer address in the pushing thread's user space.
    buffer: usize,
    /// Maximum transfer length.
    transfer_len: usize,
}

///
/// # Description
///
/// A pending pull request: a thread that is sleeping while waiting for a matching push.
///
struct PendingPull {
    /// Process identifier of the pulling thread.
    pid: ProcessIdentifier,
    /// Thread identifier of the pulling thread.
    tid: ThreadIdentifier,
    /// Source process identifier (expected sender).
    src_pid: ProcessIdentifier,
    /// Source thread identifier (expected sender).
    src_tid: ThreadIdentifier,
    /// Buffer address in the pulling thread's user space.
    buffer: usize,
    /// Maximum transfer length.
    transfer_len: usize,
    /// Actual bytes transferred, set by the matching push before waking.
    bytes_transferred: Arc<AtomicUsize>,
}

//==================================================================================================
// Global Variables
//==================================================================================================

///
/// # Description
///
/// Wrapper that centralises all `unsafe` access to the pending rendezvous lists behind a single
/// abstraction. On Nanvix's single-core kernel (interrupts disabled during kernel calls) there is
/// no concurrent access, so interior mutability via [`UnsafeCell`] is sound.
///
/// # Safety
///
/// The contained [`UnsafeCell`] must only be accessed while the kernel is running on a single core
/// with interrupts disabled.
///
struct PendingLists {
    /// Pending push requests (threads sleeping while waiting for a matching pull).
    pushes: UnsafeCell<Vec<PendingPush>>,
    /// Pending pull requests (threads sleeping while waiting for a matching push).
    pulls: UnsafeCell<Vec<PendingPull>>,
}

// SAFETY: Nanvix is a single-core system and the kernel runs with interrupts disabled.
unsafe impl Sync for PendingLists {}

impl PendingLists {
    /// Creates a new, empty set of pending lists.
    const fn new() -> Self {
        Self {
            pushes: UnsafeCell::new(Vec::new()),
            pulls: UnsafeCell::new(Vec::new()),
        }
    }

    /// Returns a mutable reference to the pending push list.
    ///
    /// # Safety
    ///
    /// The caller must ensure exclusive access (single-core, interrupts disabled).
    #[allow(clippy::mut_from_ref)]
    unsafe fn pushes(&self) -> &mut Vec<PendingPush> {
        &mut *self.pushes.get()
    }

    /// Returns a mutable reference to the pending pull list.
    ///
    /// # Safety
    ///
    /// The caller must ensure exclusive access (single-core, interrupts disabled).
    #[allow(clippy::mut_from_ref)]
    unsafe fn pulls(&self) -> &mut Vec<PendingPull> {
        &mut *self.pulls.get()
    }
}

/// Global pending rendezvous lists.
static PENDING: PendingLists = PendingLists::new();

//==================================================================================================
// Private Functions
//==================================================================================================

///
/// # Description
///
/// Copies data directly between user spaces of two processes, without an intermediate kernel
/// buffer.
///
/// # Parameters
///
/// - `pm`: Reference to the process manager.
/// - `src_pid`: Process whose user space contains the source buffer.
/// - `src_buffer`: Source buffer address in `src_pid`'s user space.
/// - `dst_pid`: Process whose user space contains the destination buffer.
/// - `dst_buffer`: Destination buffer address in `dst_pid`'s user space.
/// - `len`: Number of bytes to copy.
///
/// # Returns
///
/// Upon successful completion, empty is returned. On failure, an error is returned instead.
///
fn cross_process_copy(
    pm: &mut ProcessManager,
    src_pid: ProcessIdentifier,
    src_buffer: usize,
    dst_pid: ProcessIdentifier,
    dst_buffer: usize,
    len: usize,
) -> Result<(), Error> {
    let src: VirtualAddress = VirtualAddress::from_raw_value(src_buffer);
    let dst: VirtualAddress = VirtualAddress::from_raw_value(dst_buffer);
    pm.vmcopy_user_to_user(src_pid, src, dst_pid, dst, len)
}

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Performs a rendezvous push: sends data to a destination thread. If the destination is already
/// waiting to pull data from the caller, the transfer completes immediately. Otherwise, the caller
/// is put to sleep until a matching pull arrives.
///
/// # Parameters
///
/// - `caller_pid`: Process identifier of the pushing thread.
/// - `caller_tid`: Thread identifier of the pushing thread.
/// - `dst_pid`: Destination process identifier.
/// - `dst_tid`: Destination thread identifier.
/// - `buffer`: Buffer address in the caller's user space.
/// - `transfer_len`: Number of bytes to transfer.
/// - `timeout`: Bounds the blocking wait when no matching pull is present.
///
/// # Returns
///
/// Upon successful completion, empty is returned. On failure, a sleep error is returned instead.
///
pub fn do_push(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    dst_pid: ProcessIdentifier,
    dst_tid: ThreadIdentifier,
    buffer: usize,
    transfer_len: usize,
    timeout: RendezvousTimeout,
) -> Result<(), SleepError> {
    // Prevent self-deadlock: a thread cannot push to itself.
    if caller_tid == dst_tid {
        let reason: &str = "cannot push data to self";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, dst_pid={dst_pid:?}, \
             dst_tid={dst_tid:?})"
        );
        return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
    }

    // SAFETY: single-core system with interrupts disabled.
    let pending_pulls: &mut Vec<PendingPull> = unsafe { PENDING.pulls() };

    // Search for a matching pending pull request. A pull matches if the puller is the destination
    // and the puller is waiting for data from the caller.
    //
    // Thread identifiers are globally unique and never recycled, so the (src_tid, dst_tid) pair
    // identifies the rendezvous unambiguously and the process identifiers are redundant for
    // matching. Keying on the thread identifiers alone keeps the rendezvous working when a
    // counterpart's process identifier cannot be derived from its thread identifier — e.g. a
    // thread reached via fork()+execv(), whose main-thread tid no longer equals its pid, so a peer
    // such as vfsd that derives the destination pid by casting the tid supplies a mismatched
    // `dst_pid`. The authoritative process identifiers stored in the matched entry are still used
    // for the cross-process copy below.
    let match_idx: Option<usize> = pending_pulls
        .iter()
        .position(|pull| pull.src_tid == caller_tid && pull.tid == dst_tid);

    if let Some(idx) = match_idx {
        // Found a matching pull: the destination is already waiting for our data.
        // NOTE: `swap_remove` is O(1) but does not preserve insertion order. This is acceptable
        // because the rendezvous protocol is strictly 1:1: at most one pending push (or pull)
        // can exist for a given (caller_tid, dst_tid) pair at any time.
        // Multiple concurrent pushes from the same thread to the same destination are not
        // supported and would require FIFO ordering if ever needed.
        let pull_req: PendingPull = pending_pulls.swap_remove(idx);

        // Determine actual transfer length (minimum of what both sides can handle).
        let actual_len: usize = usize::min(transfer_len, pull_req.transfer_len);

        if actual_len > 0 {
            // Perform cross-process copy: caller's buffer -> puller's buffer.
            // SAFETY: the process manager is initialized and access is synchronized.
            let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
            if let Err(copy_error) = cross_process_copy(
                pm,
                caller_pid,
                buffer,
                pull_req.pid,
                pull_req.buffer,
                actual_len,
            ) {
                // Copy failed. Wake up the sleeping puller to prevent deadlock.
                warn!(
                    "cross-process copy failed, waking sleeping puller (puller_tid={:?}, \
                     error={copy_error:?})",
                    pull_req.tid
                );
                if let Err(wakeup_error) = unsafe { ProcessManager::wakeup(pull_req.tid) } {
                    error!(
                        "failed to wake up sleeping puller (tid={:?}, error={wakeup_error:?})",
                        pull_req.tid
                    );
                }
                return Err(SleepError::Generic(copy_error));
            }
        }

        // Store actual bytes transferred for the puller to read.
        pull_req.bytes_transferred.store(actual_len, ORDER);

        // Wake up the pulling thread.
        // SAFETY: the calling process does not hold a reference to the process manager.
        unsafe { ProcessManager::wakeup(pull_req.tid) }.map_err(SleepError::Generic)?;

        trace!(
            "push completed immediately (caller_tid={caller_tid:?}, dst_tid={dst_tid:?}, \
             bytes={actual_len})"
        );

        Ok(())
    } else {
        // No matching pull found.
        //
        // A non-blocking probe must not register a pending entry or sleep: report a timeout
        // immediately so the caller can decide what to do (the syscall layer maps this to EAGAIN
        // for non-blocking descriptors).
        if let RendezvousTimeout::NonBlocking = timeout {
            trace!(
                "push would block, non-blocking probe reports timeout (caller_tid={caller_tid:?}, \
                 dst_tid={dst_tid:?})"
            );
            return Err(SleepError::Interrupted(InterruptReason::TimedOut));
        }

        // Register this push and sleep until a matching pull arrives or the deadline expires.
        // SAFETY: single-core system with interrupts disabled.
        let pending_pushes: &mut Vec<PendingPush> = unsafe { PENDING.pushes() };
        pending_pushes.push(PendingPush {
            pid: caller_pid,
            tid: caller_tid,
            dst_pid,
            dst_tid,
            buffer,
            transfer_len,
        });

        trace!(
            "push sleeping (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
             dst_pid={dst_pid:?}, dst_tid={dst_tid:?})"
        );

        // A finite timeout sleeps until its deadline; an infinite timeout sleeps indefinitely.
        let alarm: Option<SystemTime> = match timeout {
            RendezvousTimeout::Deadline(deadline) => Some(deadline),
            _ => None,
        };

        // Sleep until a matching pull arrives and performs the copy, the deadline expires, or the
        // thread is interrupted.
        // SAFETY: no global resources are held, the calling thread is not the kernel.
        match unsafe { ProcessManager::sleep(alarm) } {
            Ok(()) => Ok(()),
            Err(error) => {
                // Remove the pending push if the thread was interrupted or timed out.
                // NOTE: if a matching pull arrived between `sleep()` returning and this
                // `retain()` call, the counterpart already consumed the entry via
                // `swap_remove` and completed the transfer. In that case `retain()` is a
                // harmless no-op. On a single-core system with interrupts disabled this
                // window cannot actually occur, but the logic is correct regardless.
                // SAFETY: single-core system with interrupts disabled.
                let pending_pushes: &mut Vec<PendingPush> = unsafe { PENDING.pushes() };
                pending_pushes.retain(|push| push.pid != caller_pid || push.tid != caller_tid);
                Err(error)
            },
        }
    }
}

///
/// # Description
///
/// Performs a rendezvous pull: receives data from a source thread. If the source is already
/// waiting to push data to the caller, the transfer completes immediately. Otherwise, the caller
/// is put to sleep until a matching push arrives.
///
/// # Parameters
///
/// - `caller_pid`: Process identifier of the pulling thread.
/// - `caller_tid`: Thread identifier of the pulling thread.
/// - `src_pid`: Source process identifier (expected sender).
/// - `src_tid`: Source thread identifier (expected sender).
/// - `buffer`: Buffer address in the caller's user space.
/// - `transfer_len`: Maximum number of bytes to receive.
/// - `timeout`: Bounds the blocking wait when no matching push is present.
///
/// # Returns
///
/// On successful completion, the number of bytes actually transferred is returned. On failure,
/// a sleep error is returned instead.
///
pub fn do_pull(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    src_pid: ProcessIdentifier,
    src_tid: ThreadIdentifier,
    buffer: usize,
    transfer_len: usize,
    timeout: RendezvousTimeout,
) -> Result<usize, SleepError> {
    // Prevent self-deadlock: a thread cannot pull from itself.
    if caller_tid == src_tid {
        let reason: &str = "cannot pull data from self";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, src_pid={src_pid:?}, \
             src_tid={src_tid:?})"
        );
        return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
    }

    // SAFETY: single-core system with interrupts disabled.
    let pending_pushes: &mut Vec<PendingPush> = unsafe { PENDING.pushes() };

    // Search for a matching pending push request. A push matches if the pusher is the expected
    // source and the pusher is targeting the caller.
    //
    // Thread identifiers are globally unique and never recycled, so the (src_tid, dst_tid) pair
    // identifies the rendezvous unambiguously and the process identifiers are redundant for
    // matching. Keying on the thread identifiers alone keeps the rendezvous working when a
    // counterpart's process identifier cannot be derived from its thread identifier — e.g. a
    // thread reached via fork()+execv(), whose main-thread tid no longer equals its pid, so a peer
    // such as vfsd that derives the source pid by casting the tid supplies a mismatched `src_pid`.
    // The authoritative process identifiers stored in the matched entry are still used for the
    // cross-process copy below.
    let match_idx: Option<usize> = pending_pushes
        .iter()
        .position(|push| push.tid == src_tid && push.dst_tid == caller_tid);

    if let Some(idx) = match_idx {
        // Found a matching push: the source is already waiting to send data.
        // NOTE: `swap_remove` is O(1) but does not preserve insertion order. This is acceptable
        // because the rendezvous protocol is strictly 1:1: at most one pending push (or pull)
        // can exist for a given (caller_tid, dst_tid) pair at any time.
        // Multiple concurrent pulls from the same thread to the same source are not
        // supported and would require FIFO ordering if ever needed.
        let push_req: PendingPush = pending_pushes.swap_remove(idx);

        // Determine actual transfer length (minimum of what both sides can handle).
        let actual_len: usize = usize::min(transfer_len, push_req.transfer_len);

        if actual_len > 0 {
            // Perform cross-process copy: pusher's buffer -> caller's buffer.
            // SAFETY: the process manager is initialized and access is synchronized.
            let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
            if let Err(copy_error) = cross_process_copy(
                pm,
                push_req.pid,
                push_req.buffer,
                caller_pid,
                buffer,
                actual_len,
            ) {
                // Copy failed. Wake up the sleeping pusher to prevent deadlock.
                warn!(
                    "cross-process copy failed, waking sleeping pusher (pusher_tid={:?}, \
                     error={copy_error:?})",
                    push_req.tid
                );
                if let Err(wakeup_error) = unsafe { ProcessManager::wakeup(push_req.tid) } {
                    error!(
                        "failed to wake up sleeping pusher (tid={:?}, error={wakeup_error:?})",
                        push_req.tid
                    );
                }
                return Err(SleepError::Generic(copy_error));
            }
        }

        // Wake up the pushing thread.
        // SAFETY: the calling process does not hold a reference to the process manager.
        unsafe { ProcessManager::wakeup(push_req.tid) }.map_err(SleepError::Generic)?;

        trace!(
            "pull completed immediately (caller_tid={caller_tid:?}, src_tid={src_tid:?}, \
             bytes={actual_len})"
        );

        Ok(actual_len)
    } else {
        // No matching push found.
        //
        // A non-blocking probe must not register a pending entry or sleep: report a timeout
        // immediately so the caller can decide what to do (the syscall layer maps this to EAGAIN
        // for non-blocking descriptors).
        if let RendezvousTimeout::NonBlocking = timeout {
            trace!(
                "pull would block, non-blocking probe reports timeout (caller_tid={caller_tid:?}, \
                 src_tid={src_tid:?})"
            );
            return Err(SleepError::Interrupted(InterruptReason::TimedOut));
        }

        // Register this pull and sleep until a matching push arrives or the deadline expires.
        let bytes_transferred: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let bytes_transferred_clone: Arc<AtomicUsize> = bytes_transferred.clone();

        // SAFETY: single-core system with interrupts disabled.
        let pending_pulls: &mut Vec<PendingPull> = unsafe { PENDING.pulls() };
        pending_pulls.push(PendingPull {
            pid: caller_pid,
            tid: caller_tid,
            src_pid,
            src_tid,
            buffer,
            transfer_len,
            bytes_transferred: bytes_transferred_clone,
        });

        trace!(
            "pull sleeping (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
             src_pid={src_pid:?}, src_tid={src_tid:?})"
        );

        // A finite timeout sleeps until its deadline; an infinite timeout sleeps indefinitely.
        let alarm: Option<SystemTime> = match timeout {
            RendezvousTimeout::Deadline(deadline) => Some(deadline),
            _ => None,
        };

        // Sleep until a matching push arrives and performs the copy, the deadline expires, or the
        // thread is interrupted.
        // SAFETY: no global resources are held, the calling thread is not the kernel.
        match unsafe { ProcessManager::sleep(alarm) } {
            Ok(()) => {
                let actual: usize = bytes_transferred.load(ORDER);
                Ok(actual)
            },
            Err(error) => {
                // Remove the pending pull if the thread was interrupted or timed out.
                // NOTE: if a matching push arrived between `sleep()` returning and this
                // `retain()` call, the counterpart already consumed the entry via
                // `swap_remove` and completed the transfer. In that case `retain()` is a
                // harmless no-op. On a single-core system with interrupts disabled this
                // window cannot actually occur, but the logic is correct regardless.
                // SAFETY: single-core system with interrupts disabled.
                let pending_pulls: &mut Vec<PendingPull> = unsafe { PENDING.pulls() };
                pending_pulls.retain(|pull| pull.pid != caller_pid || pull.tid != caller_tid);
                Err(error)
            },
        }
    }
}

///
/// # Description
///
/// Cleans up all pending rendezvous entries associated with a terminated process and returns the
/// thread identifiers of counterpart threads that must be woken up by the caller.
///
/// For each entry owned by the terminated process, it is simply removed (the owning thread will
/// be killed by the normal process termination flow). For each entry belonging to a **counterpart**
/// thread (i.e., a thread in another process that is sleeping while waiting for data from, or
/// targeting, the terminated process), the entry is removed and the thread identifier is collected
/// so the caller can wake it up. This prevents counterpart threads from blocking forever.
///
/// This follows the same approach used for semaphores and locks during process termination:
/// sleeping threads are woken rather than silently orphaned.
///
/// # Parameters
///
/// - `pid`: Process identifier of the terminated process.
///
/// # Returns
///
/// A vector of thread identifiers belonging to counterpart threads that must be woken up by the
/// caller.
///
/// # Safety
///
/// This function accesses global mutable state. It is safe to call on a single-core system with
/// interrupts disabled.
///
#[must_use]
pub unsafe fn cleanup_process(pid: ProcessIdentifier) -> Vec<ThreadIdentifier> {
    let mut threads_to_wake: Vec<ThreadIdentifier> = Vec::new();

    // --- Pending pushes ---
    // SAFETY: single-core system with interrupts disabled.
    let pending_pushes: &mut Vec<PendingPush> = unsafe { PENDING.pushes() };
    let mut removed_pushes: usize = 0;

    pending_pushes.retain(|push| {
        if push.pid == pid {
            // Entry owned by the terminated process: remove it. The thread will be killed by the
            // normal process termination flow when it wakes from sleep.
            removed_pushes += 1;
            false
        } else if push.dst_pid == pid {
            // Counterpart entry: a thread in another process is sleeping while waiting for the
            // terminated process to pull data. Collect the TID so the caller wakes it up.
            threads_to_wake.push(push.tid);
            removed_pushes += 1;
            false
        } else {
            true
        }
    });

    // --- Pending pulls ---
    // SAFETY: single-core system with interrupts disabled.
    let pending_pulls: &mut Vec<PendingPull> = unsafe { PENDING.pulls() };
    let mut removed_pulls: usize = 0;

    pending_pulls.retain(|pull| {
        if pull.pid == pid {
            // Entry owned by the terminated process: just remove it.
            removed_pulls += 1;
            false
        } else if pull.src_pid == pid {
            // Counterpart entry: a thread in another process is sleeping while waiting for the
            // terminated process to push data. Collect the TID so the caller wakes it up.
            threads_to_wake.push(pull.tid);
            removed_pulls += 1;
            false
        } else {
            true
        }
    });

    if removed_pushes > 0 || removed_pulls > 0 {
        trace!(
            "cleanup_process(): cleaned up rendezvous entries (pid={pid:?}, \
             pushes_removed={removed_pushes}, pulls_removed={removed_pulls}, threads_to_wake={})",
            threads_to_wake.len(),
        );
    }

    threads_to_wake
}
