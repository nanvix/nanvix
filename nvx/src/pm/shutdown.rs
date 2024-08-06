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
    },
};
use ::core::mem;
use ::error::Error;
use ::kcall::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// Shutdown Message
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
static_assert_size!(ShutdownMessage, ProcessManagementMessage::PAYLOAD_SIZE);

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
