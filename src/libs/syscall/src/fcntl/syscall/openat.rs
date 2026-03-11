// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::mode_t,
};
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        fcntl::message::{
            OpenAtRequest,
            OpenAtResponse,
        },
        message::MessagePartitioner,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::alloc::vec::Vec,
    ::sys::{
        ipc::Message,
        pm::ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(unreachable_code)]
pub fn openat(dirfd: i32, pathname: &str, flags: c_int, mode: mode_t) -> Result<c_int, Error> {
    ::syslog::trace!(
        "openat(): dirfd={dirfd:?}, pathname={pathname:?}, flags={flags:?}, mode={mode:?}"
    );

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_open(pathname, flags).map_err(|e| {
            let code: ErrorCode = e.into();
            ::syslog::error!("openat(): VFS open failed (pathname={pathname:?}, error={e})");
            Error::new(code, "vfs open failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    openat_linuxd(dirfd, pathname, flags, mode)
}

/// Forwards an `openat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn openat_linuxd(dirfd: i32, pathname: &str, flags: c_int, mode: mode_t) -> Result<c_int, Error> {
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
