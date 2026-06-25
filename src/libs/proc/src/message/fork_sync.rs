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
/// A message sent by a freshly forked parent to the process manager daemon asking it to confirm
/// that the child's filesystem state has been duplicated. The parent and child both block until the
/// daemon acknowledges, so that neither process races ahead of the fork-clone snapshot taken in the
/// filesystem daemon.
///
#[repr(C, packed)]
pub struct ForkSyncMessage {
    /// Process identifier of the child (the freshly forked process).
    pub child: ProcessIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a fork-sync message must match the size of a process management message payload.
::static_assert::assert_eq_size!(ForkSyncMessage, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A message sent by the process manager daemon to release a parent or child blocked on a fork-sync
/// request. The `status` field conveys the outcome of the fork synchronization: `0` when the
/// fork-clone was acknowledged by the filesystem daemon after the snapshot was taken, or a non-zero
/// error code when the snapshot failed. A non-zero status lets a blocked `fork()` abort instead of
/// hanging forever on a snapshot that will never be taken.
///
#[repr(C, packed)]
pub struct ForkSyncAckMessage {
    /// Outcome of the fork synchronization: `0` on success, or a non-zero error code on failure.
    pub status: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a fork-sync acknowledgement message must match the size of a process management
// message payload.
::static_assert::assert_eq_size!(ForkSyncAckMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl ForkSyncMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize =
        ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<ProcessIdentifier>();

    ///
    /// # Description
    ///
    /// Instantiates a new fork-sync message.
    ///
    /// # Parameters
    ///
    /// - `child`: Process identifier of the child.
    ///
    pub fn new(child: ProcessIdentifier) -> Self {
        Self {
            child,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a fork-sync message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A fork-sync message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a fork-sync message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl ForkSyncAckMessage {
    /// Status value that marks a successful fork synchronization.
    pub const STATUS_SUCCESS: i32 = 0;

    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new fork-sync acknowledgement message.
    ///
    /// # Parameters
    ///
    /// - `status`: Outcome of the fork synchronization (`0` on success, a non-zero error code on
    ///   failure).
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
    /// Converts a byte array into a fork-sync acknowledgement message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A fork-sync acknowledgement message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a fork-sync acknowledgement message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

/// Wraps a process management message destined to `destination` into an IPC message sent from
/// `source`.
fn wrap(
    source: ProcessIdentifier,
    destination: ProcessIdentifier,
    pm_message: ProcessManagementMessage,
) -> Message {
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    Message::new(
        MessageSender::new(source, ThreadIdentifier::NONE),
        MessageReceiver::new(destination, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    )
}

///
/// # Description
///
/// Builds a fork-sync request message addressed to the process manager daemon.
///
/// # Parameters
///
/// - `parent`: Process identifier of the requesting parent (message source).
/// - `child`: Process identifier of the freshly forked child.
///
/// # Returns
///
/// Upon successful completion, a fork-sync request message is returned. Otherwise, an error is
/// returned instead.
///
pub fn fork_sync_request(
    parent: ProcessIdentifier,
    child: ProcessIdentifier,
) -> Result<Message, Error> {
    let fork_sync_message: ForkSyncMessage = ForkSyncMessage::new(child);
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::ForkSync,
        fork_sync_message.into_bytes(),
    );

    Ok(wrap(parent, ProcessIdentifier::PROCD, pm_message))
}

///
/// # Description
///
/// Builds a fork-sync acknowledgement message sent by the process manager daemon to release a
/// blocked parent or child.
///
/// # Parameters
///
/// - `destination`: Process identifier of the process to release.
/// - `status`: Outcome of the fork synchronization (`0` on success, a non-zero error code on
///   failure).
///
/// # Returns
///
/// Upon successful completion, an IPC message carrying a fork-sync acknowledgement is returned.
/// Otherwise, an error is returned instead.
///
pub fn fork_sync_ack(destination: ProcessIdentifier, status: i32) -> Result<Message, Error> {
    let ack_message: ForkSyncAckMessage = ForkSyncAckMessage::new(status);
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::ForkSyncAck,
        ack_message.into_bytes(),
    );

    Ok(wrap(ProcessIdentifier::PROCD, destination, pm_message))
}
