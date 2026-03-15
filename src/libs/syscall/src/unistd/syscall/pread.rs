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
            PartialReadRequest,
            PartialReadResponse,
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
/// Reads data from a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to read.
/// - `offset`: Offset to read from.
///
/// # Returns
///
/// Upon successful completion, `pread()` returns the number of bytes read. Otherwise, it
/// returns an error.
///
pub fn pread(fd: RawFileDescriptor, buffer: &mut [u8], offset: off_t) -> Result<c_size_t, Error> {
    ::syslog::trace!("pread(): fd={}, buffer={:?}, offset={}", fd, buffer, offset);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_pread(fd, buffer, offset).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("pread(): VFS pread failed (fd={fd}, error={e})");
                Error::new(code, "vfs pread failed")
            });
        }
        if fd == STDIN_FILENO || fd == STDOUT_FILENO || fd == STDERR_FILENO {
            let reason: &str = "illegal seek on stdio";
            ::syslog::error!("pread(): {reason} (fd={fd})");
            return Err(Error::new(ErrorCode::IllegalSeek, reason));
        }
        let reason: &str = "pread: fd is not a VFS fd in standalone mode";
        ::syslog::error!("pread(): {reason} (fd={fd})");
        Err(Error::new(ErrorCode::BadFile, reason))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    pread_linuxd(fd, buffer, offset)
}

/// Forwards a `pread` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn pread_linuxd(
    fd: RawFileDescriptor,
    buffer: &mut [u8],
    offset: off_t,
) -> Result<c_size_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let mut total_read: c_size_t = 0;
    let mut buffer_offset: usize = 0;

    while buffer_offset < buffer.len() {
        let chunk_size: usize =
            cmp::min(PartialReadResponse::BUFFER_SIZE, buffer.len() - buffer_offset);

        // Build request and send it.
        let request: Message = PartialReadRequest::build(
            tid,
            fd,
            chunk_size as c_size_t,
            offset + buffer_offset as off_t,
        );
        ::sys::kcall::ipc::send(&request)?;

        // Receive response.
        let response: Message = ::sys::kcall::ipc::recv()?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            ::syslog::error!(
                "pread(): failed (fd={}, buffer.len={}, offset={}, error_code={})",
                fd,
                buffer.len(),
                offset,
                { response.status }
            );

            match ErrorCode::try_from(response.status) {
                // System call failed, return error.
                Ok(error_code) => return Err(Error::new(error_code, "pread() failed")),
                // System call failed, return unknown error.
                Err(error) => {
                    ::syslog::error!("pread(): failed to convert error code (error={:?})", error);
                    return Err(Error::new(ErrorCode::TryAgain, "pread() failed"));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
            // Response was successfully parsed.
            match message.header {
                // Response was successfully parsed.
                LinuxDaemonMessageHeader::PartialReadResponse => {
                    // Parse response.
                    let response: PartialReadResponse =
                        PartialReadResponse::from_bytes(message.payload);

                    // Check if any data was read.
                    if response.count == 0 {
                        break;
                    }

                    // Copy response buffer to user buffer.
                    buffer[buffer_offset..buffer_offset + chunk_size]
                        .copy_from_slice(&response.buffer[..chunk_size]);
                    total_read += response.count as c_size_t;
                    buffer_offset += chunk_size;

                    // Check for partial read.
                    if (response.count as usize) < chunk_size {
                        break;
                    }
                },
                header => {
                    ::syslog::error!(
                        "pread(): failed to parse response (fd={}, buffer.len={}, offset={}, \
                         header={:?})",
                        fd,
                        buffer.len(),
                        offset,
                        header
                    );
                    return Err(Error::new(ErrorCode::TryAgain, "pread() failed"));
                },
            }
        }
    }

    Ok(total_read)
}
