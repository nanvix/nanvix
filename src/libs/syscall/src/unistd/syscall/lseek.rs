// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::{
        SeekRequest,
        SeekResponse,
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
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn lseek(fd: RawFileDescriptor, offset: off_t, whence: c_int) -> Result<off_t, Error> {
    ::syslog::trace!("lseek(): fd={:?}, offset={}, whence={}", fd, offset, whence);

    // Route to the VFS if this is a VFS file descriptor.
    #[cfg(feature = "memfs")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_lseek(fd, offset, whence).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("lseek(): VFS lseek failed (fd={fd}, error={e})");
                Error::new(code, "vfs lseek failed")
            });
        }
    }

    // In standalone mode, reject non-VFS fds (no linuxd).
    #[cfg(feature = "standalone")]
    {
        let _ = (fd, offset, whence);
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "lseek not available in standalone mode",
        ));
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    lseek_linuxd(fd, offset, whence)
}

/// Forwards a `lseek` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn lseek_linuxd(fd: RawFileDescriptor, offset: off_t, whence: c_int) -> Result<off_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = SeekRequest::build(tid, fd, offset, whence);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "lseek(): failed (fd={}, offset={}, whence={}, error={})",
            fd,
            offset,
            whence,
            { response.status },
        );

        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "lseek() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::syslog::error!(
                    "lseek(): failed to parse error code (fd={}, offset={}, whence={}, error={:?})",
                    fd,
                    offset,
                    whence,
                    error
                );
                Err(Error::new(ErrorCode::InvalidMessage, "failed to parse error code"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::SeekResponse => {
                // Parse response.
                let response: SeekResponse = SeekResponse::from_bytes(message.payload);

                Ok(response.offset)
            },
            // Response was not successfully parsed.
            header => {
                ::syslog::error!(
                    "lseek(): failed to parse response (fd={}, offset={}, whence={}, header={:?})",
                    fd,
                    offset,
                    whence,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "failed to parse response"))
            },
        }
    }
}
