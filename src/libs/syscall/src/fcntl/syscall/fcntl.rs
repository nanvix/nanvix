// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::{
        FileControlRequest,
        FileControlResponse,
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
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn fcntl(fd: i32, cmd: i32, arg: Option<c_int>) -> Result<c_int, Error> {
    ::syslog::trace!("fcntl(): fd={:?}, cmd={:?}, arg={:?}", fd, cmd, arg);
    // Flag queries and the F_DUPFD duplication family are served by vfsd on the slot it owns, so
    // they are addressed by the flat descriptor — for console descriptors too, whose slot vfsd owns
    // even though their I/O is routed to the kernel by stream number.
    let backend_fd: i32 = crate::fdtable::resolve_table_op(fd, "fcntl")?;
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let mut request: Message = FileControlRequest::build(
        tid,
        backend_fd,
        cmd,
        arg.unwrap_or(0),
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "fcntl(): failed (fd={:?}, cmd={:?}, arg={:?}, status={:?})",
            fd,
            cmd,
            arg,
            { response.status }
        );

        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error code.
                Err(Error::new(error_code, "fcntl() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::syslog::warn!(
                    "fcntl(): failed to parse error code (fd={:?}, cmd={:?}, arg={:?}, error={:?})",
                    fd,
                    cmd,
                    arg,
                    error
                );
                // Return error code.
                Err(Error::new(ErrorCode::TryAgain, "fcntl() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.kind() {
            // Response was successfully parsed.
            SystemCallMessageKind::FileControlResponse => {
                let message: FileControlResponse = FileControlResponse::from_bytes(message.payload);
                let ret: c_int = message.ret;
                // A duplication command (`F_DUPFD` and its close-on-exec/close-on-fork variants)
                // returns a freshly allocated descriptor. Drop any cached resolution for that
                // number so its first use re-resolves against vfsd's table rather than answering
                // from an entry that described the number's previous occupant.
                if is_dup_command(cmd) && ret >= 0 {
                    crate::fdtable::invalidate(ret);
                }
                Ok(ret)
            },
            // Response was not successfully parsed.
            header => {
                ::syslog::warn!(
                    "fcntl(): invalid response (fd={:?}, cmd={:?}, arg={:?}, header={:?})",
                    fd,
                    cmd,
                    arg,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "fcntl() failed"))
            },
        }
    }
}

/// Returns whether `cmd` is one of the descriptor-duplication `fcntl` commands (`F_DUPFD` and its
/// close-on-exec / close-on-fork variants), which allocate a new descriptor rather than querying or
/// setting a flag.
fn is_dup_command(cmd: i32) -> bool {
    use ::sysapi::fcntl::file_control_request::{
        F_DUPFD,
        F_DUPFD_CLOEXEC,
        F_DUPFD_CLOFORK,
    };
    matches!(cmd, F_DUPFD | F_DUPFD_CLOEXEC | F_DUPFD_CLOFORK)
}
