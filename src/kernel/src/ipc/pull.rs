// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::SleepError;
#[cfg(feature = "stdio")]
use crate::{
    hal::mem::VirtualAddress,
    pm::ProcessManager,
};
#[cfg(feature = "stdio")]
use ::sys::ipc::{
    GuestSgBulkKind,
    GuestSgSegment,
    SG_BULK_MAX_BYTES,
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
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Pulls data from a sender process using rendezvous synchronization.
///
/// When the sender is the kernel (linuxd), data is transferred via the vmbus scatter/gather data
/// chunk transfer path.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `caller_tid`: Identifier of the calling thread.
/// - `sender_raw`: Raw identifier of the sender process.
/// - `sender_tid_raw`: Raw identifier of the sender thread.
/// - `buffer_raw`: Raw pointer to the buffer where data will be stored.
/// - `transfer_len_raw`: Raw length of data to be transferred.
///
/// # Returns
///
/// On successful completion, this function returns the number of bytes pulled from the sender
/// process.  On failure, an error code is returned instead.
///
pub fn pull(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    sender_raw: u32,
    sender_tid_raw: u32,
    buffer_raw: usize,
    transfer_len_raw: u32,
) -> Result<usize, SleepError> {
    // Convert sender process identifier.
    let sender_pid: ProcessIdentifier =
        ProcessIdentifier::try_from(sender_raw).map_err(|error| {
            let reason: &str = "invalid sender process identifier";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 sender_raw={sender_raw}, error={error:?})"
            );
            SleepError::Generic(error)
        })?;

    // Convert sender thread identifier.
    let sender_tid: ThreadIdentifier =
        ThreadIdentifier::try_from(sender_tid_raw).map_err(|error| {
            let reason: &str = "invalid sender thread identifier";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 sender_tid_raw={sender_tid_raw}, error={error:?})"
            );
            SleepError::Generic(error)
        })?;

    // Convert transfer length.
    let transfer_len: usize = usize::try_from(transfer_len_raw).map_err(|_| {
        let reason: &str = "transfer length is too large";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
             sender_pid={sender_pid:?}, sender_tid={sender_tid:?}, \
             transfer_len_raw={transfer_len_raw})"
        );
        SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason))
    })?;

    trace!(
        "tid={:?}, pid={:?}, src_tid={:?}, src_pid={:?}, buffer={:#x}, len={}",
        caller_tid,
        caller_pid,
        sender_tid,
        sender_pid,
        buffer_raw,
        transfer_len
    );

    // When the source is the kernel (linuxd), use the vmbus for data chunk transfer instead of the
    // rendezvous cross-process copy. The user buffer virtual address is translated to a guest
    // physical address so the VMM can write data directly into guest physical memory without an
    // intermediate kernel buffer copy. After the transfer completes the data is already in place.
    #[cfg(feature = "stdio")]
    if sender_pid == ProcessIdentifier::KERNEL {
        if transfer_len == 0 || transfer_len > SG_BULK_MAX_BYTES {
            let reason: &str = "invalid scatter/gather bulk pull length";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 buffer={buffer_raw:#x}, len={transfer_len})"
            );
            return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
        }

        trace!(
            "pull(): data chunk transfer via vmbus (caller_pid={caller_pid:?}, \
             caller_tid={caller_tid:?}, len={transfer_len})"
        );

        // SAFETY: IPC runs after the process manager is initialized, and the process manager
        // provides the synchronization needed for address translation.
        let pm: &ProcessManager = unsafe { ProcessManager::get() };
        let segments: ::alloc::vec::Vec<GuestSgSegment> = super::sg::build_user_segments(
            pm,
            caller_pid,
            VirtualAddress::from_raw_value(buffer_raw),
            transfer_len,
        )
        .map_err(SleepError::Generic)?;

        crate::stdio::write_bulk(
            caller_pid,
            caller_tid,
            sender_pid,
            sender_tid,
            GuestSgBulkKind::Pull,
            &segments,
            transfer_len_raw,
        )
        .map_err(SleepError::Generic)?;

        // Register a pending bulk pull and sleep until the completion arrives. UserVM scatters the
        // response into the guest physical segments before waking this thread.
        return super::bulk_pull::register_and_sleep(caller_tid);
    }

    super::rendezvous::do_pull(
        caller_pid,
        caller_tid,
        sender_pid,
        sender_tid,
        buffer_raw,
        transfer_len,
    )
}
