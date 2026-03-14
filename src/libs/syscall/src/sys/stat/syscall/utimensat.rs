// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::time::timespec;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        message::MessagePartitioner,
        sys::stat::message::UpdateFileAccessTimeAtRequest,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::alloc::{
        string::ToString,
        vec::Vec,
    },
    ::sys::{
        error::ErrorCode,
        ipc::Message,
        pm::ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets file access and modification times.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `pathname`: Pathname of the file.
/// - `times`: Access and modification times.
/// - `flags`: Flags.
///
/// # Returns
///
/// Upon successful completion, the `utimensat()` system call returns empty. Otherwise, it returns
/// an error.
///
#[allow(unreachable_code)]
pub fn utimensat(
    dirfd: i32,
    pathname: &str,
    times: &[timespec; 2],
    flags: i32,
) -> Result<(), Error> {
    ::syslog::trace!(
        "utimensat(): dirfd={:?}, pathname={:?}, times={:?}, flags={:?}",
        dirfd,
        pathname,
        times,
        flags
    );

    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_utimensat(dirfd, pathname, times, flags).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            Error::new(code, "vfs utimensat failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    utimensat_linuxd(dirfd, pathname, times, flags)
}

/// Forwards a `utimensat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn utimensat_linuxd(
    dirfd: i32,
    pathname: &str,
    times: &[timespec; 2],
    flags: i32,
) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let request: UpdateFileAccessTimeAtRequest =
        UpdateFileAccessTimeAtRequest::new(dirfd, pathname.to_string(), flags, times)?;

    let requests: Vec<Message> = request.into_parts(tid)?;

    // Send request.
    for request in &requests {
        sys::kcall::ipc::send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        ::syslog::error!(
            "utimensat(): failed (dirfd={:?}, pathname={:?}, times={:?}, flags={:?}, \
             error_code={:?})",
            dirfd,
            pathname,
            times,
            flags,
            { response.status }
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => Err(Error::new(error_code, "utimensat() failed")),
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::error!(
                    "utimensat(): failed to convert error code (dirfd={:?}, pathname={:?}, \
                     times={:?}, flags={:?}, error={:?})",
                    dirfd,
                    pathname,
                    times,
                    flags,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "utimensat() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::UpdateFileAccessTimeAtResponse => Ok(()),
            // Response was not successfully parsed.
            _ => {
                let reason: &str = "unexpected message header";
                ::syslog::error!(
                    "utimensat(): failed (dirfd={:?}, pathname={:?}, times={:?}, flags={:?}, \
                     reason={:?})",
                    dirfd,
                    pathname,
                    times,
                    flags,
                    reason
                );
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
