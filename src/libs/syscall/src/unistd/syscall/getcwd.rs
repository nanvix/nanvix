// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;
use ::sys::error::Error;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        message::{
            LinuxDaemonLongMessage,
            LinuxDaemonMessagePart,
            MessagePartitioner,
        },
        unistd::message::{
            GetCurrentWorkingDirectoryRequest,
            GetCurrentWorkingDirectoryResponse,
        },
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::alloc::vec::Vec,
    ::sys::{
        error::ErrorCode,
        ipc::Message,
        pm::ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Gets the current working directory.
pub fn getcwd() -> Result<String, Error> {
    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_getcwd().map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!("getcwd(): VFS getcwd failed (error={e})");
            Error::new(code, "vfs getcwd failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    getcwd_linuxd()
}

/// Forwards a `getcwd` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn getcwd_linuxd() -> Result<String, Error> {
    // Send request.
    getcwd_request()?;

    // Wait for response.
    getcwd_response()
}

/// Handles the request of the `getcwd()` system call.
#[cfg(not(feature = "standalone"))]
fn getcwd_request() -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let request: Message = GetCurrentWorkingDirectoryRequest::build(tid);

    // Send request.
    ::sys::kcall::ipc::send(&request)
}

/// Handles the response of the `getcwd()` system call.
#[cfg(not(feature = "standalone"))]
fn getcwd_response() -> Result<String, Error> {
    // Compute the maximum number of parts in the response.
    let capacity: usize =
        GetCurrentWorkingDirectoryResponse::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);

    let mut assembler: LinuxDaemonLongMessage = LinuxDaemonLongMessage::new(capacity)?;

    loop {
        let response: Message = ::sys::kcall::ipc::recv()?;

        // Check whether the system call succeeded or not.
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
                LinuxDaemonMessageHeader::GetCurrentWorkingDirectoryResponsePart => {
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
