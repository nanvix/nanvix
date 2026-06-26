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
/// A message that encodes a kill operation. It is sent by a process to the process manager daemon
/// to post a signal to a target process (`kill()`).
///
#[repr(C, packed)]
pub struct KillMessage {
    /// Process identifier of the target process.
    pub target: ProcessIdentifier,
    /// Signal number to post, or zero for the null-signal probe.
    pub signum: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a kill message must match the size of a process management message payload.
::static_assert::assert_eq_size!(KillMessage, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A message that encodes the response of a kill operation.
///
#[repr(C, packed)]
pub struct KillResponseMessage {
    /// Error code of the kill operation (`0` on success).
    pub error: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a kill response message must match the size of a process management message payload.
::static_assert::assert_eq_size!(KillResponseMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl KillMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new kill message.
    ///
    /// # Parameters
    ///
    /// - `target`: Process identifier of the target process.
    /// - `signum`: Signal number to post.
    ///
    pub fn new(target: ProcessIdentifier, signum: i32) -> Self {
        Self {
            target,
            signum,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a kill message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A kill message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a kill message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl KillResponseMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new kill response message.
    ///
    /// # Parameters
    ///
    /// - `error`: Error code of the kill operation.
    ///
    pub fn new(error: i32) -> Self {
        Self {
            error,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a kill response message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A kill response message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a kill response message into a byte array.
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
/// Builds a kill request message.
///
/// # Parameters
///
/// - `caller`: Process identifier of the calling process (and message sender).
/// - `target`: Process identifier of the target process.
/// - `signum`: Signal number to post.
///
/// # Returns
///
/// Upon successful completion, a kill request message is returned. Otherwise, an error is returned
/// instead.
///
pub fn kill_request(
    caller: ProcessIdentifier,
    target: ProcessIdentifier,
    signum: i32,
) -> Result<Message, Error> {
    // Construct a kill message.
    let kill_message: KillMessage = KillMessage::new(target, signum);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::Kill,
        kill_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC message.
    let ipc_message: Message = Message::new(
        MessageSender::from(caller),
        MessageReceiver::from(ProcessIdentifier::PROCD),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    );

    Ok(ipc_message)
}

///
/// # Description
///
/// Builds a kill response message.
///
/// # Parameters
///
/// - `destination`: Destination process.
/// - `error`: Error code of the kill operation.
///
/// # Returns
///
/// Upon successful completion, a kill response message is returned. Otherwise, an error is returned
/// instead.
///
pub fn kill_response(destination: ProcessIdentifier, error: i32) -> Result<Message, Error> {
    // Construct a kill response message.
    let kill_response_message: KillResponseMessage = KillResponseMessage::new(error);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::KillResponse,
        kill_response_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC message.
    let ipc_message: Message = Message::new(
        MessageSender::from(ProcessIdentifier::PROCD),
        MessageReceiver::from(destination),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    );

    Ok(ipc_message)
}
