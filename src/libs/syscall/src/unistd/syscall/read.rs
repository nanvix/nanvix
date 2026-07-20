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
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
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
/// Reads a single page-bounded chunk from a file descriptor via IKC. Sends a ReadRequest,
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
    destination: ProcessIdentifier,
    message_type: MessageType,
    pull_pid: ProcessIdentifier,
    pull_tid: ThreadIdentifier,
) -> Result<c_size_t, Error> {
    // Send metadata-only ReadRequest via IPC message.
    let request: Message =
        ReadRequest::build(tid, fd, chunk.len() as c_size_t, destination, message_type);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Pull data via data chunk transfer.
    let bytes_pulled: usize = ::sys::kcall::ipc::__kcall_pull(pull_pid, pull_tid, chunk)?;

    // Receive response metadata (count, status). The bulk data is already in the buffer.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "read_chunk(): failed (fd={:?}, chunk.len={:?}, error_code={:?})",
            fd,
            chunk.len(),
            { response.status }
        );

        match ErrorCode::try_from(response.status) {
            Ok(error_code) => return Err(Error::new(error_code, "read() failed")),
            Err(error) => {
                ::syslog::warn!(
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
    let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
    match message.header {
        SystemCallMessageHeader::ReadResponse => {
            let resp: ReadResponse = ReadResponse::from_bytes(message.payload);
            let count: i32 = resp.count;

            // Guard against a negative count that would wrap when cast to usize.
            if count < 0 {
                ::syslog::warn!(
                    "read_chunk(): backend returned negative count (fd={:?}, count={:?})",
                    fd,
                    count
                );
                return Err(Error::new(
                    ErrorCode::InvalidMessage,
                    "read response count is negative",
                ));
            }

            // Sanity-check: the number of bytes reported by the backend should match the bytes
            // actually pulled via the data chunk transfer.
            if (count as usize) != bytes_pulled {
                ::syslog::warn!(
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
            ::syslog::warn!(
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

    // Route by the descriptor's resolved backend so flat descriptors are dispatched through vfsd's
    // authoritative table.
    use crate::fdtable::{
        resolve,
        resolve_console,
        ConsoleLookup,
        Route,
    };
    match resolve_console(fd) {
        // When vfsd owns the console slot, use the flat descriptor so vfsd can apply the shared
        // terminal line discipline. In direct-ELF runs without vfsd, keep the historical direct
        // kernel-console path.
        ConsoleLookup::Console {
            backend_fd: STDIN_FILENO,
            via_vfsd,
        } => {
            if via_vfsd {
                read_ipc(
                    fd,
                    buffer,
                    crate::VFS_DESTINATION,
                    crate::VFS_MESSAGE_TYPE,
                    crate::VFS_PUSH_PULL_PID,
                    crate::VFS_PUSH_PULL_TID,
                )
            } else {
                read_ipc(
                    STDIN_FILENO,
                    buffer,
                    crate::HOST_IO,
                    MessageType::Ikc,
                    ProcessIdentifier::KERNEL,
                    ThreadIdentifier::KERNEL,
                )
            }
        },
        ConsoleLookup::Console { .. } | ConsoleLookup::BadFile => {
            ::syslog::warn!("read(): bad file descriptor fd={fd}");
            Err(Error::new(ErrorCode::BadFile, "read: bad file descriptor"))
        },
        ConsoleLookup::Other => match resolve(fd) {
            // VFS-backed descriptors go to vfsd.
            Some(res) if res.route == Route::Vfs => read_ipc(
                res.backend_fd,
                buffer,
                crate::VFS_DESTINATION,
                crate::VFS_MESSAGE_TYPE,
                crate::VFS_PUSH_PULL_PID,
                crate::VFS_PUSH_PULL_TID,
            ),
            // stdout/stderr, sockets, and unroutable descriptors are not readable here.
            _ => {
                ::syslog::warn!("read(): bad file descriptor fd={fd}");
                Err(Error::new(ErrorCode::BadFile, "read: bad file descriptor"))
            },
        },
    }
}

/// Forwards a `read` request via IPC, splitting the buffer into page-aligned chunks.
fn read_ipc(
    fd: RawFileDescriptor,
    buffer: &mut [u8],
    destination: ProcessIdentifier,
    message_type: MessageType,
    pull_pid: ProcessIdentifier,
    pull_tid: ThreadIdentifier,
) -> Result<c_size_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let mut total_read: c_size_t = 0;
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize =
            page_chunk_size(buffer[offset..].as_ptr() as usize, buffer.len() - offset);
        let chunk: &mut [u8] = &mut buffer[offset..offset + chunk_size];

        let count: c_size_t =
            read_chunk(tid, fd, chunk, destination, message_type, pull_pid, pull_tid)?;

        // EOF or zero-length read.
        if count == 0 {
            break;
        }

        total_read += count;
        offset += count as usize;

        // A short reply ends the read. Because the chunk never spans more than one page (see
        // `page_chunk_size`) and the IKC backends deliver up to a full page per request, receiving
        // fewer bytes than requested means the input is exhausted: end-of-file for a regular file,
        // or no more bytes currently available for a stream. Continuing here would truncate neither
        // a multi-page file (the next page is fetched on the following iteration) nor block a
        // partially-filled stream. Do not switch this loop to multi-page chunks without revisiting
        // this guard: a capped multi-page reply is indistinguishable from genuine end-of-input.
        if (count as usize) < chunk_size {
            break;
        }
    }

    Ok(total_read)
}
