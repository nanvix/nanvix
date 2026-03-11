// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::RawFileDescriptor;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::ffi::c_int;
use sysapi::sys_types::off_t;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        fcntl::message::FileAdvisoryInformationRequest,
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

///
/// # Description
///
/// Provides advice about the use of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset in bytes.
/// - `len`: Length in bytes.
/// - `advice`: Advice to provide.
///
/// # Returns
///
/// Upon success, `posix_fadvise()` empty. Otherwise, it returns an error.
///
#[allow(unreachable_code)]
pub fn posix_fadvise(
    fd: RawFileDescriptor,
    offset: off_t,
    len: off_t,
    advice: c_int,
) -> Result<(), Error> {
    ::syslog::error!(
        "posix_fadvise(): fd={:?}, offset={:?}, len={:?}, advice={:?}",
        fd,
        offset,
        len,
        advice
    );

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return Ok(());
        }
        Err(Error::new(
            ErrorCode::OperationNotSupported,
            "fadvise not available in standalone mode",
        ))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    posix_fadvise_linuxd(fd, offset, len, advice)
}

/// Forwards a `posix_fadvise` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn posix_fadvise_linuxd(
    fd: RawFileDescriptor,
    offset: off_t,
    len: off_t,
    advice: c_int,
) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = FileAdvisoryInformationRequest::build(tid, fd, offset, len, advice);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "posix_fadvise(): failed (fd={:?}, offset={:?}, len={:?}, advice={:?}, status={:?})",
            fd,
            offset,
            len,
            advice,
            { response.status }
        );

        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => Err(Error::new(error_code, "posix_fadvise() failed")),
            // Error code was not successfully parsed.
            Err(error) => {
                ::syslog::error!("posix_fadvise(): invalid error code (error={:?})", error);
                Err(Error::new(ErrorCode::TryAgain, "posix_fadvise(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileAdvisoryInformationResponse => Ok(()),
            header => {
                // Response was not successfully parsed.
                ::syslog::error!(
                    "posix_fadvise(): unexpected message header (fd={:?}, offset={:?}, len={:?}, \
                     advice={:?}, header={:?})",
                    fd,
                    offset,
                    len,
                    advice,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header"))
            },
        }
    }
}
