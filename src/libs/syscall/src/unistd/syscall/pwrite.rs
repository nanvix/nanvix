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
use ::sysapi::sys_types::{
    c_size_t,
    off_t,
};
#[cfg(feature = "standalone")]
use ::sysapi::unistd::{
    STDERR_FILENO,
    STDIN_FILENO,
    STDOUT_FILENO,
};
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        unistd::message::{
            PartialWriteRequest,
            PartialWriteResponse,
        },
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::core::cmp,
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
/// Writes data to a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to write.
/// - `offset`: Offset to write to.
///
/// # Returns
///
/// Upon successful completion, `pwrite()` returns the number of bytes written. Otherwise, it
/// returns an error.
///
pub fn pwrite(fd: RawFileDescriptor, buffer: &[u8], offset: off_t) -> Result<c_size_t, Error> {
    ::syslog::trace!("pwrite(): fd={}, buffer={:?}, offset={}", fd, buffer, offset);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_pwrite(fd, buffer, offset).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("pwrite(): VFS pwrite failed (fd={fd}, error={e})");
                Error::new(code, "vfs pwrite failed")
            });
        }
        if fd == STDIN_FILENO || fd == STDOUT_FILENO || fd == STDERR_FILENO {
            let reason: &str = "illegal seek on stdio";
            ::syslog::error!("pwrite(): {reason} (fd={fd})");
            return Err(Error::new(ErrorCode::IllegalSeek, reason));
        }
        let reason: &str = "pwrite not available in standalone mode";
        ::syslog::error!("pwrite(): {reason} (fd={fd})");
        Err(Error::new(ErrorCode::OperationNotSupported, reason))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    pwrite_linuxd(fd, buffer, offset)
}

/// Forwards a `pwrite` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn pwrite_linuxd(fd: RawFileDescriptor, buffer: &[u8], offset: off_t) -> Result<c_size_t, Error> {
    let mut total_written: c_size_t = 0;
    let mut buffer_offset: usize = 0;

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    while buffer_offset < buffer.len() {
        let chunk_size: usize =
            cmp::min(PartialWriteRequest::BUFFER_SIZE, buffer.len() - buffer_offset);
        let mut chunk: [u8; PartialWriteRequest::BUFFER_SIZE] =
            [0; PartialWriteRequest::BUFFER_SIZE];
        chunk[..chunk_size].copy_from_slice(&buffer[buffer_offset..buffer_offset + chunk_size]);

        // Build request and send it.
        let request: Message = PartialWriteRequest::build(
            tid,
            fd,
            chunk_size as c_size_t,
            offset + buffer_offset as off_t,
            chunk,
        );
        ::sys::kcall::ipc::send(&request)?;

        // Receive response.
        let response: Message = ::sys::kcall::ipc::recv()?;

        // Check whether the system call succeeded or not.
        if response.status != 0 {
            ::syslog::error!(
                "pwrite(): failed (fd={}, buffer.len={}, error_code={})",
                fd,
                buffer.len(),
                { response.status }
            );

            match ErrorCode::try_from(response.status) {
                // Error code was successfully parsed.
                Ok(error_code) => return Err(Error::new(error_code, "pwritev() failed")),
                // Error code was not parsed.
                Err(error) => {
                    ::syslog::error!("pwrite(): failed to convert error code (error={:?})", error);
                    return Err(Error::new(ErrorCode::TryAgain, "pwritev() failed"));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
            // Response was successfully parsed.
            match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::PartialWriteResponse => {
                    // Parse response.
                    let message: PartialWriteResponse =
                        PartialWriteResponse::from_bytes(message.payload);

                    // Update total written count.
                    total_written += message.count as c_size_t;
                    buffer_offset += message.count as usize;
                },
                // Response was not expected.
                header => {
                    ::syslog::error!(
                        "pwrite(): failed to parse response (fd={}, buffer.len={}, header={:?})",
                        fd,
                        buffer.len(),
                        header
                    );
                    return Err(Error::new(ErrorCode::InvalidMessage, "failed to parse response"));
                },
            }
        }
    }

    Ok(total_written)
}
