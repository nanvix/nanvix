// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    sync::condvar::Condvar,
    InterruptReason,
    SleepError,
};
use ::alloc::{
    collections::BTreeMap,
    sync::Arc,
};
use ::core::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        DataChunkHeader,
        Message,
    },
    pm::ThreadIdentifier,
    time::SystemTime,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Ordering used for all atomic operations. Relaxed is safe because Nanvix is a single-core system
/// and the kernel runs with interrupts disabled.
const ORDER: Ordering = Ordering::Relaxed;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A pending bulk pull request: a thread that is sleeping on a condition variable while waiting for
/// the host I/O backend to supply the requested data via the vmbus.
///
struct PendingBulkPull {
    /// Condition variable on which the pulling thread is sleeping.
    condvar: Condvar,
    /// Actual bytes transferred, written by the completion handler before waking.
    bytes_transferred: Arc<AtomicUsize>,
}

//==================================================================================================
// Global Variables
//==================================================================================================

/// Pending bulk pull requests keyed by thread identifier.
static mut PENDING_BULK_PULLS: BTreeMap<ThreadIdentifier, PendingBulkPull> = BTreeMap::new();

//==================================================================================================
// Private Functions
//==================================================================================================

///
/// # Description
///
/// Tests whether a sleep error was caused by a timeout rather than by a real abort condition.
///
/// # Parameters
///
/// - `error`: Sleep error to inspect.
///
/// # Returns
///
/// This function returns `true` if the wait timed out, and `false` otherwise.
///
fn is_timeout(error: &SleepError) -> bool {
    match error {
        SleepError::Interrupted(InterruptReason::TimedOut) => true,
        SleepError::Generic(error) if error.code == ErrorCode::OperationTimedOut => true,
        _ => false,
    }
}

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Registers a pending bulk pull for the calling thread and puts it to sleep. The thread is woken
/// by [`complete`] when the host responds with the bulk data.
///
/// # Parameters
///
/// - `caller_tid`: Thread identifier of the thread requesting the pull.
/// - `alarm`: Optional absolute deadline that bounds the wait. [`None`] blocks until the host
///   responds; [`Some`] reports [`ErrorCode::OperationTimedOut`](sys::error::ErrorCode) once the
///   deadline elapses, so a slow or wedged host cannot block the guest thread forever.
///
/// # Returns
///
/// Upon successful completion (after being woken), the number of bytes transferred is returned.
/// On failure, a sleep error is returned instead.
///
/// # Errors
///
/// Fails with [`ErrorCode::ResourceBusy`] if a previous bulk pull from the same thread timed out
/// while its host completion was still in flight and that completion has not yet drained. The new
/// request is refused until then, because completions are correlated back to the thread by
/// identifier alone and overwriting the pending entry would let the stale completion be
/// mis-delivered to this request.
///
/// # Caveats
///
/// A finite `alarm` bounds only the *guest* wait; it does **not** cancel the transfer already in
/// flight on the host. UserVM has been handed the caller's buffer as guest physical segments and
/// will scatter the host reply into them whenever it eventually arrives, with no liveness check. A
/// finite deadline is therefore safe only against a host that never responds: if the host is merely
/// slow and replies *after* the deadline, it can write into buffer pages the caller may have since
/// freed or reused. Because of this, finite timeouts on the bulk (host) pull path must not be
/// exposed to callers until the guest-to-host cancel protocol is in place; all current callers use
/// the infinite variant. Tracked in issue #2908
/// (<https://github.com/nanvix/nanvix/issues/2908>).
///
pub fn register_and_sleep(
    caller_tid: ThreadIdentifier,
    alarm: Option<SystemTime>,
) -> Result<usize, SleepError> {
    let condvar: Condvar = Condvar::new();
    let bytes_transferred: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

    // Keep a local clone of the condvar and bytes counter so they remain valid after the entry is
    // removed from the map by `complete()`.
    let condvar_clone: Condvar = condvar.clone();
    let bytes_transferred_clone: Arc<AtomicUsize> = bytes_transferred.clone();

    // SAFETY: single-core system with interrupts disabled.
    let pending: &mut BTreeMap<ThreadIdentifier, PendingBulkPull> =
        unsafe { &mut PENDING_BULK_PULLS };

    // Refuse to overwrite an entry left behind by a previous bulk pull from this thread. Such an
    // entry lingers only when that earlier request timed out while its host completion was still in
    // flight: the pending map is keyed solely by thread identifier, so the late completion is
    // correlated back to this thread by TID alone. Because the calling thread is synchronous, an
    // existing entry can only be such a leftover, and overwriting it would let the stale completion
    // wake and complete this new request with the earlier request's data. Fail fast until the
    // in-flight completion drains the entry.
    if pending.contains_key(&caller_tid) {
        let reason: &str = "a previous bulk pull from this thread is still in flight";
        warn!("register_and_sleep(): {reason} (caller_tid={caller_tid:?})");
        return Err(SleepError::Generic(Error::new(ErrorCode::ResourceBusy, reason)));
    }

    pending.insert(
        caller_tid,
        PendingBulkPull {
            condvar: condvar_clone,
            bytes_transferred: bytes_transferred_clone,
        },
    );

    trace!("bulk pull sleeping (caller_tid={caller_tid:?})");

    // Sleep on the condition variable until the completion handler wakes us up or the deadline
    // expires.
    // SAFETY: no global resources are held, the calling thread is not the kernel.
    match unsafe { condvar.wait(alarm) } {
        Ok(()) => {
            let actual: usize = bytes_transferred.load(ORDER);
            Ok(actual)
        },
        Err(error) => {
            if !is_timeout(&error) {
                // Remove the pending entry only if the wait was truly aborted. On timeout the
                // host request is still in flight, so a late completion must be allowed to consume
                // the entry rather than being dropped as stale. The entry is left keyed by this
                // thread; a subsequent bulk pull from the same thread is refused (see the guard
                // above) until that late completion drains it, so the completion can never be
                // mis-delivered to a different request.
                // SAFETY: single-core system with interrupts disabled.
                let pending: &mut BTreeMap<ThreadIdentifier, PendingBulkPull> =
                    unsafe { &mut PENDING_BULK_PULLS };
                pending.remove(&caller_tid);
            }
            Err(error)
        },
    }
}

///
/// # Description
///
/// Completes a pending bulk pull by extracting the source thread identifier and transfer length
/// from a [`PullResponse`] message, storing the result, and waking the sleeping thread.
///
/// The message payload must contain a serialized [`DataChunkHeader`] starting at byte offset 0.
/// The `source_tid` field identifies the thread to wake, and `data_len` holds the actual number of
/// bytes transferred.
///
/// # Parameters
///
/// - `message`: The [`PullResponse`] message whose payload encodes the completion info.
///
/// # Returns
///
/// `true` if a matching pending pull was found and the thread was woken, `false` otherwise.
///
pub fn complete(message: &Message) -> bool {
    // Extract the DataChunkHeader from the message payload.
    let mut header_bytes: [u8; DataChunkHeader::SIZE] = [0u8; DataChunkHeader::SIZE];
    header_bytes.copy_from_slice(&message.payload[..DataChunkHeader::SIZE]);
    let header: DataChunkHeader = match DataChunkHeader::try_from_bytes(header_bytes) {
        Ok(h) => h,
        Err(e) => {
            error!("complete(): failed to parse data chunk transfer header from payload: {e:?}");
            return false;
        },
    };

    let caller_tid: ThreadIdentifier = header.source_tid();
    let bytes_transferred: usize = header.data_len() as usize;

    // SAFETY: single-core system with interrupts disabled.
    let pending: &mut BTreeMap<ThreadIdentifier, PendingBulkPull> =
        unsafe { &mut PENDING_BULK_PULLS };

    // Find and remove the matching pending pull by thread identifier.
    if let Some(entry) = pending.remove(&caller_tid) {
        // Store the actual bytes transferred so the woken thread can read it.
        entry.bytes_transferred.store(bytes_transferred, ORDER);

        // Wake up the sleeping thread via the condition variable.
        // SAFETY: the calling process does not hold a reference to the process manager.
        if let Err(e) = unsafe { entry.condvar.notify_thread(caller_tid) } {
            error!(
                "complete(): failed to wake up sleeping pull thread (tid={caller_tid:?}, \
                 error={e:?})"
            );
        }

        trace!(
            "bulk pull completed (caller_tid={caller_tid:?}, \
             bytes_transferred={bytes_transferred})"
        );

        true
    } else {
        warn!("complete(): no pending bulk pull found for tid={caller_tid:?}");
        false
    }
}
