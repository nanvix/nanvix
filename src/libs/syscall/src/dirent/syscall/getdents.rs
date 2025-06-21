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
    pm::ProcessIdentifier,
};
use ::sysapi::ffi::c_int;

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
    ::syslog::trace!("posix_getdents(): fd={}, count={:?}", fd, count);
    posix_getdents_request(fd, count)?;
    posix_getdents_response()
}

/// Processes the request of the `posix_getdents()` system call.
fn posix_getdents_request(fd: c_int, count: usize) -> Result<(), Error> {
    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    let request: Message = GetDirectoryEntriesRequest::build(pid, fd, count)?;

    ::sys::kcall::ipc::send(&request)
}

/// Processes the response of the `posix_getdents()` system call.
fn posix_getdents_response() -> Result<Vec<posix_dent>, Error> {
    let capacity: usize =
        GetDirectoryEntriesResponse::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);

    let mut assembler: LinuxDaemonLongMessage = LinuxDaemonLongMessage::new(capacity)?;

    loop {
        let response: Message = ::sys::kcall::ipc::recv()?;

        // Check whether system call succeeded or not
        if response.status != 0 {
            // System call failed, parse error code and return it.
            match ErrorCode::try_from(response.status) {
                Ok(error_code) => return Err(Error::new(error_code, "system call failed")),
                Err(_) => break Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
            }
        } else {
            // System call succeeded, parse response.
            let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;

            match message.header {
                LinuxDaemonMessageHeader::GetDirectoryEntriesResponsePart => {
                    let part: LinuxDaemonMessagePart =
                        LinuxDaemonMessagePart::from_bytes(message.payload);

                    // Add part to message assembler and check for errors.
                    if let Err(e) = assembler.add_part(part) {
                        break Err(e);
                    }

                    // Check if we received all parts of the message.
                    if !assembler.is_complete() {
                        continue;
                    }

                    let parts: Vec<LinuxDaemonMessagePart> = assembler.take_parts();

                    match GetDirectoryEntriesResponse::from_parts(&parts) {
                        Ok(response) => break Ok(response.entries),
                        Err(error) => {
                            ::syslog::warn!(
                                "posix_getdents(): invalid message (error={:?})",
                                error
                            );
                            break Err(error);
                        },
                    }
                },
                _ => break Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
            }
        }
    }
}
