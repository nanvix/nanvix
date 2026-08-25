// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    cancel::cancel_pipe_operation,
    util::sg_chunk_size,
};
use crate::{
    poll::input_message::PipeOperation,
    safe::RawFileDescriptor,
    unistd::message::{
        WriteRequest,
        WriteResponse,
    },
    SystemCallMessage,
    SystemCallMessageKind,
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
    unistd::{
        STDERR_FILENO,
        STDOUT_FILENO,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Backend routing and interruption policy for one write operation.
#[derive(Clone, Copy)]
struct WriteBackend {
    destination: ProcessIdentifier,
    message_type: MessageType,
    push_pid: ProcessIdentifier,
    push_tid: ThreadIdentifier,
    cancel_pipe_on_interrupt: bool,
}

///
/// # Description
///
/// Writes a single scatter/gather chunk to a file descriptor via IKC. Sends a WriteRequest,
/// pushes the chunk data, and receives the WriteResponse.
///
/// # Parameters
///
/// - `tid`: Thread identifier of the calling thread.
/// - `fd`: File descriptor.
/// - `chunk`: Byte slice to write.
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
    backend: WriteBackend,
) -> Result<c_size_t, Error> {
    // Build metadata-only request and send it via IPC message.
    let empty_buf: [u8; WriteRequest::BUFFER_SIZE] = [0u8; WriteRequest::BUFFER_SIZE];
    let mut request: Message = WriteRequest::build(
        tid,
        fd,
        chunk.len() as c_size_t,
        empty_buf,
        backend.destination,
        backend.message_type,
    );
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Push actual data via data chunk transfer.
    ::sys::kcall::ipc::__kcall_push(backend.push_pid, backend.push_tid, chunk)?;

    // Receive response.
    let response: Message = match crate::rpc::recv_response_interruptible(&token) {
        Ok(response) => response,
        Err(error) if error.code == ErrorCode::Interrupted && backend.cancel_pipe_on_interrupt => {
            match cancel_pipe_operation(tid, fd, PipeOperation::Write, token.identifier())? {
                Some(_transferred) => return Err(error),
                None => crate::rpc::recv_response(&token)?,
            }
        },
        Err(error) => return Err(error),
    };

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "write_chunk(): failed (fd={:?}, chunk.len={:?}, error_code={:?})",
            fd,
            chunk.len(),
            { response.status }
        );

        match ErrorCode::try_from(response.status) {
            Ok(error_code) => return Err(Error::new(error_code, "write() failed")),
            Err(error) => {
                ::syslog::warn!("write_chunk(): failed to convert error code (error={:?})", error);
                return Err(Error::new(ErrorCode::TryAgain, "write() failed"));
            },
        }
    }

    // Parse response.
    let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
    match message.kind() {
        SystemCallMessageKind::WriteResponse => {
            let response: WriteResponse = WriteResponse::from_bytes(message.payload);
            let count: i32 = response.count;

            if count < 0 {
                ::syslog::warn!(
                    "write_chunk(): backend returned negative count (fd={:?}, count={:?})",
                    fd,
                    count
                );
                return Err(Error::new(
                    ErrorCode::InvalidMessage,
                    "write response count is negative",
                ));
            }

            if (count as usize) > chunk.len() {
                ::syslog::warn!(
                    "write_chunk(): backend returned oversized count (fd={:?}, count={:?}, \
                     chunk.len={:?})",
                    fd,
                    count,
                    chunk.len()
                );
                return Err(Error::new(
                    ErrorCode::InvalidMessage,
                    "write response count exceeds requested chunk length",
                ));
            }

            Ok(response.count as c_size_t)
        },
        header => {
            ::syslog::warn!(
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

    // Route by the descriptor's resolved backend so flat descriptors are dispatched through vfsd's
    // authoritative table.
    use crate::fdtable::{
        resolve_result,
        Route,
    };
    match resolve_result(fd)? {
        // stdout/stderr writes flow directly to the kernel over IKC.
        Some(res)
            if res.route == Route::Console
                && (res.backend_fd == STDOUT_FILENO || res.backend_fd == STDERR_FILENO) =>
        {
            if !buffer.is_empty() {
                notify_terminal_write();
            }
            write_ipc(
                res.backend_fd,
                buffer,
                WriteBackend {
                    destination: crate::HOST_IO,
                    message_type: MessageType::Ikc,
                    push_pid: ProcessIdentifier::KERNEL,
                    push_tid: ThreadIdentifier::KERNEL,
                    cancel_pipe_on_interrupt: false,
                },
            )
        },
        // VFS-backed descriptors go to vfsd.
        Some(res) if res.route == Route::Vfs => write_ipc(
            res.backend_fd,
            buffer,
            WriteBackend {
                destination: crate::VFS_DESTINATION,
                message_type: crate::VFS_MESSAGE_TYPE,
                push_pid: crate::VFS_PUSH_PULL_PID,
                push_tid: crate::VFS_PUSH_PULL_TID,
                cancel_pipe_on_interrupt: true,
            },
        ),
        // stdin, sockets, and unroutable descriptors are not writable here.
        _ => {
            ::syslog::warn!("write(): bad file descriptor fd={fd}");
            Err(Error::new(ErrorCode::BadFile, "write: fd is not a VFS fd"))
        },
    }
}

/// Notifies the process manager daemon that the calling process wrote to the console.
///
/// Console output bypasses vfsd and flows directly to the kernel, so this self-report is the only
/// path that lets job control raise `SIGTTOU` for background stdout/stderr writes.
fn notify_terminal_write() {
    let caller: ProcessIdentifier = match ::sys::kcall::pm::getpid() {
        Ok(pid) => pid,
        Err(error) => {
            ::syslog::warn!(
                "write(): failed to get caller pid for terminal access (error={:?})",
                error
            );
            return;
        },
    };

    // Single-binary test workloads run as pid 1, which aliases `PROCD` but has no separate process
    // manager daemon to receive this fire-and-forget message. The fixed daemon pid range is not a
    // job-control subject, so skip those writers.
    if i32::from(caller) <= ProcessIdentifier::VFSD_RAW {
        return;
    }

    match ::proc::terminal_access_request(caller, caller, true) {
        Ok(message) => {
            if let Err(error) = ::sys::kcall::ipc::__kcall_send(&message) {
                ::syslog::warn!(
                    "write(): failed to notify terminal write (pid={:?}, error={:?})",
                    caller,
                    error
                );
            }
        },
        Err(error) => {
            ::syslog::warn!(
                "write(): failed to build terminal-write notification (error={:?})",
                error
            );
        },
    }
}

/// Forwards a write request via IPC, splitting the buffer into scatter/gather chunks.
fn write_ipc(
    fd: RawFileDescriptor,
    buffer: &[u8],
    backend: WriteBackend,
) -> Result<c_size_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let mut total_written: c_size_t = 0;
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let chunk_size: usize =
            sg_chunk_size(buffer[offset..].as_ptr() as usize, buffer.len() - offset);
        let chunk: &[u8] = &buffer[offset..offset + chunk_size];

        let written: c_size_t = write_chunk(tid, fd, chunk, backend)?;
        total_written += written;
        offset += written as usize;

        // Stop only when the backend makes no forward progress. A backend may accept fewer bytes
        // than requested per request — notably hostfs caps each IKC write at
        // `MAX_INLINE_WRITE_DATA`, which shrank when the IPC payload gave up 8 bytes to the
        // kernel-stamped client identity (nanvix/nanvix#2662) — so a short write is not
        // end-of-stream: the next iteration writes the remainder and the caller still observes a
        // full write. Breaking on `written == 0` keeps the guard against an unbounded loop when no
        // forward progress is possible.
        if written == 0 {
            break;
        }
    }

    Ok(total_written)
}
