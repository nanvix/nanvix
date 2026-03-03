// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::{
        FileControlRequest,
        FileControlResponse,
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
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(unreachable_code)]
pub fn fcntl(fd: i32, cmd: i32, arg: Option<c_int>) -> Result<c_int, Error> {
    ::syslog::trace!("fcntl(): fd={:?}, cmd={:?}, arg={:?}", fd, cmd, arg);

    // Route to the VFS if this is a VFS file descriptor.
    #[cfg(feature = "memfs")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_fcntl(fd, cmd).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("fcntl(): VFS fcntl failed (fd={fd}, cmd={cmd}, error={e})");
                Error::new(code, "vfs fcntl failed")
            });
        }
    }

    // In standalone mode, handle common fcntl commands on non-VFS fds without IPC.
    #[cfg(feature = "standalone")]
    {
        use ::sysapi::fcntl::file_control_request;
        match cmd {
            file_control_request::F_GETFD
            | file_control_request::F_SETFD
            | file_control_request::F_GETFL
            | file_control_request::F_SETFL => return Ok(0),
            _ => {
                return Err(Error::new(
                    ErrorCode::OperationNotSupported,
                    "fcntl cmd not supported in standalone mode",
                ));
            },
        }
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = FileControlRequest::build(tid, fd, cmd, arg.unwrap_or(0));
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status == -1 {
        ::syslog::error!(
            "fcntl(): failed (fd={:?}, cmd={:?}, arg={:?}, status={:?})",
            fd,
            cmd,
            arg,
            { response.status }
        );

        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error code.
                Err(Error::new(error_code, "fcntl() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::syslog::error!(
                    "fcntl(): failed to parse error code (fd={:?}, cmd={:?}, arg={:?}, error={:?})",
                    fd,
                    cmd,
                    arg,
                    error
                );
                // Return error code.
                Err(Error::new(ErrorCode::TryAgain, "fcntl() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileControlResponse => {
                let message: FileControlResponse = FileControlResponse::from_bytes(message.payload);
                let ret: c_int = message.ret;
                Ok(ret)
            },
            // Response was not successfully parsed.
            header => {
                ::syslog::error!(
                    "fcntl(): invalid response (fd={:?}, cmd={:?}, arg={:?}, header={:?})",
                    fd,
                    cmd,
                    arg,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "fcntl() failed"))
            },
        }
    }
}
