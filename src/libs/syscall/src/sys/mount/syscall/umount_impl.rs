// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    sys::mount::message::UmountRequest,
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Unmounts the filesystem at the given target path.
///
/// # Parameters
///
/// - `target`: Target mount point to unmount.
///
/// # Returns
///
/// Upon success, returns `Ok(())`. Otherwise, returns an error.
pub fn umount(target: &str) -> Result<(), Error> {
    ::syslog::trace!("umount(): target={:?}", target);

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let request: UmountRequest = UmountRequest::new(target.to_string())?;

    let requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;

    // Send request parts.
    for request in &requests {
        ::sys::kcall::ipc::__kcall_send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv_response()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!("umount(): failed (target={:?}, error_code={:?})", target, {
            response.status
        });
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => Err(Error::new(error_code, "umount() failed")),
            Err(error) => {
                ::syslog::warn!("umount(): failed to parse error code (error={:?})", error);
                Err(Error::new(ErrorCode::TryAgain, "umount(): failed"))
            },
        }
    } else {
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        let header: SystemCallMessageHeader = message.header;
        if header != SystemCallMessageHeader::HostUmountResponse {
            return Err(Error::new(ErrorCode::InvalidMessage, "unexpected response header"));
        }
        Ok(())
    }
}
