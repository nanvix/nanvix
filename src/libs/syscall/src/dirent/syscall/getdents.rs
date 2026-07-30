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
        MessagePartitioner,
        SystemCallLongMessage,
        SystemCallMessagePart,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
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
    trace,
    warn,
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

    const MESSAGE_ASSEMBLER_CAPACITY: usize =
        GetDirectoryEntriesResponse::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    let backend_fd: c_int = crate::fdtable::resolve_vfs(fd, "posix_getdents")?;

    // Build request message.
    let request: Message = GetDirectoryEntriesRequest::build(
        tid,
        backend_fd,
        count,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    )
    .map_err(|error| {
        let reason: &str = "failed to build message";
        warn!("posix_getdents(): {reason} (error={:?})", error);
        Error::new(error.code, reason)
    })?;

    // Send request message.
    ::sys::kcall::ipc::__kcall_send(&request).map_err(|error| {
        let reason: &str = "failed to send message";
        warn!("posix_getdents(): {reason} (error={:?})", error);
        Error::new(error.code, reason)
    })?;

    // Create message assembler.
    let mut assembler: SystemCallLongMessage =
        SystemCallLongMessage::new(MESSAGE_ASSEMBLER_CAPACITY).map_err(|error| {
            let reason: &str = "failed to create message assembler";
            warn!("posix_getdents(): {reason} (error={:?})", error);
            Error::new(error.code, reason)
        })?;

    loop {
        // Wait for response message.
        let response: Message = ::sys::kcall::ipc::__kcall_recv_response().map_err(|error| {
            let reason: &str = "failed to receive message";
            warn!("posix_getdents(): {reason} (error={:?})", error);
            Error::new(error.code, reason)
        })?;

        // Check whether system call succeeded or not
        if response.status != 0 {
            // System call failed, parse error code and return it.
            match ErrorCode::try_from(response.status) {
                Ok(error_code) => {
                    let reason: &str = "system call failed";
                    warn!("posix_getdents(): {reason} (error_code={error_code:?})");
                    break Err(Error::new(error_code, reason));
                },
                Err(_) => {
                    let reason: &str = "failed to parse error code";
                    warn!("posix_getdents(): {reason} (response.status={})", { response.status });
                    break Err(Error::new(ErrorCode::InvalidMessage, reason));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;

            match message.header {
                SystemCallMessageHeader::GetDirectoryEntriesResponsePart => {
                    let part: SystemCallMessagePart =
                        SystemCallMessagePart::from_bytes(message.payload);

                    // Add part to message assembler and check for errors.
                    if let Err(error) = assembler.add_part(part) {
                        let reason: &str = "failed to assemble message";
                        warn!("posix_getdents(): {reason} (error={:?})", error);
                        break Err(error);
                    }

                    // Check if we received all parts of the message.
                    if !assembler.is_complete() {
                        continue;
                    }

                    let parts: Vec<SystemCallMessagePart> = assembler.take_parts();

                    match GetDirectoryEntriesResponse::from_parts(&parts) {
                        Ok(response) => break Ok(response.entries),
                        Err(error) => {
                            warn!("posix_getdents(): invalid message (error={:?})", error);
                            break Err(error);
                        },
                    }
                },
                header => {
                    let reason: &str = "unexpected message type";
                    warn!("posix_getdents(): {reason} (header={header:?})");
                    break Err(Error::new(ErrorCode::InvalidMessage, reason));
                },
            }
        }
    }
}
