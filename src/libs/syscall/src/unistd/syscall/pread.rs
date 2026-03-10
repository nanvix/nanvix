// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
    safe::RawFileDescriptor,
    unistd::message::{
        PositionedReadRequest,
        ReadResponse,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use sysapi::sys_types::{
    c_size_t,
    off_t,
};

use super::util::transfer_chunk_size;

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

    // Route to the VFS if this is a VFS file descriptor.
    #[cfg(feature = "memfs")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_pread(fd, buffer, offset).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("pread(): VFS pread failed (fd={fd}, error={e})");
                Error::new(code, "vfs pread failed")
            });
        }
    }

    // In standalone mode, reject non-VFS fds (no linuxd).
    #[cfg(feature = "standalone")]
    {
        let _ = (fd, buffer, offset);
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "pread not available in standalone mode",
        ));
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    pread_linuxd(fd, buffer, offset)
}

/// Forwards a `pread` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn pread_chunk(
    tid: ThreadIdentifier,
    fd: RawFileDescriptor,
    chunk: &mut [u8],
    offset: off_t,
) -> Result<c_size_t, Error> {
    let request: Message = PositionedReadRequest::build(tid, fd, chunk.len() as c_size_t, offset);
    ::sys::kcall::ipc::send(&request)?;

    let bytes_pulled: usize = ::sys::kcall::ipc::pull(
        ::sys::pm::ProcessIdentifier::KERNEL,
        ::sys::pm::ThreadIdentifier::KERNEL,
        chunk,
    )?;

    let response: Message = ::sys::kcall::ipc::recv()?;

    if response.status != 0 {
        ::syslog::error!(
            "pread_chunk(): failed (fd={fd}, chunk.len={}, offset={offset}, error_code={})",
            chunk.len(),
            { response.status }
        );

        match ErrorCode::try_from(response.status) {
            Ok(error_code) => return Err(Error::new(error_code, "pread() failed")),
            Err(error) => {
                ::syslog::error!("pread_chunk(): failed to convert error code (error={error:?})");
                return Err(Error::new(ErrorCode::TryAgain, "pread() failed"));
            },
        }
    }

    let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
    match message.header {
        LinuxDaemonMessageHeader::ReadResponse => {
            let response: ReadResponse = ReadResponse::from_bytes(message.payload);
            let count: i32 = response.count;

            if count < 0 {
                ::syslog::error!(
                    "pread_chunk(): linuxd returned negative count (fd={fd}, count={count})"
                );
                return Err(Error::new(
                    ErrorCode::InvalidMessage,
                    "read response count is negative",
                ));
            }

            if (count as usize) != bytes_pulled {
                ::syslog::error!(
                    "pread_chunk(): byte count mismatch (resp.count={count}, \
                     bytes_pulled={bytes_pulled})"
                );
                return Err(Error::new(
                    ErrorCode::InvalidMessage,
                    "read response count does not match bytes pulled",
                ));
            }

            Ok(count as c_size_t)
        },
        header => {
            ::syslog::error!(
                "pread_chunk(): failed to parse response (fd={fd}, chunk.len={}, offset={offset}, \
                 header={header:?})",
                chunk.len()
            );
            Err(Error::new(ErrorCode::TryAgain, "pread() failed"))
        },
    }
}

/// Forwards a `pread` request to linuxd via IPC, splitting the buffer into transport-sized
/// chunks.
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
        let chunk_size: usize = transfer_chunk_size(
            buffer[buffer_offset..].as_ptr() as usize,
            buffer.len() - buffer_offset,
        );
        let chunk: &mut [u8] = &mut buffer[buffer_offset..buffer_offset + chunk_size];
        let count: c_size_t = pread_chunk(tid, fd, chunk, offset + buffer_offset as off_t)?;

        if count == 0 {
            break;
        }

        total_read += count;
        buffer_offset += count as usize;

        if (count as usize) < chunk_size {
            break;
        }
    }

    Ok(total_read)
}
