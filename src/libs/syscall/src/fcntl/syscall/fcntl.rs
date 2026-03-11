// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::ffi::c_int;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        fcntl::message::{
            FileControlRequest,
            FileControlResponse,
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

#[allow(unreachable_code)]
pub fn fcntl(fd: i32, cmd: i32, arg: Option<c_int>) -> Result<c_int, Error> {
    ::syslog::trace!("fcntl(): fd={:?}, cmd={:?}, arg={:?}", fd, cmd, arg);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_fcntl(fd, cmd).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("fcntl(): VFS fcntl failed (fd={fd}, cmd={cmd}, error={e})");
                Error::new(code, "vfs fcntl failed")
            });
        }
        use ::sysapi::fcntl::file_control_request;
        match cmd {
            file_control_request::F_GETFD
            | file_control_request::F_SETFD
            | file_control_request::F_GETFL
            | file_control_request::F_SETFL => Ok(0),
            _ => Err(Error::new(
                ErrorCode::OperationNotSupported,
                "fcntl cmd not supported in standalone mode",
            )),
        }
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    fcntl_linuxd(fd, cmd, arg)
}

/// Forwards a `fcntl` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn fcntl_linuxd(fd: i32, cmd: i32, arg: Option<c_int>) -> Result<c_int, Error> {
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
