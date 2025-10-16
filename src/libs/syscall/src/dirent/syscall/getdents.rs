// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::{
        message::{
            GetDirectoryEntriesRequest,
            GetDirectoryEntriesResponse,
        },
        posix_dent,
    },
    message::{
        LinuxDaemonLongMessage,
        LinuxDaemonMessagePart,
        MessagePartitioner,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::vec::Vec;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::ffi::c_int;
use ::syslog::{
    error,
    trace,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets directory entries.
///
/// # Parameters
///
/// - `fd`: File descriptor of the directory.
/// - `count`: Minimum number of directory entries to get.
///
/// # Returns
///
/// On successful completion, a list with the directory entries, with at least `count` elements, is
/// returned. On failure, an error code is returned instead.
///
pub fn posix_getdents(fd: c_int, count: usize) -> Result<Vec<posix_dent>, Error> {
    trace!("posix_getdents(): fd={}, count={:?}", fd, count);

    // Capacity of message assembler.
    const MESSAGE_ASSEMBLER_CAPACITY: usize =
        GetDirectoryEntriesResponse::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request message.
    let request: Message = GetDirectoryEntriesRequest::build(tid, fd, count).map_err(|error| {
        let reason: &str = "failed to build message";
        error!("posix_getdents(): {reason} (error={:?})", error);
        Error::new(error.code, reason)
    })?;

    // Send request message.
    ::sys::kcall::ipc::send(&request).map_err(|error| {
        let reason: &str = "failed to send message";
        error!("posix_getdents(): {reason} (error={:?})", error);
        Error::new(error.code, reason)
    })?;

    // Create message assembler.
    let mut assembler: LinuxDaemonLongMessage =
        LinuxDaemonLongMessage::new(MESSAGE_ASSEMBLER_CAPACITY).map_err(|error| {
            let reason: &str = "failed to create message assembler";
            error!("posix_getdents(): {reason} (error={:?})", error);
            Error::new(error.code, reason)
        })?;

    loop {
        // Wait for response message.
        let response: Message = ::sys::kcall::ipc::recv().map_err(|error| {
            let reason: &str = "failed to receive message";
            error!("posix_getdents(): {reason} (error={:?})", error);
            Error::new(error.code, reason)
        })?;

        // Check whether system call succeeded or not
        if response.status != 0 {
            // System call failed, parse error code and return it.
            match ErrorCode::try_from(response.status) {
                Ok(error_code) => {
                    let reason: &str = "system call failed";
                    error!("posix_getdents(): {reason} (error_code={error_code:?})");
                    break Err(Error::new(error_code, reason));
                },
                Err(_) => {
                    let reason: &str = "failed to parse error code";
                    error!("posix_getdents(): {reason} (response.status={})", { response.status });
                    break Err(Error::new(ErrorCode::InvalidMessage, reason));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;

            match message.header {
                LinuxDaemonMessageHeader::GetDirectoryEntriesResponsePart => {
                    let part: LinuxDaemonMessagePart =
                        LinuxDaemonMessagePart::from_bytes(message.payload);

                    // Add part to message assembler and check for errors.
                    if let Err(error) = assembler.add_part(part) {
                        let reason: &str = "failed to assemble message";
                        error!("posix_getdents(): {reason} (error={:?})", error);
                        break Err(error);
                    }

                    // Check if we received all parts of the message.
                    if !assembler.is_complete() {
                        continue;
                    }

                    let parts: Vec<LinuxDaemonMessagePart> = assembler.take_parts();

                    match GetDirectoryEntriesResponse::from_parts(&parts) {
                        Ok(response) => break Ok(response.entries),
                        Err(error) => {
                            error!("posix_getdents(): invalid message (error={:?})", error);
                            break Err(error);
                        },
                    }
                },
                header => {
                    let reason: &str = "unexpected message type";
                    error!("posix_getdents(): {reason} (header={header:?})");
                    break Err(Error::new(ErrorCode::InvalidMessage, reason));
                },
            }
        }
    }
}
