// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::{
        PartialWriteRequest,
        PartialWriteResponse,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::core::cmp;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::sys_types::{
    c_size_t,
    off_t,
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

    // POSIX requires pwrite on a non-seekable fd (pipe/stdio) to return ESPIPE.
    #[cfg(feature = "standalone")]
    {
        use ::sysapi::unistd::{
            STDERR_FILENO,
            STDIN_FILENO,
            STDOUT_FILENO,
        };

        if fd == STDIN_FILENO || fd == STDOUT_FILENO || fd == STDERR_FILENO {
            ::syslog::warn!(
                "pwrite(): illegal seek on stdio (fd={fd:?}, buffer={buffer:?}, offset={offset})",
            );
            return Err(Error::new(ErrorCode::IllegalSeek, "illegal seek on stdio"));
        }
    }

    // In standalone mode, only VFS file descriptors should be routed to vfsd.
    #[cfg(feature = "standalone")]
    if !crate::is_vfs_fd(fd) {
        ::syslog::warn!("pwrite(): bad file descriptor fd={fd} in standalone mode");
        return Err(Error::new(
            ErrorCode::BadFile,
            "pwrite: fd is not a VFS fd in standalone mode",
        ));
    }

    let mut total_written: c_size_t = 0;
    let mut buffer_offset: usize = 0;

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

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
            crate::VFS_DESTINATION,
            crate::VFS_MESSAGE_TYPE,
        );
        ::sys::kcall::ipc::__kcall_send(&request)?;

        // Receive response.
        let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

        // Check whether the system call succeeded or not.
        if response.status != 0 {
            ::syslog::warn!(
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
                    ::syslog::warn!("pwrite(): failed to convert error code (error={:?})", error);
                    return Err(Error::new(ErrorCode::TryAgain, "pwritev() failed"));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
            // Response was successfully parsed.
            match message.header {
                // Response was successfully parsed.
                SystemCallMessageHeader::PartialWriteResponse => {
                    // Parse response.
                    let message: PartialWriteResponse =
                        PartialWriteResponse::from_bytes(message.payload);

                    // Update total written count.
                    total_written += message.count as c_size_t;
                    buffer_offset += message.count as usize;
                },
                // Response was not expected.
                header => {
                    ::syslog::warn!(
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
