// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    self,
    GetParentResponseMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageType,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Queries the process manager daemon for the parent of the calling process (`getppid()`).
///
/// # Returns
///
/// Upon successful completion, the process identifier of the parent is returned. Upon failure, an
/// error is returned instead.
///
pub fn get_parent() -> Result<ProcessIdentifier, Error> {
    // Retrieve process identifier of the calling process.
    let pid: ProcessIdentifier = ::sys::kcall::pm::__kcall_getpid()?;

    // Build get-parent message and send it.
    let message: Message = message::get_parent_request(pid)?;
    ::sys::kcall::ipc::__kcall_send(&message)?;

    // Wait response from the process manager daemon.
    let message: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Parse response.
    match message.message_type {
        MessageType::Ipc => {
            let message: SystemMessage = SystemMessage::from_bytes(message.payload)?;

            // Parse message.
            match message.header {
                // Parse process management message.
                SystemMessageHeader::ProcessManagement => {
                    let message: ProcessManagementMessage =
                        ProcessManagementMessage::from_bytes(message.payload)?;

                    // Parse operation.
                    match message.header {
                        ProcessManagementMessageHeader::GetParentResponse => {
                            let message: GetParentResponseMessage =
                                GetParentResponseMessage::from_bytes(message.payload);

                            match message.status {
                                0 => Ok(message.parent),
                                _ => Err(Error::new(
                                    ErrorCode::try_from(message.status)?,
                                    "failed to query parent process",
                                )),
                            }
                        },
                        _ => Err(Error::new(
                            ErrorCode::InvalidMessage,
                            "unexpected process management message",
                        )),
                    }
                },
                _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid system message type")),
            }
        },
        _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid message type")),
    }
}
