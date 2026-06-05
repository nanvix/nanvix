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
use ::sys::{
    error::Error,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A message that encodes a register-child operation. It is sent by a parent process to the
/// process manager daemon after a successful `fork()` to record the parent/child relationship.
///
#[repr(C, packed)]
pub struct RegisterChildMessage {
    /// Process identifier of the child.
    pub child: ProcessIdentifier,
    /// Process identifier of the parent.
    pub parent: ProcessIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a register-child message must match the size of a process management message payload.
::static_assert::assert_eq_size!(RegisterChildMessage, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A message that encodes the response of a register-child operation.
///
#[repr(C, packed)]
pub struct RegisterChildResponseMessage {
    /// Status of the register-child operation.
    pub status: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a register-child response message must match the size of a process management message payload.
::static_assert::assert_eq_size!(
    RegisterChildResponseMessage,
    ProcessManagementMessage::PAYLOAD_SIZE
);

//==================================================================================================
// Implementations
//==================================================================================================

impl RegisterChildMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<ProcessIdentifier>();

    ///
    /// # Description
    ///
    /// Instantiates a new register-child message.
    ///
    /// # Parameters
    ///
    /// - `child`: Process identifier of the child.
    /// - `parent`: Process identifier of the parent.
    ///
    pub fn new(child: ProcessIdentifier, parent: ProcessIdentifier) -> Self {
        Self {
            child,
            parent,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a register-child message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A register-child message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a register-child message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl RegisterChildResponseMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new register-child response message.
    ///
    /// # Parameters
    ///
    /// - `status`: Status of the register-child operation.
    ///
    pub fn new(status: i32) -> Self {
        Self {
            status,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a register-child response message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A register-child response message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a register-child response message into a byte array.
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
/// Builds a register-child request message.
///
/// # Parameters
///
/// - `child`: Process identifier of the child.
/// - `parent`: Process identifier of the parent (and message sender).
///
/// # Returns
///
/// Upon successful completion, a register-child request message is returned. Otherwise, an error is
/// returned instead.
///
pub fn register_child_request(
    child: ProcessIdentifier,
    parent: ProcessIdentifier,
) -> Result<Message, Error> {
    // Construct a register-child message.
    let register_child_message: RegisterChildMessage = RegisterChildMessage::new(child, parent);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::RegisterChild,
        register_child_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC message.
    let ipc_message: Message = Message::new(
        MessageSender::from(parent),
        MessageReceiver::from(crate::PROCD),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    );

    Ok(ipc_message)
}

///
/// # Description
///
/// Builds a register-child response message.
///
/// # Parameters
///
/// - `destination`: Destination process.
/// - `status`: Status of the register-child operation.
///
/// # Returns
///
/// Upon successful completion, a register-child response message is returned. Otherwise, an error
/// is returned instead.
///
pub fn register_child_response(
    destination: ProcessIdentifier,
    status: i32,
) -> Result<Message, Error> {
    // Construct a register-child response message.
    let register_child_response_message: RegisterChildResponseMessage =
        RegisterChildResponseMessage::new(status);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::RegisterChildResponse,
        register_child_response_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC message.
    let ipc_message: Message = Message::new(
        MessageSender::from(crate::PROCD),
        MessageReceiver::from(destination),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    );

    Ok(ipc_message)
}
