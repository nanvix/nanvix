// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ipc::{
        SystemMessage,
        SystemMessageHeader,
    },
    pm::message::{
        LookupResponseMessage,
        ProcessManagementMessage,
        ProcessManagementMessageHeader,
    },
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
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Looks up a process by name.
///
/// # Parameters
///
/// - `name`: Name of the process.
///
/// # Returns
///
/// Upon successful completion, the process identifier of the process is returned. Upon failure, an
/// error is returned instead.
///
pub fn lookup(name: &str) -> Result<ProcessIdentifier, Error> {
    // Send lookup message.
    crate::pm::message::lookup(name)?;

    // Wait response from the process manager daemon.
    let message: Message = ::sys::kcall::ipc::recv()?;

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
                        ProcessManagementMessageHeader::LookupResponse => {
                            let message: LookupResponseMessage =
                                LookupResponseMessage::from_bytes(message.payload);

                            match message.status {
                                0 => Ok(message.pid),
                                _ => Err(Error::new(
                                    ErrorCode::try_from(message.status)?,
                                    "failed to lookup process",
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
