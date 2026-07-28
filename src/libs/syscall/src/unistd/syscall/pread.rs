// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::{
        PartialReadRequest,
        PartialReadResponse,
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

    // POSIX requires pread on a non-seekable fd (pipe/stdio) to return ESPIPE.
    let backend_fd: RawFileDescriptor = {
        use crate::fdtable::{
            resolve_result,
            Route,
        };
        match resolve_result(fd)? {
            // VFS-backed descriptors fall through to the vfsd read path below.
            Some(res) if res.route == Route::Vfs => res.backend_fd,
            // The console (stdin/stdout/stderr) is not seekable.
            Some(res) if res.route == Route::Console => {
                ::syslog::warn!(
                    "pread(): illegal seek on stdio (fd={fd}, buffer={buffer:?}, offset={offset})",
                );
                return Err(Error::new(ErrorCode::IllegalSeek, "illegal seek on stdio"));
            },
            // Sockets and unroutable descriptors are not readable here.
            _ => {
                ::syslog::warn!("pread(): bad file descriptor fd={fd}");
                return Err(Error::new(ErrorCode::BadFile, "pread: fd is not a VFS fd"));
            },
        }
    };

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let mut total_read: c_size_t = 0;
    let mut buffer_offset: usize = 0;

    while buffer_offset < buffer.len() {
        let chunk_size: usize =
            cmp::min(PartialReadResponse::BUFFER_SIZE, buffer.len() - buffer_offset);

        // Build request and send it.
        let request: Message = PartialReadRequest::build(
            tid,
            backend_fd,
            chunk_size as c_size_t,
            offset + buffer_offset as off_t,
            crate::VFS_DESTINATION,
            crate::VFS_MESSAGE_TYPE,
        );
        ::sys::kcall::ipc::__kcall_send(&request)?;

        // Receive response.
        let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

        // Check whether system call succeeded or not.
        if response.status != 0 {
            ::syslog::warn!(
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
                    ::syslog::warn!("pread(): failed to convert error code (error={:?})", error);
                    return Err(Error::new(ErrorCode::TryAgain, "pread() failed"));
                },
            }
        } else {
            // System call succeeded, parse response.
            let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
            // Response was successfully parsed.
            match message.header {
                // Response was successfully parsed.
                SystemCallMessageHeader::PartialReadResponse => {
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
                    ::syslog::warn!(
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
