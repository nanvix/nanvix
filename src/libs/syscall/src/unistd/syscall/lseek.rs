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
#[cfg(feature = "standalone")]
use ::sysapi::unistd::{
    STDERR_FILENO,
    STDIN_FILENO,
    STDOUT_FILENO,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
};
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        unistd::message::{
            SeekRequest,
            SeekResponse,
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

pub fn lseek(fd: RawFileDescriptor, offset: off_t, whence: c_int) -> Result<off_t, Error> {
    ::syslog::trace!("lseek(): fd={:?}, offset={}, whence={}", fd, offset, whence);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_lseek(fd, offset, whence).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("lseek(): VFS lseek failed (fd={fd}, error={e})");
                Error::new(code, "vfs lseek failed")
            });
        }
        if fd == STDIN_FILENO || fd == STDOUT_FILENO || fd == STDERR_FILENO {
            let reason: &str = "illegal seek on stdio";
            ::syslog::trace!("lseek(): {reason} (fd={fd})");
            return Err(Error::new(ErrorCode::IllegalSeek, reason));
        }
        let reason: &str = "lseek: fd is not a VFS fd in standalone mode";
        ::syslog::trace!("lseek(): {reason} (fd={fd})");
        Err(Error::new(ErrorCode::BadFile, reason))
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
