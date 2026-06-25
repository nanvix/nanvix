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
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A message sent by the process manager daemon to the filesystem daemon notifying it that a
/// process has terminated, so that the filesystem daemon can reclaim the process's per-process
/// state (open file descriptors and current working directory).
///
#[repr(C, packed)]
pub struct ProcessExitMessage {
    /// Process identifier of the terminated process.
    pub pid: ProcessIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a process-exit message must match the size of a process management message
// payload.
::static_assert::assert_eq_size!(ProcessExitMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl ProcessExitMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize =
        ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<ProcessIdentifier>();

    ///
    /// # Description
    ///
    /// Instantiates a new process-exit message.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier of the terminated process.
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
    /// Converts a byte array into a process-exit message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A process-exit message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a process-exit message into a byte array.
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
/// Builds a process-exit notification addressed to the filesystem daemon.
///
/// # Parameters
///
/// - `pid`: Process identifier of the terminated process.
///
/// # Returns
///
/// Upon successful completion, a process-exit notification message is returned. Otherwise, an error
/// is returned instead.
///
pub fn process_exit_request(pid: ProcessIdentifier) -> Result<Message, Error> {
    let process_exit_message: ProcessExitMessage = ProcessExitMessage::new(pid);
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::ProcessExit,
        process_exit_message.into_bytes(),
    );

    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    let ipc_message: Message = Message::new(
        MessageSender::new(ProcessIdentifier::PROCD, ThreadIdentifier::NONE),
        MessageReceiver::new(ProcessIdentifier::VFSD, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    );

    Ok(ipc_message)
}
