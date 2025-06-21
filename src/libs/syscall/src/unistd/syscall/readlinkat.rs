// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::{
        LinuxDaemonLongMessage,
        LinuxDaemonMessagePart,
        MessagePartitioner,
    },
    unistd::message::{
        ReadLinkAtRequest,
        ReadLinkAtResponse,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
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
    ipc::Message,
    pm::ProcessIdentifier,
};
use ::sysapi::sys_types::ssize_t;

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
pub fn readlinkat(dirfd: i32, path: &str, buf: &mut [u8]) -> Result<ssize_t, Error> {
    ::syslog::trace!("readlinkat(): dirfd={:?}, path={:?}, buf.len={:?}", dirfd, path, buf.len());

    let pid: ProcessIdentifier = ::sys::kcall::pm::getpid()?;

    let request: ReadLinkAtRequest = ReadLinkAtRequest::new(dirfd, path.to_string(), buf.len())?;

    let requests: Vec<Message> = request.into_parts(pid)?;

    for request in requests {
        ::sys::kcall::ipc::send(&request)?;
    }

    let capacity: usize =
        ReadLinkAtResponse::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);

    let mut assembler: LinuxDaemonLongMessage = LinuxDaemonLongMessage::new(capacity)?;

    loop {
        let response: Message = ::sys::kcall::ipc::recv()?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            ::syslog::error!(
                "readlinkat(): system call failed (dirfd={:?}, path={:?}, error_code={:?})",
                dirfd,
                path,
                { response.status }
            );
            // System call failed, parse error code and return.
            match ErrorCode::try_from(response.status) {
                Ok(error_code) => break Err(Error::new(error_code, "system call failed")),
                Err(error) => {
                    ::syslog::error!(
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
            let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
            match message.header {
                LinuxDaemonMessageHeader::ReadLinkAtResponsePart => {
                    let part: LinuxDaemonMessagePart =
                        LinuxDaemonMessagePart::from_bytes(message.payload);

                    if let Err(error) = assembler.add_part(part) {
                        ::syslog::error!(
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

                    let parts: Vec<LinuxDaemonMessagePart> = assembler.take_parts();

                    match ReadLinkAtResponse::from_parts(&parts) {
                        Ok(response) => {
                            assert!(response.buffer.len() <= buf.len());
                            buf[..response.buffer.len()].copy_from_slice(&response.buffer);
                            break Ok(response.buffer.len() as i32);
                        },
                        Err(error) => {
                            ::syslog::error!(
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
                        ::syslog::error!(
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
