// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//  Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    kcall1,
    kcall4,
    number::KcallNumber,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Send Message
//==================================================================================================

pub fn send(message: &Message) -> Result<(), Error> {
    let result: i64 = kcall1!(KcallNumber::Send.into(), message as *const Message as usize as u32);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to send()"))
    }
}

//==================================================================================================
// Receive Message
//==================================================================================================

pub fn recv() -> Result<Message, Error> {
    let mut message: Message = Default::default();

    let result: i64 =
        kcall1!(KcallNumber::Recv.into(), &mut message as *mut Message as usize as u32);

    if result == 0 {
        Ok(message)
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to recv()"))
    }
}

//==================================================================================================
// Rendezvous Push
//==================================================================================================

///
/// # Description
///
/// Pushes data to a destination thread using rendezvous synchronization.
///
/// # Parameters
///
/// - `destination_pid`: Process identifier of the destination.
/// - `destination_tid`: Thread identifier of the destination.
/// - `buffer`: Byte slice containing the data to send.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
/// # Errors
///
/// - [`ErrorCode::InvalidArgument`]: Invalid destination identifiers, self-push, or transfer
///   length exceeds `u32::MAX`.
///
pub fn push(
    destination_pid: ProcessIdentifier,
    destination_tid: ThreadIdentifier,
    buffer: &[u8],
) -> Result<(), Error> {
    let destination_raw: u32 = u32::try_from(destination_pid)?;
    let destination_tid_raw: u32 = u32::try_from(destination_tid)?;
    let transfer_len: u32 = buffer
        .len()
        .try_into()
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "transfer length exceeds u32::MAX"))?;

    let result: i64 = kcall4!(
        KcallNumber::Push.into(),
        destination_raw,
        destination_tid_raw,
        buffer.as_ptr() as usize as u32,
        transfer_len
    );

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to push()"))
    }
}

//==================================================================================================
// Rendezvous Pull
//==================================================================================================

///
/// # Description
///
/// Pulls data from a source thread using rendezvous synchronization.
///
/// # Parameters
///
/// - `sender_pid`: Process identifier of the expected sender.
/// - `sender_tid`: Thread identifier of the expected sender.
/// - `buffer`: Mutable byte slice where received data will be stored.
///
/// # Returns
///
/// Upon successful completion, the number of bytes actually transferred is returned. Upon failure,
/// an error is returned instead.
///
/// # Errors
///
/// - [`ErrorCode::InvalidArgument`]: Invalid sender identifiers, self-pull, or transfer length
///   exceeds `u32::MAX`.
///
pub fn pull(
    sender_pid: ProcessIdentifier,
    sender_tid: ThreadIdentifier,
    buffer: &mut [u8],
) -> Result<usize, Error> {
    let sender_raw: u32 = u32::try_from(sender_pid)?;
    let sender_tid_raw: u32 = u32::try_from(sender_tid)?;
    let transfer_len: u32 = buffer
        .len()
        .try_into()
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "transfer length exceeds u32::MAX"))?;

    let result: i64 = kcall4!(
        KcallNumber::Pull.into(),
        sender_raw,
        sender_tid_raw,
        buffer.as_mut_ptr() as usize as u32,
        transfer_len
    );

    if result >= 0 {
        usize::try_from(result).map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, "kernel returned invalid pull() length")
        })
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to pull()"))
    }
}
