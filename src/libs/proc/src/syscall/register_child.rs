// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    self,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    RegisterChildResponseMessage,
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
/// Registers a parent/child relationship with the process manager daemon. This is invoked by a
/// parent process right after a successful `fork()`.
///
/// # Parameters
///
/// - `child`: Process identifier of the child.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn register_child(child: ProcessIdentifier) -> Result<(), Error> {
    // Retrieve process identifier of the calling process (the parent).
    let parent: ProcessIdentifier = ::sys::kcall::pm::__kcall_getpid()?;

    // Build register-child message and send it.
    let message: Message = message::register_child_request(child, parent)?;
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
                        ProcessManagementMessageHeader::RegisterChildResponse => {
                            let message: RegisterChildResponseMessage =
                                RegisterChildResponseMessage::from_bytes(message.payload);

                            match message.status {
                                0 => Ok(()),
                                _ => Err(Error::new(
                                    ErrorCode::try_from(message.status)?,
                                    "failed to register child with process manager daemon",
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
