// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
};
use ::core::mem;
use ::nvx::{
    ipc::{
        Message,
        MessageType,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::ProcessIdentifier,
    sys::error::Error,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A message that encodes a shutdown operation.
///
pub struct ShutdownMessage {
    /// Shutdown code.
    pub code: u8,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: the size of a shutdown message must match the size of a process management message payload.
::nvx::sys::static_assert_size!(ShutdownMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl ShutdownMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<u8>();

    ///
    /// # Description
    ///
    /// Instantiates a new shutdown message.
    ///
    /// # Parameters
    ///
    /// - `code`: Shutdown code.
    ///
    /// # Returns
    ///
    /// A shutdown message.
    ///
    pub fn new(code: u8) -> Self {
        Self {
            code,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a shutdown message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// The corresponding shutdown message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a shutdown message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds a shutdown message.
///
/// # Parameters
///
/// - `destination`: Destination process.
/// - `code`: Shutdown code.
///
/// # Returns
///
/// Upon successful completion, the IPC message is returned. Upon failure, an error is returned
/// instead.
///
pub fn shutdown_request(destination: ProcessIdentifier, code: u8) -> Result<Message, Error> {
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
    let ipc_message: Message =
        Message::new(crate::PROCD, destination, MessageType::Ipc, None, sys_message.into_bytes());

    Ok(ipc_message)
}
