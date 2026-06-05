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
/// A message that encodes a get-parent operation. It is sent by a process to the process manager
/// daemon to query the process identifier of its parent (`getppid()`).
///
#[repr(C, packed)]
pub struct GetParentMessage {
    /// Process identifier of the process whose parent is queried.
    pub pid: ProcessIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a get-parent message must match the size of a process management message payload.
::static_assert::assert_eq_size!(GetParentMessage, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A message that encodes the response of a get-parent operation.
///
#[repr(C, packed)]
pub struct GetParentResponseMessage {
    /// Process identifier of the parent.
    pub parent: ProcessIdentifier,
    /// Status of the get-parent operation.
    pub status: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a get-parent response message must match the size of a process management message payload.
::static_assert::assert_eq_size!(GetParentResponseMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl GetParentMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize =
        ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<ProcessIdentifier>();

    ///
    /// # Description
    ///
    /// Instantiates a new get-parent message.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier of the process whose parent is queried.
    ///
    pub fn new(pid: ProcessIdentifier) -> Self {
        Self {
            pid,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a get-parent message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A get-parent message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a get-parent message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl GetParentResponseMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new get-parent response message.
    ///
    /// # Parameters
    ///
    /// - `parent`: Process identifier of the parent.
    /// - `status`: Status of the get-parent operation.
    ///
    pub fn new(parent: ProcessIdentifier, status: i32) -> Self {
        Self {
            parent,
            status,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a get-parent response message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A get-parent response message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a get-parent response message into a byte array.
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
/// Builds a get-parent request message.
///
/// # Parameters
///
/// - `pid`: Process identifier of the process whose parent is queried (and message sender).
///
/// # Returns
///
/// Upon successful completion, a get-parent request message is returned. Otherwise, an error is
/// returned instead.
///
pub fn get_parent_request(pid: ProcessIdentifier) -> Result<Message, Error> {
    // Construct a get-parent message.
    let get_parent_message: GetParentMessage = GetParentMessage::new(pid);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::GetParent,
        get_parent_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC message.
    let ipc_message: Message = Message::new(
        MessageSender::from(pid),
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
/// Builds a get-parent response message.
///
/// # Parameters
///
/// - `destination`: Destination process.
/// - `parent`: Process identifier of the parent.
/// - `status`: Status of the get-parent operation.
///
/// # Returns
///
/// Upon successful completion, a get-parent response message is returned. Otherwise, an error is
/// returned instead.
///
pub fn get_parent_response(
    destination: ProcessIdentifier,
    parent: ProcessIdentifier,
    status: i32,
) -> Result<Message, Error> {
    // Construct a get-parent response message.
    let get_parent_response_message: GetParentResponseMessage =
        GetParentResponseMessage::new(parent, status);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::GetParentResponse,
        get_parent_response_message.into_bytes(),
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
