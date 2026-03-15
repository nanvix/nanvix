// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::util::page_chunk_size;
use crate::{
    safe::RawFileDescriptor,
    unistd::message::{
        ReadRequest,
        ReadResponse,
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
    unistd::STDIN_FILENO,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads a single page-aligned chunk from a file descriptor via IKC. Sends a ReadRequest,
/// pulls the chunk data, and receives the ReadResponse.
///
/// # Parameters
///
/// - `tid`: Thread identifier of the calling thread.
/// - `fd`: File descriptor.
/// - `chunk`: Mutable byte slice to read into (must not cross a page boundary).
///
/// # Returns
///
/// Upon successful completion, the number of bytes read is returned. Otherwise, an
/// error is returned.
///
fn read_chunk(
    tid: ThreadIdentifier,
    fd: RawFileDescriptor,
    chunk: &mut [u8],
) -> Result<c_size_t, Error> {
    // Send metadata-only ReadRequest via IKC message.
    let request: Message = ReadRequest::build(tid, fd, chunk.len() as c_size_t);
    ::sys::kcall::ipc::send(&request)?;

    // Pull data from linuxd via data chunk transfer.
    let bytes_pulled: usize = ::sys::kcall::ipc::pull(
        ::sys::pm::ProcessIdentifier::KERNEL,
        ::sys::pm::ThreadIdentifier::KERNEL,
        chunk,
    )?;

    // Receive response metadata (count, status). The bulk data is already in the buffer.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "read_chunk(): failed (fd={:?}, chunk.len={:?}, error_code={:?})",
            fd,
            chunk.len(),
            { response.status }
        );

        match ErrorCode::try_from(response.status) {
            Ok(error_code) => return Err(Error::new(error_code, "read() failed")),
            Err(error) => {
                ::syslog::error!(
                    "read_chunk(): failed (fd={:?}, chunk.len={:?}, error_code={:?})",
                    fd,
                    chunk.len(),
                    error
                );
                return Err(Error::new(ErrorCode::TryAgain, "read() failed"));
            },
        }
    }

    // Parse response.
    let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
    match message.header {
        LinuxDaemonMessageHeader::ReadResponse => {
            let resp: ReadResponse = ReadResponse::from_bytes(message.payload);
            let count: i32 = resp.count;

            // Guard against a negative count that would wrap when cast to usize.
            if count < 0 {
                ::syslog::error!(
                    "read_chunk(): linuxd returned negative count (fd={:?}, count={:?})",
                    fd,
                    count
                );
                return Err(Error::new(
                    ErrorCode::InvalidMessage,
                    "read response count is negative",
                ));
            }

            // Sanity-check: the number of bytes reported by linuxd should match the bytes
            // actually pulled via the data chunk transfer.
            if (count as usize) != bytes_pulled {
                ::syslog::error!(
                    "read_chunk(): byte count mismatch (resp.count={:?}, bytes_pulled={:?})",
                    count,
                    bytes_pulled
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
                "read_chunk(): failed to parse response (fd={:?}, chunk.len={:?}, header={:?})",
                fd,
                chunk.len(),
                header
            );
            Err(Error::new(ErrorCode::InvalidMessage, "read() failed"))
        },
    }
}

///
/// # Description
///
/// Reads data from a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to read into.
///
/// # Returns
///
/// Upon successful completion, `read()` returns the number of bytes read. Otherwise, it returns an
/// error.
///
pub fn read(fd: RawFileDescriptor, buffer: &mut [u8]) -> Result<c_size_t, Error> {
    // Skip logging for stdin to avoid spamming the output.
    if fd != STDIN_FILENO {
        ::syslog::trace!("read(): fd={:?}, buffer.len={:?}", fd, buffer.len());
    }

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            let n: c_size_t = ::nvx::vfs::fd::vfs_read(fd, buffer).map_err(|e| {
                let code: ErrorCode = e.into();
                ::syslog::error!("read(): VFS read failed (fd={fd}, error={e})");
                Error::new(code, "vfs read failed")
            })?;
            return Ok(n);
        }
        if fd == STDIN_FILENO {
            return read_via_ikc(fd, buffer);
        }
        Err(Error::new(ErrorCode::BadFile, "read: fd is not a VFS fd in standalone mode"))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    read_via_ikc(fd, buffer)
}

/// Forwards a `read` request via IKC, splitting the buffer into page-aligned chunks.
fn read_via_ikc(fd: RawFileDescriptor, buffer: &mut [u8]) -> Result<c_size_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let mut total_read: c_size_t = 0;
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize =
            page_chunk_size(buffer[offset..].as_ptr() as usize, buffer.len() - offset);
        let chunk: &mut [u8] = &mut buffer[offset..offset + chunk_size];

        let count: c_size_t = read_chunk(tid, fd, chunk)?;

        // EOF or zero-length read.
        if count == 0 {
            break;
        }

        total_read += count;
        offset += count as usize;

        // Short read: fewer bytes returned than the chunk size.
        if (count as usize) < chunk_size {
            break;
        }
    }

    Ok(total_read)
}
