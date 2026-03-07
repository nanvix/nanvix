// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        unistd::message::{
            PipeRequest,
            PipeResponse,
        },
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::sys::{
        ipc::Message,
        pm::ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn pipe() -> Result<[i32; 2], Error> {
    ::syslog::trace!("pipe()");

    // In standalone mode, pipe is not available (no linuxd).
    #[cfg(feature = "standalone")]
    {
        Err(Error::new(ErrorCode::OperationNotSupported, "pipe not available in standalone mode"))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    pipe_linuxd()
}

/// Forwards a `pipe` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn pipe_linuxd() -> Result<[i32; 2], Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = PipeRequest::build(tid);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::error!("pipe(): failed (error={})", error_code);
        Err(Error::new(error_code, "pipe() failed"))
    } else {
        // System call succeeded, parse response.
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::PipeResponse => {
                    // Parse response.
                    let response: PipeResponse = PipeResponse::from_bytes(message.payload);

                    ::syslog::trace!(
                        "pipe(): read_fd={:?}, write_fd={:?}",
                        { response.read_fd },
                        { response.write_fd },
                    );
                    Ok([response.read_fd, response.write_fd])
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

    ///
    /// # Description
    ///
    /// Creates a pipe.
    ///
    /// # Parameters
    ///
    /// - `fds`: Array to store the file descriptors of the pipe.
    ///
    /// # Returns
    ///
    /// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
    /// indicate the error.
    ///
    #[unsafe(no_mangle)]
    #[trace_syscall]
    pub unsafe extern "C" fn pipe(fds: *mut c_int) -> c_int {
        match super::pipe() {
            Ok([read_fd, write_fd]) => {
                ::syslog::trace!("pipe(): read_fd={read_fd:?}, write_fd={write_fd:?}");
                unsafe {
                    *fds.offset(0) = read_fd;
                    *fds.offset(1) = write_fd;
                }
                0
            },
            Err(error) => {
                ::syslog::error!("pipe(): failed (error={error:?})");
                unsafe {
                    *__errno_location() = error.code.get();
                }
                -1
            },
        }
    }
}
