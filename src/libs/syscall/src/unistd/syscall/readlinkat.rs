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
        ReadLinkAtRequest,
        ReadLinkAtResponse,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        RequestToken,
    },
    pm::ThreadIdentifier,
};
use ::sysapi::sys_types::c_ssize_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads the value of a symbolic link relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`: Path to the symbolic link.
/// - `buf`: Buffer to store the value of the symbolic link.
///
/// # Returns
///
/// Upon successful completion, `readlinkat()` returns the number of bytes read. Otherwise, it
/// returns an error.
///
pub fn readlinkat(dirfd: i32, path: &str, buf: &mut [u8]) -> Result<c_ssize_t, Error> {
    ::syslog::trace!("readlinkat(): dirfd={:?}, path={:?}, buf.len={:?}", dirfd, path, buf.len());

    let path: alloc::borrow::Cow<'_, str> = crate::path::expand_path(path);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let request: ReadLinkAtRequest = ReadLinkAtRequest::new(dirfd, path.to_string(), buf.len())?;

    let mut requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;

    let token: RequestToken = crate::rpc::send_requests(&mut requests)?;

    let capacity: usize =
        ReadLinkAtResponse::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);

    let mut assembler: SystemCallLongMessage = SystemCallLongMessage::new(capacity)?;

    loop {
        let response: Message = crate::rpc::recv_response(&token)?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            // System call failed, parse error code and return.
            match ErrorCode::try_from(response.status) {
                Ok(error_code) => {
                    ::syslog::warn!(
                        "readlinkat(): system call failed (dirfd={:?}, path={:?}, error_code={:?})",
                        dirfd,
                        path,
                        error_code
                    );
                    break Err(Error::new(error_code, "system call failed"));
                },
                Err(error) => {
                    ::syslog::warn!(
                        "readlinkat(): failed to parse error code (dirfd={:?}, path={:?}, \
                         error_code={:?})",
                        dirfd,
                        path,
                        error
                    );
                    break Err(Error::new(ErrorCode::InvalidMessage, "failed to parse error code"));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
            match message.header {
                SystemCallMessageHeader::ReadLinkAtResponsePart => {
                    let part: SystemCallMessagePart =
                        SystemCallMessagePart::from_bytes(message.payload);

                    if let Err(error) = assembler.add_part(part) {
                        ::syslog::warn!(
                            "readlinkat(): failed to add part (dirfd={:?}, path={:?}, \
                             error_code={:?})",
                            dirfd,
                            path,
                            error
                        );
                        break Err(Error::new(
                            ErrorCode::InvalidMessage,
                            "failed to assemble response part",
                        ));
                    }

                    if !assembler.is_complete() {
                        continue;
                    }

                    let parts: Vec<SystemCallMessagePart> = assembler.take_parts();

                    match ReadLinkAtResponse::from_parts(&parts) {
                        Ok(response) => {
                            assert!(response.buffer.len() <= buf.len());
                            buf[..response.buffer.len()].copy_from_slice(&response.buffer);
                            break Ok(response.buffer.len() as i32);
                        },
                        Err(error) => {
                            ::syslog::warn!(
                                "readlinkat(): failed to assemble response (dirfd={:?}, \
                                 path={:?}, error_code={:?})",
                                dirfd,
                                path,
                                error
                            );
                            break Err(Error::new(
                                ErrorCode::InvalidMessage,
                                "failed to assemble response",
                            ));
                        },
                    }
                },
                header => {
                    break {
                        ::syslog::warn!(
                            "readlinkat(): failed to parse response (dirfd={:?}, path={:?}, \
                             header={:?})",
                            dirfd,
                            path,
                            header
                        );
                        Err(Error::new(ErrorCode::InvalidMessage, "failed to parse response"))
                    }
                },
            }
        }
    }
}
