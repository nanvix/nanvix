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
    SystemCallMessage,
    SystemCallMessageHeader,
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

pub fn fcntl(fd: i32, cmd: i32, arg: Option<c_int>) -> Result<c_int, Error> {
    ::syslog::trace!("fcntl(): fd={:?}, cmd={:?}, arg={:?}", fd, cmd, arg);
    let backend_fd: i32 = crate::fdtable::resolve_vfs(fd, "fcntl")?;
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: Message = FileControlRequest::build(
        tid,
        backend_fd,
        cmd,
        arg.unwrap_or(0),
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not.
    if response.status == -1 {
        ::syslog::warn!(
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
                ::syslog::warn!(
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
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::FileControlResponse => {
                let message: FileControlResponse = FileControlResponse::from_bytes(message.payload);
                let ret: c_int = message.ret;
                Ok(ret)
            },
            // Response was not successfully parsed.
            header => {
                ::syslog::warn!(
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
