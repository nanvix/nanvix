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
use sysapi::sys_types::mode_t;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        sys::stat::message::FileChmodRequest,
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
/// Changes the mode of a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, `fchmod()` returns empty. Otherwise, it returns an error.
///
#[allow(unreachable_code)]
pub fn fchmod(fd: RawFileDescriptor, mode: mode_t) -> Result<(), Error> {
    ::syslog::trace!("fchmod(): fd={:?}, mode={:o}", fd, mode);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return Ok(());
        }
        Err(Error::new(ErrorCode::OperationNotSupported, "fchmod not available in standalone mode"))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    fchmod_linuxd(fd, mode)
}

/// Forwards a `fchmod` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn fchmod_linuxd(fd: RawFileDescriptor, mode: mode_t) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it
    let request: Message = FileChmodRequest::build(tid, fd, mode);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!("fchmod(): syscall failed (fd={:?}, mode={:o}, status={:?})", fd, mode, {
            response.status
        });
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => {
                ::syslog::error!(
                    "fchmod(): syscall failed (fd={:?}, mode={:o}, error_code={:?})",
                    fd,
                    mode,
                    error_code
                );
                Err(Error::new(error_code, "system call failed"))
            },
            Err(error) => {
                ::syslog::error!(
                    "fchmod(): syscall failed (fd={:?}, mode={:o}, error={:?})",
                    fd,
                    mode,
                    error
                );
                Err(Error::new(ErrorCode::InvalidMessage, "system call failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileChmodResponse => Ok(()),
            // Invalid response.
            header => {
                ::syslog::error!(
                    "fchmod(): invalid response (fd={:?}, mode={:o}, header={:?})",
                    fd,
                    mode,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
