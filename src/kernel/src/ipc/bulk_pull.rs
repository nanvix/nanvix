// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    sync::condvar::Condvar,
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
    ipc::{
        DataChunkHeader,
        Message,
    },
    pm::ThreadIdentifier,
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
/// linuxd to supply the requested data via the vmbus.
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
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Registers a pending bulk pull for the calling thread and puts it to sleep. The thread is woken
/// by [`complete`] when linuxd responds with the bulk data.
///
/// # Parameters
///
/// - `caller_tid`: Thread identifier of the thread requesting the pull.
///
/// # Returns
///
/// Upon successful completion (after being woken), the number of bytes transferred is returned.
/// On failure, a sleep error is returned instead.
///
#[cfg_attr(all(feature = "microvm", feature = "ring-buffer"), allow(dead_code))]
pub fn register_and_sleep(caller_tid: ThreadIdentifier) -> Result<usize, SleepError> {
    let condvar: Condvar = Condvar::new();
    let bytes_transferred: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

    // Keep a local clone of the condvar and bytes counter so they remain valid after the entry is
    // removed from the map by `complete()`.
    let condvar_clone: Condvar = condvar.clone();
    let bytes_transferred_clone: Arc<AtomicUsize> = bytes_transferred.clone();

    // SAFETY: single-core system with interrupts disabled.
    let pending: &mut BTreeMap<ThreadIdentifier, PendingBulkPull> =
        unsafe { &mut PENDING_BULK_PULLS };
    pending.insert(
        caller_tid,
        PendingBulkPull {
            condvar: condvar_clone,
            bytes_transferred: bytes_transferred_clone,
        },
    );

    trace!("bulk pull sleeping (caller_tid={caller_tid:?})");

    // Sleep on the condition variable until the completion handler wakes us up.
    // SAFETY: no global resources are held, the calling thread is not the kernel.
    match unsafe { condvar.wait(None) } {
        Ok(()) => {
            let actual: usize = bytes_transferred.load(ORDER);
            Ok(actual)
        },
        Err(error) => {
            // Remove the pending entry if the thread was interrupted.
            // SAFETY: single-core system with interrupts disabled.
            let pending: &mut BTreeMap<ThreadIdentifier, PendingBulkPull> =
                unsafe { &mut PENDING_BULK_PULLS };
            pending.remove(&caller_tid);
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
