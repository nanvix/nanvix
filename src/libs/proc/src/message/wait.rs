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
/// A message that encodes a wait operation. It is sent by a process to the process manager daemon
/// to wait for the termination of a child process (`waitpid()`).
///
#[repr(C, packed)]
pub struct WaitMessage {
    /// Process identifier of the child to wait for (`-1` waits for any child).
    pub pid: ProcessIdentifier,
    /// Wait options.
    pub options: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a wait message must match the size of a process management message payload.
::static_assert::assert_eq_size!(WaitMessage, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A message that encodes the response of a wait operation.
///
#[repr(C, packed)]
pub struct WaitResponseMessage {
    /// Process identifier of the reaped child (`0` when no child was ready and `WNOHANG` was set).
    pub child: ProcessIdentifier,
    /// Termination status of the reaped child.
    pub status: i32,
    /// Error code of the wait operation (`0` on success).
    pub error: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a wait response message must match the size of a process management message payload.
::static_assert::assert_eq_size!(WaitResponseMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl WaitMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new wait message.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier of the child to wait for.
    /// - `options`: Wait options.
    ///
    pub fn new(pid: ProcessIdentifier, options: i32) -> Self {
        Self {
            pid,
            options,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a wait message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A wait message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a wait message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl WaitResponseMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<i32>()
        - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new wait response message.
    ///
    /// # Parameters
    ///
    /// - `child`: Process identifier of the reaped child.
    /// - `status`: Termination status of the reaped child.
    /// - `error`: Error code of the wait operation.
    ///
    pub fn new(child: ProcessIdentifier, status: i32, error: i32) -> Self {
        Self {
            child,
            status,
            error,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a wait response message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A wait response message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a wait response message into a byte array.
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
/// Builds a wait request message.
///
/// # Parameters
///
/// - `caller`: Process identifier of the calling process (and message sender).
/// - `pid`: Process identifier of the child to wait for.
/// - `options`: Wait options.
///
/// # Returns
///
/// Upon successful completion, a wait request message is returned. Otherwise, an error is returned
/// instead.
///
pub fn wait_request(
    caller: ProcessIdentifier,
    pid: ProcessIdentifier,
    options: i32,
) -> Result<Message, Error> {
    // Construct a wait message.
    let wait_message: WaitMessage = WaitMessage::new(pid, options);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::Wait,
        wait_message.into_bytes(),
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
/// Builds a wait response message.
///
/// # Parameters
///
/// - `destination`: Destination process.
/// - `child`: Process identifier of the reaped child.
/// - `status`: Termination status of the reaped child.
/// - `error`: Error code of the wait operation.
///
/// # Returns
///
/// Upon successful completion, a wait response message is returned. Otherwise, an error is returned
/// instead.
///
pub fn wait_response(
    destination: ProcessIdentifier,
    child: ProcessIdentifier,
    status: i32,
    error: i32,
) -> Result<Message, Error> {
    // Construct a wait response message.
    let wait_response_message: WaitResponseMessage = WaitResponseMessage::new(child, status, error);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::WaitResponse,
        wait_response_message.into_bytes(),
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
