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
/// A message that asks the filesystem daemon to clone the filesystem resources (open file
/// descriptors, current working directory, file-creation mask) of a parent process onto a freshly
/// forked child. It is sent by the process manager daemon while it records the parent/child
/// relationship, so that the child inherits a copy of the parent's filesystem state.
///
#[repr(C, packed)]
pub struct ForkCloneMessage {
    /// Process identifier of the parent (clone source).
    pub parent: ProcessIdentifier,
    /// Process identifier of the child (clone destination).
    pub child: ProcessIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a fork-clone message must match the size of a process management message payload.
::static_assert::assert_eq_size!(ForkCloneMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl ForkCloneMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<ProcessIdentifier>();

    ///
    /// # Description
    ///
    /// Instantiates a new fork-clone message.
    ///
    /// # Parameters
    ///
    /// - `parent`: Process identifier of the parent.
    /// - `child`: Process identifier of the child.
    ///
    pub fn new(parent: ProcessIdentifier, child: ProcessIdentifier) -> Self {
        Self {
            parent,
            child,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a fork-clone message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A fork-clone message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a fork-clone message into a byte array.
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
/// Builds a fork-clone request message addressed to the filesystem daemon.
///
/// # Parameters
///
/// - `parent`: Process identifier of the parent (clone source).
/// - `child`: Process identifier of the child (clone destination).
///
/// # Returns
///
/// Upon successful completion, a fork-clone request message is returned. Otherwise, an error is
/// returned instead.
///
pub fn fork_clone_request(
    parent: ProcessIdentifier,
    child: ProcessIdentifier,
) -> Result<Message, Error> {
    // Construct a fork-clone message.
    let fork_clone_message: ForkCloneMessage = ForkCloneMessage::new(parent, child);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::ForkClone,
        fork_clone_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC message. The notification is sent by the process manager daemon to the
    // filesystem daemon.
    let ipc_message: Message = Message::new(
        MessageSender::from(ProcessIdentifier::PROCD),
        MessageReceiver::from(ProcessIdentifier::VFSD),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    );

    Ok(ipc_message)
}
