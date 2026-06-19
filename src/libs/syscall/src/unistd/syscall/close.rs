// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    unistd::message::{
        CloseRequest,
        CloseResponse,
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

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn close(fd: i32) -> Result<(), Error> {
    // In standalone mode, route based on the descriptor's resolved backend.
    #[cfg(feature = "standalone")]
    {
        use crate::fdtable::{
            resolve,
            Route,
        };
        let result: Result<(), Error> = match resolve(fd) {
            Some(res) => match res.route {
                // VFS-backed descriptors are closed by vfsd.
                Route::Vfs => {
                    close_ipc(res.backend_fd, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)
                },
                // Console descriptors still occupy slots in vfsd's flat table; closing one must
                // release that slot so the descriptor number can be reused.
                Route::Console => {
                    close_ipc(res.backend_fd, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)
                },
                // Sockets are closed by networkd.
                Route::Socket => {
                    close_ipc(res.backend_fd, crate::NETWORK_DESTINATION, MessageType::Ikc)
                },
            },
            // Unknown fd: no handler available.
            None => {
                ::syslog::warn!("close(): bad file descriptor fd={fd}");
                Err(Error::new(ErrorCode::BadFile, "bad file descriptor"))
            },
        };
        // Drop any cached resolution once the descriptor is gone, so a number later reused by a
        // different backend is never answered from a stale entry. Only a successful close frees the
        // descriptor; a failed close leaves it open, so its resolution stays valid.
        if result.is_ok() {
            crate::fdtable::invalidate(fd);
        }
        result
    }

    #[cfg(not(feature = "standalone"))]
    {
        close_ipc(fd, crate::LINUXD, MessageType::Ikc)
    }
}

/// Forwards a `close` request via IPC to the given destination.
fn close_ipc(
    fd: i32,
    destination: ProcessIdentifier,
    message_type: MessageType,
) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: Message = CloseRequest::build(tid, fd, destination, message_type);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::warn!("close(): failed (error={})", error_code);
        Err(Error::new(error_code, "close() failed"))
    } else {
        match SystemCallMessage::try_from_bytes(response.payload) {
            Ok(message) => match message.header {
                SystemCallMessageHeader::CloseResponse => {
                    let _: CloseResponse = CloseResponse::from_bytes(message.payload);
                    Ok(())
                },
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
        }
    }
}

pub mod bindings {
    use crate::errno::__errno_location;
    use ::sysapi::ffi::c_int;
    use ::syslog::trace_syscall;

    #[unsafe(no_mangle)]
    #[trace_syscall]
    pub extern "C" fn close(fd: c_int) -> c_int {
        match crate::unistd::close(fd) {
            Ok(()) => 0,
            Err(error) => {
                ::syslog::warn!("close(): failed ({:?})", error);
                unsafe {
                    *__errno_location() = error.code.get();
                }
                -1
            },
        }
    }
}
