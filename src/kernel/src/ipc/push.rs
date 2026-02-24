// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::SleepError;
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
    let buffer_ptr: *const u8 = buffer_raw as *const u8;

    trace!(
        "tid={:?}, pid={:?}, dst_tid={:?}, dst_pid={:?}, buffer={:?}, len={}",
        caller_tid,
        caller_pid,
        destination_tid,
        destination_pid,
        buffer_ptr,
        transfer_len
    );

    super::rendezvous::do_push(
        caller_pid,
        caller_tid,
        destination_pid,
        destination_tid,
        buffer_raw,
        transfer_len,
    )
}
