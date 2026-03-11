// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::util::page_chunk_size;
use crate::{
    safe::RawFileDescriptor,
    unistd::message::{
        WriteRequest,
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
use ::sysapi::{
    sys_types::c_size_t,
    unistd::{
        STDERR_FILENO,
        STDOUT_FILENO,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes a single page-aligned chunk to a file descriptor via IKC. Sends a WriteRequest,
/// pushes the chunk data, and receives the WriteResponse.
///
/// # Parameters
///
/// - `tid`: Thread identifier of the calling thread.
/// - `fd`: File descriptor.
/// - `chunk`: Byte slice to write (must not cross a page boundary).
///
/// # Returns
///
/// Upon successful completion, the number of bytes written is returned. Otherwise, an
/// error is returned.
///
fn write_chunk(
    tid: ThreadIdentifier,
    fd: RawFileDescriptor,
    chunk: &[u8],
) -> Result<c_size_t, Error> {
    // Build metadata-only request and send it via IKC message.
    let empty_buf: [u8; WriteRequest::BUFFER_SIZE] = [0u8; WriteRequest::BUFFER_SIZE];
    let request: Message = WriteRequest::build(tid, fd, chunk.len() as c_size_t, empty_buf);
    ::sys::kcall::ipc::send(&request)?;

    // Push actual data to linuxd via data chunk transfer.
    ::sys::kcall::ipc::push(
        ::sys::pm::ProcessIdentifier::KERNEL,
        ::sys::pm::ThreadIdentifier::KERNEL,
        chunk,
    )?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "write_chunk(): failed (fd={:?}, chunk.len={:?}, error_code={:?})",
            fd,
            chunk.len(),
            { response.status }
        );

        match ErrorCode::try_from(response.status) {
            Ok(error_code) => return Err(Error::new(error_code, "write() failed")),
            Err(error) => {
                ::syslog::error!("write_chunk(): failed to convert error code (error={:?})", error);
                return Err(Error::new(ErrorCode::TryAgain, "write() failed"));
            },
        }
    }

    // Parse response.
    let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
    match message.header {
        LinuxDaemonMessageHeader::WriteResponse => {
            let response: WriteResponse = WriteResponse::from_bytes(message.payload);
            Ok(response.count as c_size_t)
        },
        header => {
            ::syslog::error!(
                "write_chunk(): failed to parse response (fd={:?}, chunk.len={:?}, header={:?})",
                fd,
                chunk.len(),
                header
            );
            Err(Error::new(ErrorCode::InvalidMessage, "failed to parse response"))
        },
    }
}

///
/// # Description
///
/// Writes data to a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to write.
///
/// # Returns
///
/// Upon successful completion, the `write()` system call returns the number of bytes written.
/// Otherwise, it returns an error.
///
pub fn write(fd: RawFileDescriptor, buffer: &[u8]) -> Result<c_size_t, Error> {
    // Skip logging for stdout and stderr to avoid spamming the output.
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        ::syslog::trace!("write(): fd={:?}, buffer.len={:?}", fd, buffer.len());
    }

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            let n: c_size_t = ::nvx::vfs::fd::vfs_write(fd, buffer).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("write(): VFS write failed (fd={fd}, error={e})");
                Error::new(code, "vfs write failed")
            })?;
            return Ok(n);
        }
        write_standalone(fd, buffer)
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    write_via_ikc(fd, buffer)
}

/// Standalone-mode write: routes stdout/stderr through IKC to the host-side I/O handler.
#[cfg(feature = "standalone")]
fn write_standalone(fd: RawFileDescriptor, buffer: &[u8]) -> Result<c_size_t, Error> {
    if fd == STDOUT_FILENO || fd == STDERR_FILENO {
        return write_via_ikc(fd, buffer);
    }
    Err(Error::new(ErrorCode::OperationNotSupported, "write not supported in standalone mode"))
}

/// Forwards a write request via IKC, splitting the buffer into page-aligned chunks.
fn write_via_ikc(fd: RawFileDescriptor, buffer: &[u8]) -> Result<c_size_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let mut total_written: c_size_t = 0;
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize =
            page_chunk_size(buffer[offset..].as_ptr() as usize, buffer.len() - offset);
        let chunk: &[u8] = &buffer[offset..offset + chunk_size];

        let written: c_size_t = write_chunk(tid, fd, chunk)?;
        total_written += written;
        offset += written as usize;

        // Short write: fewer bytes written than requested.
        if (written as usize) < chunk_size {
            break;
        }
    }

    Ok(total_written)
}
