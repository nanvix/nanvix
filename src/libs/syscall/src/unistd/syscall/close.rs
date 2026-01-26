// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    unistd::message::{
        CloseRequest,
        CloseResponse,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
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

pub fn close(fd: i32) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = CloseRequest::build(tid, fd);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::error!("close(): failed (error={})", error_code);
        Err(Error::new(error_code, "close() failed"))
    } else {
        // System call succeeded, parse response.
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::CloseResponse => {
                    // Parse response.
                    let _: CloseResponse = CloseResponse::from_bytes(message.payload);

                    // Return result.
                    Ok(())
                },
                // Response was not successfully parsed.
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            // Response was not successfully parsed.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
        }
    }
}

pub mod bindings {
    use crate::errno::__errno_location;
    use ::sysapi::ffi::c_int;
    use ::syslog::trace_syscall;

    #[unsafe(no_mangle)]
    #[trace_syscall]
    pub extern "C" fn close(fd: c_int) -> c_int {
        match crate::unistd::close(fd) {
            Ok(()) => 0,
            Err(error) => {
                ::syslog::error!("close(): failed ({:?})", error);
                unsafe {
                    *__errno_location() = error.code.get();
                }
                -1
            },
        }
    }
}
