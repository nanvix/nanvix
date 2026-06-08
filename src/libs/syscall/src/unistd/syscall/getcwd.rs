// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::{
        MessagePartitioner,
        SystemCallLongMessage,
        SystemCallMessagePart,
    },
    unistd::message::{
        GetCurrentWorkingDirectoryRequest,
        GetCurrentWorkingDirectoryResponse,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Gets the current working directory.
pub fn getcwd() -> Result<String, Error> {
    ::syslog::trace!("getcwd()");
    // Send request.
    getcwd_request()?;

    // Wait for response.
    getcwd_response()
}

/// Handles the request of the `getcwd()` system call.
fn getcwd_request() -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let request: Message = GetCurrentWorkingDirectoryRequest::build(
        tid,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );

    // Send request.
    ::sys::kcall::ipc::__kcall_send(&request)
}

/// Handles the response of the `getcwd()` system call.
fn getcwd_response() -> Result<String, Error> {
    // Compute the maximum number of parts in the response.
    let capacity: usize =
        GetCurrentWorkingDirectoryResponse::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);

    let mut assembler: SystemCallLongMessage = SystemCallLongMessage::new(capacity)?;

    loop {
        let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

        // Check whether the system call succeeded or not.
        if response.status != 0 {
            // System call failed, parse error code and return it.
            match ErrorCode::try_from(response.status) {
                Ok(error_code) => return Err(Error::new(error_code, "system call failed")),
                Err(_) => break Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
            }
        } else {
            // System call succeeded, parse response.
            let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;

            match message.header {
                SystemCallMessageHeader::GetCurrentWorkingDirectoryResponsePart => {
                    let part: SystemCallMessagePart =
                        SystemCallMessagePart::from_bytes(message.payload);

                    // Add part to message assembler and check for errors.
                    if let Err(e) = assembler.add_part(part) {
                        break Err(e);
                    }

                    // Check if we received all parts of the message.
                    if !assembler.is_complete() {
                        continue;
                    }

                    let parts: Vec<SystemCallMessagePart> = assembler.take_parts();

                    match GetCurrentWorkingDirectoryResponse::from_parts(&parts) {
                        Ok(response) => break Ok(response.cwd),
                        Err(_) => {
                            break Err(Error::new(ErrorCode::InvalidMessage, "invalid message"))
                        },
                    }
                },
                _ => break Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
            }
        }
    }
}
