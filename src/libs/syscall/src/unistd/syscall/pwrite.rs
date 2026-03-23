// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::{
        PositionedWriteRequest,
        WriteResponse,
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
use ::sysapi::sys_types::{
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

    // Route to the VFS if this is a VFS file descriptor.
    #[cfg(feature = "memfs")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_pwrite(fd, buffer, offset).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("pwrite(): VFS pwrite failed (fd={fd}, error={e})");
                Error::new(code, "vfs pwrite failed")
            });
        }
    }

    // In standalone mode, reject non-VFS fds (no linuxd).
    #[cfg(feature = "standalone")]
    {
        let _ = (fd, buffer, offset);
        return Err(Error::new(
            ErrorCode::OperationNotSupported,
            "pwrite not available in standalone mode",
        ));
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    pwrite_linuxd(fd, buffer, offset)
}

/// Writes a single page-aligned chunk to a file descriptor via linuxd.
#[cfg(not(feature = "standalone"))]
fn pwrite_chunk(
    tid: ThreadIdentifier,
    fd: RawFileDescriptor,
    chunk: &[u8],
    offset: off_t,
) -> Result<c_size_t, Error> {
    let request: Message = PositionedWriteRequest::build(tid, fd, chunk.len() as c_size_t, offset);
    ::sys::kcall::ipc::send(&request)?;

    ::sys::kcall::ipc::push(
        ::sys::pm::ProcessIdentifier::KERNEL,
        ::sys::pm::ThreadIdentifier::KERNEL,
        chunk,
    )?;

    let response: Message = ::sys::kcall::ipc::recv()?;

    if response.status != 0 {
        ::syslog::error!(
            "pwrite_chunk(): failed (fd={fd}, chunk.len={}, error_code={})",
            chunk.len(),
            { response.status }
        );

        match ErrorCode::try_from(response.status) {
            Ok(error_code) => return Err(Error::new(error_code, "pwrite() failed")),
            Err(error) => {
                ::syslog::error!("pwrite_chunk(): failed to convert error code (error={error:?})");
                return Err(Error::new(ErrorCode::TryAgain, "pwrite() failed"));
            },
        }
    }

    let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
    match message.header {
        LinuxDaemonMessageHeader::WriteResponse => {
            let response: WriteResponse = WriteResponse::from_bytes(message.payload);
            Ok(response.count as c_size_t)
        },
        header => {
            ::syslog::error!(
                "pwrite_chunk(): failed to parse response (fd={fd}, chunk.len={}, \
                 header={header:?})",
                chunk.len()
            );
            Err(Error::new(ErrorCode::InvalidMessage, "failed to parse response"))
        },
    }
}

/// Forwards a `pwrite` request to linuxd via IPC, splitting the buffer into transport-sized
/// chunks.
#[cfg(not(feature = "standalone"))]
fn pwrite_linuxd(fd: RawFileDescriptor, buffer: &[u8], offset: off_t) -> Result<c_size_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;
    let mut total_written: c_size_t = 0;
    let mut buffer_offset: usize = 0;

    while buffer_offset < buffer.len() {
        let chunk_size: usize = transfer_chunk_size(
            buffer[buffer_offset..].as_ptr() as usize,
            buffer.len() - buffer_offset,
        );
        let chunk: &[u8] = &buffer[buffer_offset..buffer_offset + chunk_size];
        let written: c_size_t = pwrite_chunk(tid, fd, chunk, offset + buffer_offset as off_t)?;
        total_written += written;
        buffer_offset += written as usize;

        if (written as usize) < chunk_size {
            break;
        }
    }

    Ok(total_written)
}
