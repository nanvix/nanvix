// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::stat::message::{
        FileCreationMaskRequest,
        FileCreationMaskResponse,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::sys_types::mode_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the calling process's file mode creation mask.
///
/// # Parameters
///
/// - `mask`: New file mode creation mask.
///
/// # Returns
///
/// Upon success, the previous file mode creation mask is returned. Otherwise, an error is returned.
///
pub fn umask(mask: mode_t) -> Result<mode_t, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    let request: Message =
        FileCreationMaskRequest::build(tid, mask, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    let response: Message = ::sys::kcall::ipc::__kcall_recv_response()?;
    if response.status != 0 {
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        return Err(Error::new(error_code, "umask() failed"));
    }

    let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
    let header: SystemCallMessageHeader = message.header;
    if header != SystemCallMessageHeader::FileCreationMaskResponse {
        return Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header"));
    }

    let response: FileCreationMaskResponse = FileCreationMaskResponse::from_bytes(message.payload);
    Ok(response.mask)
}
