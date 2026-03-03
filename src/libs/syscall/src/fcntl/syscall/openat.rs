// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::{
        OpenAtRequest,
        OpenAtResponse,
    },
    message::MessagePartitioner,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
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
use ::sysapi::{
    ffi::c_int,
    sys_types::mode_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(unreachable_code)]
pub fn openat(dirfd: i32, pathname: &str, flags: c_int, mode: mode_t) -> Result<c_int, Error> {
    ::syslog::trace!(
        "openat(): dirfd={dirfd:?}, pathname={pathname:?}, flags={flags:?}, mode={mode:?}"
    );

    // Route to the VFS if the path belongs to an in-memory filesystem mount.
    #[cfg(feature = "memfs")]
    {
        if ::nvx::vfs::fd::is_vfs_path(pathname) {
            return ::nvx::vfs::fd::vfs_open(pathname, flags).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("openat(): VFS open failed (pathname={pathname:?}, error={e})");
                Error::new(code, "vfs open failed")
            });
        }
    }

    // In standalone mode, reject non-VFS paths (no linuxd).
    #[cfg(feature = "standalone")]
    {
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "openat not available in standalone mode",
        ));
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: OpenAtRequest = OpenAtRequest::new(dirfd, pathname, flags, mode)?;
    let requests: Vec<Message> = request.into_parts(tid)?;
    for request in &requests {
        ::sys::kcall::ipc::send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        ::syslog::error!(
            "openat(): failed (dirfd={:?}, pathname={:?}, flags={:?}, mode={:?}, error={:?})",
            dirfd,
            pathname,
            flags,
            mode,
            { response.status }
        );
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "openat() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::error!(
                    "openat(): failed to parse error code (dirfd={:?}, pathname={:?}, flags={:?}, \
                     mode={:?}, error={:?})",
                    dirfd,
                    pathname,
                    flags,
                    mode,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "openat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                LinuxDaemonMessageHeader::OpenAtResponse => {
                    // Parse response.
                    let response: OpenAtResponse = OpenAtResponse::from_bytes(message.payload);

                    // Return file descriptor.
                    Ok(response.ret)
                },
                // Response was not successfully parsed.
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            // Response was not successfully parsed.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
        }
    }
}
