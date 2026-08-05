// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    cancel::cancel_pipe_operation,
    util::page_chunk_size,
};
use crate::{
    poll::input_message::{
        ConsoleReadCancel,
        PipeOperation,
    },
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
        RequestToken,
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

/// Backend routing and interruption policy for one read operation.
#[derive(Clone, Copy)]
struct ReadBackend {
    destination: ProcessIdentifier,
    message_type: MessageType,
    pull_pid: ProcessIdentifier,
    pull_tid: ThreadIdentifier,
    cancellation: ReadCancellation,
}

/// Cancellation protocol to run when a read's bulk pull is interrupted.
#[derive(Clone, Copy)]
enum ReadCancellation {
    None,
    Console,
    Pipe,
}

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
    backend: ReadBackend,
) -> Result<c_size_t, Error> {
    // Send metadata-only ReadRequest via IPC message.
    let mut request: Message = ReadRequest::build(
        tid,
        fd,
        chunk.len() as c_size_t,
        backend.destination,
        backend.message_type,
    );
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Pull data via data chunk transfer.
    let mut interrupted: Option<Error> = None;
    let bytes_pulled: Option<usize> =
        match ::sys::kcall::ipc::__kcall_pull(backend.pull_pid, backend.pull_tid, chunk) {
            Ok(bytes_pulled) => Some(bytes_pulled),
            Err(error) if error.code == ErrorCode::Interrupted => {
                let cancelled: bool = match backend.cancellation {
                    ReadCancellation::None => return Err(error),
                    ReadCancellation::Console => cancel_console_read(tid, token.identifier())?,
                    ReadCancellation::Pipe => {
                        cancel_pipe_operation(tid, fd, PipeOperation::Read, token.identifier())?
                            .is_some()
                    },
                };
                if cancelled {
                    return Err(error);
                }
                interrupted = Some(error);
                None
            },
            Err(error) => return Err(error),
        };

    // Receive response metadata (count, status). Once the bulk transfer completed, always drain the
    // matching response so a caught signal cannot leave stale metadata in this thread's mailbox.
    let response: Message = loop {
        match crate::rpc::recv_response_interruptible(&token) {
            Ok(response) => break response,
            Err(error) if error.code == ErrorCode::Interrupted => {
                interrupted.get_or_insert(error);
            },
            Err(error) => return Err(error),
        }
    };

    // Check whether system call succeeded or not.
    if response.status != 0 {
        if let Some(error) = interrupted {
            return Err(error);
        }
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
            // actually pulled via the data chunk transfer. When the transfer never completed, a
            // positive count would report bytes that never reached the caller's buffer.
            match bytes_pulled {
                Some(bytes_pulled) if (count as usize) != bytes_pulled => {
                    ::syslog::warn!(
                        "read_chunk(): byte count mismatch (resp.count={:?}, bytes_pulled={:?})",
                        count,
                        bytes_pulled
                    );
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "read response count does not match bytes pulled",
                    ));
                },
                None if count != 0 => {
                    ::syslog::warn!(
                        "read_chunk(): response reports data without a completed transfer \
                         (resp.count={:?})",
                        count
                    );
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "read response reports data that was never transferred",
                    ));
                },
                _ => {},
            }

            if count == 0 {
                if let Some(error) = interrupted {
                    return Err(error);
                }
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

/// Cancels this thread's parked VFSD console read and drains the acknowledgement.
fn cancel_console_read(
    tid: ThreadIdentifier,
    request_id: ::sys::ipc::RequestIdentifier,
) -> Result<bool, Error> {
    let mut request: Message = ConsoleReadCancel::build_request(tid, request_id);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    loop {
        let response: Message = match crate::rpc::recv_response_interruptible(&token) {
            Ok(response) => response,
            Err(error) if error.code == ErrorCode::Interrupted => continue,
            Err(error) => return Err(error),
        };
        let source: ::sys::ipc::MessageSender = response.source;
        if source.pid != ProcessIdentifier::VFSD {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "console read cancellation returned an invalid sender",
            ));
        }
        if response.status != 0 {
            return Err(Error::new(
                ErrorCode::try_from(response.status)?,
                "console read cancellation failed",
            ));
        }
        let response: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        let header: SystemCallMessageHeader = response.header;
        if header != SystemCallMessageHeader::ConsoleReadCancelResponse {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "unexpected console read cancellation response",
            ));
        }
        return Ok(ConsoleReadCancel::cancelled(&response.payload));
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
        resolve_console,
        resolve_result,
        ConsoleLookup,
        Route,
    };
    match resolve_console(fd)? {
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
                    ReadBackend {
                        destination: crate::VFS_DESTINATION,
                        message_type: crate::VFS_MESSAGE_TYPE,
                        pull_pid: crate::VFS_PUSH_PULL_PID,
                        pull_tid: crate::VFS_PUSH_PULL_TID,
                        cancellation: ReadCancellation::Console,
                    },
                )
            } else {
                read_ipc(
                    STDIN_FILENO,
                    buffer,
                    ReadBackend {
                        destination: crate::HOST_IO,
                        message_type: MessageType::Ikc,
                        pull_pid: ProcessIdentifier::KERNEL,
                        pull_tid: ThreadIdentifier::KERNEL,
                        cancellation: ReadCancellation::None,
                    },
                )
            }
        },
        ConsoleLookup::Console { .. } | ConsoleLookup::BadFile => {
            ::syslog::warn!("read(): bad file descriptor fd={fd}");
            Err(Error::new(ErrorCode::BadFile, "read: bad file descriptor"))
        },
        ConsoleLookup::Other => match resolve_result(fd)? {
            // VFS-backed descriptors go to vfsd.
            Some(res) if res.route == Route::Vfs => read_ipc(
                res.backend_fd,
                buffer,
                ReadBackend {
                    destination: crate::VFS_DESTINATION,
                    message_type: crate::VFS_MESSAGE_TYPE,
                    pull_pid: crate::VFS_PUSH_PULL_PID,
                    pull_tid: crate::VFS_PUSH_PULL_TID,
                    cancellation: ReadCancellation::Pipe,
                },
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
    backend: ReadBackend,
) -> Result<c_size_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let mut total_read: c_size_t = 0;
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize =
            page_chunk_size(buffer[offset..].as_ptr() as usize, buffer.len() - offset);
        let chunk: &mut [u8] = &mut buffer[offset..offset + chunk_size];

        let count: c_size_t = read_chunk(tid, fd, chunk, backend)?;

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
