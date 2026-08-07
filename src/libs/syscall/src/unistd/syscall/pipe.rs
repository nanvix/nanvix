// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    unistd::message::{
        PipeRequest,
        PipeResponse,
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
        RequestToken,
    },
    pm::ThreadIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn pipe() -> Result<[i32; 2], Error> {
    ::syslog::trace!("pipe()");

    pipe_vfsd()
}

/// Sends a `pipe` request to vfsd via IPC and parses the response.
///
/// Mirrors the short-syscall convention used by `close`: send the request, then receive the reply.
/// vfsd allocates the shared pipe buffer and the two descriptors and returns them.
fn pipe_vfsd() -> Result<[i32; 2], Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it to vfsd over local IPC.
    let mut request: Message =
        PipeRequest::build(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;
    parse_pipe_response(response)
}

/// Parses a `pipe` response, mapping a non-zero status onto an error code and otherwise extracting
/// the read/write descriptors from the [`PipeResponse`].
fn parse_pipe_response(response: Message) -> Result<[i32; 2], Error> {
    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::warn!("pipe(): failed (error={})", error_code);
        Err(Error::new(error_code, "pipe() failed"))
    } else {
        // System call succeeded, parse response.
        match SystemCallMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.kind() {
                // Response was successfully parsed.
                SystemCallMessageKind::PipeResponse => {
                    // Parse response.
                    let response: PipeResponse = PipeResponse::from_bytes(message.payload);

                    let read_fd: i32 = response.read_fd;
                    let write_fd: i32 = response.write_fd;
                    ::syslog::trace!("pipe(): read_fd={read_fd:?}, write_fd={write_fd:?}");

                    // Seed the resolution cache so both pipe ends resolve from the cache rather than
                    // by number. They are vfsd-served descriptors carrying the table generation they
                    // were allocated at.
                    // `PipeResponse` is `#[repr(C, packed)]`, so read `epoch` through a raw
                    // pointer to avoid forming an unaligned reference.
                    let epoch: u64 =
                        unsafe { ::core::ptr::addr_of!(response.epoch).read_unaligned() };
                    if read_fd >= 0 {
                        crate::fdtable::record(read_fd, crate::fdtable::Route::Vfs, read_fd, epoch);
                    }
                    if write_fd >= 0 {
                        crate::fdtable::record(
                            write_fd,
                            crate::fdtable::Route::Vfs,
                            write_fd,
                            epoch,
                        );
                    }

                    Ok([read_fd, write_fd])
                },
                // Response was not successfully parsed.
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            // Response was not successfully parsed.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
        }
    }
}

pub mod bindings {
    use crate::errno::__errno_location;
    use ::sysapi::ffi::c_int;
    use ::syslog::trace_syscall;

    ///
    /// # Description
    ///
    /// Creates a pipe.
    ///
    /// # Parameters
    ///
    /// - `fds`: Array to store the file descriptors of the pipe.
    ///
    /// # Returns
    ///
    /// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
    /// indicate the error.
    ///
    #[unsafe(no_mangle)]
    #[trace_syscall]
    pub unsafe extern "C" fn pipe(fds: *mut c_int) -> c_int {
        match super::pipe() {
            Ok([read_fd, write_fd]) => {
                ::syslog::trace!("pipe(): read_fd={read_fd:?}, write_fd={write_fd:?}");
                unsafe {
                    *fds.offset(0) = read_fd;
                    *fds.offset(1) = write_fd;
                }
                0
            },
            Err(error) => {
                ::syslog::warn!("pipe(): failed (error={error:?})");
                unsafe {
                    *__errno_location() = error.code.get();
                }
                -1
            },
        }
    }
}
