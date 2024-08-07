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
        ProcessManagementMessage,
        ProcessManagementMessageHeader,
        ShutdownMessage,
    },
};
use error::Error;
use kcall::{
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
/// Sends a shutdown message to a process.
///
/// # Parameters
///
/// - `destination`: Destination process.
/// - `code`: Shutdown code.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn shutdown(destination: ProcessIdentifier, code: u8) -> Result<(), Error> {
    // Construct a shutdown message.
    let shutdown_message: ShutdownMessage = ShutdownMessage::new(code);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::Shutdown,
        shutdown_message.into_bytes(),
    );

    // Construct a system message.
    let sys_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC message.
    let ipc_message: Message = Message::new(
        ProcessIdentifier::PROCD,
        destination,
        sys_message.into_bytes(),
        MessageType::Ipc,
    );

    // Send IPC message.
    ::kcall::ipc::send(&ipc_message)
}
