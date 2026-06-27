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
/// Pushes data to a destination process using rendezvous synchronization.
///
/// When the destination is the kernel (linuxd), data is transferred via the vmbus scatter/gather
/// data chunk transfer path.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `caller_tid`: Identifier of the calling thread.
/// - `destination_raw`: Raw identifier of the destination process.
/// - `destination_tid_raw`: Raw identifier of the destination thread.
/// - `buffer_raw`: Raw pointer to the buffer whose contents will be sent.
/// - `transfer_len_raw`: Raw length of data to be transferred.
///
/// # Returns
///
/// On successful completion, this function returns `()` after delivering the payload to the
/// destination.  On failure, an error code is returned instead.
///
pub fn push(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    destination_raw: u32,
    destination_tid_raw: u32,
    buffer_raw: usize,
    transfer_len_raw: u32,
) -> Result<(), SleepError> {
    let destination_pid: ProcessIdentifier =
        ProcessIdentifier::try_from(destination_raw).map_err(|error| {
            let reason: &str = "invalid destination process identifier";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 destination_raw={destination_raw}, error={error:?})"
            );
            SleepError::Generic(error)
        })?;

    let destination_tid: ThreadIdentifier = ThreadIdentifier::try_from(destination_tid_raw)
        .map_err(|error| {
            let reason: &str = "invalid destination thread identifier";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 destination_tid_raw={destination_tid_raw}, error={error:?})"
            );
            SleepError::Generic(error)
        })?;

    let transfer_len: usize = usize::try_from(transfer_len_raw).map_err(|_| {
        let reason: &str = "transfer length is too large";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
             destination_pid={destination_pid:?}, destination_tid={destination_tid:?}, \
             transfer_len_raw={transfer_len_raw})"
        );
        SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason))
    })?;

    trace!(
        "tid={:?}, pid={:?}, dst_tid={:?}, dst_pid={:?}, buffer={:#x}, len={}",
        caller_tid,
        caller_pid,
        destination_tid,
        destination_pid,
        buffer_raw,
        transfer_len
    );

    // When the destination is the kernel (linuxd), use the vmbus for data chunk transfer instead
    // of the rendezvous cross-process copy. The user buffer virtual address is translated to a
    // guest physical address so the VMM can directly read the data without an intermediate kernel
    // buffer copy.
    #[cfg(feature = "stdio")]
    if destination_pid == ProcessIdentifier::KERNEL {
        if transfer_len == 0 || transfer_len > SG_BULK_MAX_BYTES {
            let reason: &str = "invalid scatter/gather bulk push length";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 buffer={buffer_raw:#x}, len={transfer_len})"
            );
            return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
        }

        trace!(
            "push(): data chunk transfer via vmbus (caller_pid={caller_pid:?}, \
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

        return crate::stdio::write_bulk(
            caller_pid,
            caller_tid,
            destination_pid,
            destination_tid,
            GuestSgBulkKind::Push,
            &segments,
            transfer_len_raw,
        )
        .map_err(SleepError::Generic);
    }

    super::rendezvous::do_push(
        caller_pid,
        caller_tid,
        destination_pid,
        destination_tid,
        buffer_raw,
        transfer_len,
    )
}
