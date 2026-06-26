// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    self,
    KillResponseMessage,
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
/// Posts a signal to a target process by routing the request through the process manager daemon,
/// which enforces the cross-process permission policy (`kill()`).
///
/// # Parameters
///
/// - `target`: Process identifier of the target process.
/// - `signum`: Signal number to post, or zero for the null-signal probe.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn kill(target: ProcessIdentifier, signum: i32) -> Result<(), Error> {
    // Retrieve process identifier of the calling process.
    let caller: ProcessIdentifier = ::sys::kcall::pm::getpid()?;

    // Build kill message and send it.
    let message: Message = message::kill_request(caller, target, signum)?;
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
                        ProcessManagementMessageHeader::KillResponse => {
                            let message: KillResponseMessage =
                                KillResponseMessage::from_bytes(message.payload);

                            // Check for errors.
                            if message.error != 0 {
                                return Err(Error::new(
                                    ErrorCode::try_from(message.error)?,
                                    "failed to post signal to target process",
                                ));
                            }

                            Ok(())
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
