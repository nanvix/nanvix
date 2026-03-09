// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::VirtualAddress,
    pm::{
        sync::condvar::Condvar,
        ProcessManager,
        SleepError,
    },
};
use ::alloc::{
    collections::BTreeMap,
    sync::Arc,
};
use ::core::sync::atomic::{
    AtomicI32,
    AtomicUsize,
    Ordering,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
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

struct PendingFixedPull {
    condvar: Condvar,
    bytes_transferred: Arc<AtomicUsize>,
    status_code: Arc<AtomicI32>,
    caller_pid: ProcessIdentifier,
    buffer_raw: usize,
    buffer_len: usize,
}

//==================================================================================================
// Global Variables
//==================================================================================================

/// Pending fixed-buffer pull requests keyed by thread identifier.
static mut PENDING_FIXED_PULLS: BTreeMap<ThreadIdentifier, PendingFixedPull> = BTreeMap::new();

//==================================================================================================
// Public Functions
//==================================================================================================

pub fn register_and_sleep(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    buffer_raw: usize,
    buffer_len: usize,
) -> Result<usize, SleepError> {
    let condvar: Condvar = Condvar::new();
    let bytes_transferred: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let status_code: Arc<AtomicI32> = Arc::new(AtomicI32::new(0));

    let condvar_clone: Condvar = condvar.clone();
    let bytes_transferred_clone: Arc<AtomicUsize> = bytes_transferred.clone();
    let status_code_clone: Arc<AtomicI32> = status_code.clone();

    // SAFETY: single-core system with interrupts disabled.
    let pending: &mut BTreeMap<ThreadIdentifier, PendingFixedPull> =
        unsafe { &mut PENDING_FIXED_PULLS };
    pending.insert(
        caller_tid,
        PendingFixedPull {
            condvar: condvar_clone,
            bytes_transferred: bytes_transferred_clone,
            status_code: status_code_clone,
            caller_pid,
            buffer_raw,
            buffer_len,
        },
    );

    trace!(
        "fixed pull sleeping (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
         buffer_raw={buffer_raw:#x}, buffer_len={buffer_len})"
    );

    // SAFETY: no global resources are held, the calling thread is not the kernel.
    match unsafe { condvar.wait(None) } {
        Ok(()) => {
            let status: i32 = status_code.load(ORDER);
            if status != 0 {
                let error_code: ErrorCode = ErrorCode::try_from(i64::from(status)).unwrap_or_else(|_| {
                    error!(
                        "register_and_sleep(): invalid completion error code (status={status}), \
                         falling back to InvalidMessage"
                    );
                    ErrorCode::InvalidMessage
                });
                return Err(SleepError::Generic(Error::new(
                    error_code,
                    "fixed pull completion failed",
                )));
            }

            let actual: usize = bytes_transferred.load(ORDER);
            Ok(actual)
        },
        Err(error) => {
            // SAFETY: single-core system with interrupts disabled.
            let pending: &mut BTreeMap<ThreadIdentifier, PendingFixedPull> =
                unsafe { &mut PENDING_FIXED_PULLS };
            pending.remove(&caller_tid);
            Err(error)
        },
    }
}

pub fn complete(caller_tid: ThreadIdentifier, buffer_id: u32, data_len: usize) -> bool {
    // SAFETY: single-core system with interrupts disabled.
    let pending: &mut BTreeMap<ThreadIdentifier, PendingFixedPull> =
        unsafe { &mut PENDING_FIXED_PULLS };

    let Some(entry) = pending.remove(&caller_tid) else {
        warn!("complete(): no pending fixed pull found for tid={caller_tid:?}");
        return false;
    };

    let bytes_to_copy: usize = core::cmp::min(data_len, entry.buffer_len);
    let copy_result: Result<(), Error> = (|| {
        if bytes_to_copy == 0 {
            return Ok(());
        }

        let src: VirtualAddress =
            VirtualAddress::from_raw_value(crate::ring::fixed_buffer_vaddr(buffer_id)?);
        let dst: VirtualAddress = VirtualAddress::from_raw_value(entry.buffer_raw);
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        pm.vmcopy_to_user(entry.caller_pid, dst, src, bytes_to_copy)
    })();

    match copy_result {
        Ok(()) => {
            entry.bytes_transferred.store(bytes_to_copy, ORDER);
            entry.status_code.store(0, ORDER);
        },
        Err(error) => {
            error!(
                "complete(): failed to copy fixed-buffer payload to user buffer \
                 (caller_tid={caller_tid:?}, caller_pid={:?}, buffer_id={buffer_id}, \
                 buffer_raw={:#x}, data_len={data_len}, error={error:?})",
                entry.caller_pid,
                entry.buffer_raw
            );
            entry.bytes_transferred.store(0, ORDER);
            entry.status_code.store(i32::from(error.code), ORDER);
        },
    }

    // SAFETY: the calling process does not hold a reference to the process manager.
    if let Err(error) = unsafe { entry.condvar.notify_thread(caller_tid) } {
        error!(
            "complete(): failed to wake up sleeping fixed pull thread (tid={caller_tid:?}, \
             error={error:?})"
        );
    }

    trace!(
        "fixed pull completed (caller_tid={caller_tid:?}, buffer_id={buffer_id}, \
         bytes_transferred={bytes_to_copy})"
    );

    true
}
