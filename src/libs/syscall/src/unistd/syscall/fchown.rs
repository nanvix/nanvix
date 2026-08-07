// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::FileChownRequest,
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
use ::sysapi::sys_types::{
    gid_t,
    uid_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the owner and group of a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `owner`: Owner of the file.
/// - `group`: Group of the file.
///
/// # Returns
///
/// Upon successful completion, `fchown()` returns empty. Otherwise, it returns an error.
///
pub fn fchown(fd: RawFileDescriptor, owner: uid_t, group: gid_t) -> Result<(), Error> {
    ::syslog::trace!("fchown(): fd={:?}, owner={:?}, group={:?}", fd, owner, group);

    // Only VFS-backed descriptors are routable here.
    let backend_fd: RawFileDescriptor = {
        use crate::fdtable::{
            resolve_result,
            Route,
        };
        match resolve_result(fd)? {
            Some(res) if res.route == Route::Vfs => res.backend_fd,
            _ => {
                ::syslog::warn!("fchown(): bad file descriptor fd={fd}");
                return Err(Error::new(ErrorCode::BadFile, "fchown: fd is not a VFS fd"));
            },
        }
    };

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it
    let mut request: Message = FileChownRequest::build(
        tid,
        backend_fd,
        owner,
        group,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "fchown(): failed (fd={:?}, owner={:?}, group={:?}, status={:?})",
            fd,
            owner,
            group,
            { response.status }
        );

        match ErrorCode::try_from(response.status) {
            // System call failed, return error.
            Ok(error_code) => Err(Error::new(error_code, "system call failed")),
            // Invalid error code.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid error code received")),
        }
    } else {
        // System call succeeded, parse response.
        let message = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.kind() {
            // Response was successfully parsed.
            SystemCallMessageKind::FileChownResponse => Ok(()),
            // Invalid response.
            header => {
                ::syslog::warn!(
                    "fchown(): invalid response (fd={:?}, owner={:?}, group={:?}, header={:?})",
                    fd,
                    owner,
                    group,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
