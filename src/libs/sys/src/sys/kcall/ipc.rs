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
    ipc::{
        Message,
        PullArgs,
        PushArgs,
        Timeout,
    },
    kcall1,
    number::KcallNumber,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::core::time::Duration;

//==================================================================================================
// Send Message
//==================================================================================================

#[unsafe(no_mangle)]
pub fn __kcall_send(message: &Message) -> Result<(), Error> {
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

#[unsafe(no_mangle)]
pub fn __kcall_recv() -> Result<Message, Error> {
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
/// Pushes data to a destination thread using rendezvous synchronization, blocking until the
/// destination pulls.
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
#[unsafe(no_mangle)]
pub fn __kcall_push(
    destination_pid: ProcessIdentifier,
    destination_tid: ThreadIdentifier,
    buffer: &[u8],
) -> Result<(), Error> {
    push_with_timeout(destination_pid, destination_tid, buffer, Timeout::infinite())
}

///
/// # Description
///
/// Pushes data to a destination thread using rendezvous synchronization, bounding the blocking wait
/// with a timeout.
///
/// # Parameters
///
/// - `destination_pid`: Process identifier of the destination.
/// - `destination_tid`: Thread identifier of the destination.
/// - `buffer`: Byte slice containing the data to send.
/// - `timeout`: Optional deadline. [`None`] blocks until the destination pulls; a finite duration
///   bounds the wait; a zero duration is a non-blocking probe.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
/// # Errors
///
/// - [`ErrorCode::InvalidArgument`]: Invalid destination identifiers, self-push, or transfer
///   length exceeds `u32::MAX`.
/// - [`ErrorCode::OperationTimedOut`]: The destination did not pull before the timeout elapsed
///   (or, for a zero timeout, was not already waiting).
///
#[unsafe(no_mangle)]
pub fn __kcall_push_timed(
    destination_pid: ProcessIdentifier,
    destination_tid: ThreadIdentifier,
    buffer: &[u8],
    timeout: Option<Duration>,
) -> Result<(), Error> {
    push_with_timeout(destination_pid, destination_tid, buffer, Timeout::from_duration(timeout))
}

///
/// # Description
///
/// Builds the push descriptor and issues the kernel call. Shared by [`__kcall_push`] and
/// [`__kcall_push_timed`].
///
fn push_with_timeout(
    destination_pid: ProcessIdentifier,
    destination_tid: ThreadIdentifier,
    buffer: &[u8],
    timeout: Timeout,
) -> Result<(), Error> {
    let transfer_len: u32 = buffer
        .len()
        .try_into()
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "transfer length exceeds u32::MAX"))?;

    let args: PushArgs = PushArgs {
        dst_pid: destination_pid,
        dst_tid: destination_tid,
        buffer: buffer.as_ptr() as usize as u32,
        len: transfer_len,
        timeout,
    };

    let result: i64 = kcall1!(KcallNumber::Push.into(), &args as *const PushArgs as usize as u32);

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
/// Pulls data from a source thread using rendezvous synchronization, blocking until the source
/// pushes.
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
#[unsafe(no_mangle)]
pub fn __kcall_pull(
    sender_pid: ProcessIdentifier,
    sender_tid: ThreadIdentifier,
    buffer: &mut [u8],
) -> Result<usize, Error> {
    pull_with_timeout(sender_pid, sender_tid, buffer, Timeout::infinite())
}

///
/// # Description
///
/// Pulls data from a source thread using rendezvous synchronization, bounding the blocking wait
/// with a timeout.
///
/// # Parameters
///
/// - `sender_pid`: Process identifier of the expected sender.
/// - `sender_tid`: Thread identifier of the expected sender.
/// - `buffer`: Mutable byte slice where received data will be stored.
/// - `timeout`: Optional deadline. [`None`] blocks until the source pushes; a finite duration
///   bounds the wait; a zero duration is a non-blocking probe.
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
/// - [`ErrorCode::OperationTimedOut`]: The source did not push before the timeout elapsed (or, for
///   a zero timeout, was not already waiting).
///
#[unsafe(no_mangle)]
pub fn __kcall_pull_timed(
    sender_pid: ProcessIdentifier,
    sender_tid: ThreadIdentifier,
    buffer: &mut [u8],
    timeout: Option<Duration>,
) -> Result<usize, Error> {
    pull_with_timeout(sender_pid, sender_tid, buffer, Timeout::from_duration(timeout))
}

///
/// # Description
///
/// Builds the pull descriptor and issues the kernel call. Shared by [`__kcall_pull`] and
/// [`__kcall_pull_timed`].
///
fn pull_with_timeout(
    sender_pid: ProcessIdentifier,
    sender_tid: ThreadIdentifier,
    buffer: &mut [u8],
    timeout: Timeout,
) -> Result<usize, Error> {
    let transfer_len: u32 = buffer
        .len()
        .try_into()
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "transfer length exceeds u32::MAX"))?;

    let args: PullArgs = PullArgs {
        src_pid: sender_pid,
        src_tid: sender_tid,
        buffer: buffer.as_mut_ptr() as usize as u32,
        len: transfer_len,
        timeout,
    };

    let result: i64 = kcall1!(KcallNumber::Pull.into(), &args as *const PullArgs as usize as u32);

    if result >= 0 {
        usize::try_from(result).map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, "kernel returned invalid pull() length")
        })
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to pull()"))
    }
}
