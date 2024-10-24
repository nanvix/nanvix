// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    self,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    SignupResponseMessage,
};
use ::nvx::{
    ipc::{
        SystemMessage,
        SystemMessageHeader,
    },
    sys::{
        error::{
            Error,
            ErrorCode,
        },
        ipc::{
            Message,
            MessageType,
        },
        pm::ProcessIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Registers a process with the process manager daemon.
///
/// # Parameters
///
/// - `pid`: Process identifier.
/// - `name`: Process name.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn signup(pid: &ProcessIdentifier, name: &str) -> Result<(), Error> {
    // Build signup message and send it.
    let message: Message = message::signup_request(*pid, name)?;
    ::nvx::ipc::send(&message)?;

    // Wait unblock message from the process manager daemon.
    let message: Message = ::nvx::ipc::recv()?;

    // Parse message.
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
                        ProcessManagementMessageHeader::SignupResponse => {
                            let message: SignupResponseMessage =
                                SignupResponseMessage::from_bytes(message.payload);

                            match message.status {
                                0 => Ok(()),
                                _ => Err(Error::new(
                                    ErrorCode::try_from(message.status)?,
                                    "failed to signup to process manager daemon",
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
