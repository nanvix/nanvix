// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::typ::MessageType,
    pm::ProcessIdentifier,
};
use ::core::mem;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MessageSender(i32);

impl MessageSender {
    /// The kernel process is the sender of the message.
    pub const KERNEL: Self = MessageSender(ProcessIdentifier::KERNEL_RAW);
}

impl From<ProcessIdentifier> for MessageSender {
    fn from(pid: ProcessIdentifier) -> Self {
        Self(pid.into())
    }
}

impl From<MessageSender> for ProcessIdentifier {
    fn from(sender: MessageSender) -> ProcessIdentifier {
        ProcessIdentifier::from(sender.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MessageReceiver(i32);

impl MessageReceiver {
    /// The kernel process is the receiver of the message.
    pub const KERNEL: Self = MessageReceiver(ProcessIdentifier::KERNEL_RAW);
}

impl From<ProcessIdentifier> for MessageReceiver {
    fn from(pid: ProcessIdentifier) -> Self {
        Self(pid.into())
    }
}

impl From<MessageReceiver> for ProcessIdentifier {
    fn from(receiver: MessageReceiver) -> ProcessIdentifier {
        ProcessIdentifier::from(receiver.0)
    }
}

///
/// # Description
///
/// A structure that represents a message that can be sent between processes.
///
/// # Notes
///
/// - All fields in this structure are intentionally public to enable zero-copy message parsing.
///
#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct Message {
    /// Type of the message.
    pub message_type: MessageType,
    /// Process that sent the message.
    pub source: MessageSender,
    /// Process that should receive the message.
    pub destination: MessageReceiver,
    /// Message status.
    pub status: i32,
    /// Payload of the message.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}
::static_assert::assert_eq_size!(Message, config::kernel::IPC_MESSAGE_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl Message {
    /// The size of the message header fields (source, destination and type).
    pub const HEADER_SIZE: usize =
        2 * mem::size_of::<ProcessIdentifier>() + MessageType::SIZE + mem::size_of::<i32>();
    /// The size of the message's payload.
    pub const PAYLOAD_SIZE: usize = config::kernel::IPC_MESSAGE_SIZE - Self::HEADER_SIZE;

    ///
    /// # Description
    ///
    /// Creates a new message.
    ///
    /// # Parameters
    ///
    /// - `source`: The sender of the message.
    /// - `destination`: The recipient of the message.
    /// - `message_type`: The type of the message.
    /// - `status`: Error status of the message (`None` for success).
    /// - `payload`: The message payload.
    ///
    /// # Returns
    ///
    /// The new message.
    ///
    pub fn new(
        source: MessageSender,
        destination: MessageReceiver,
        message_type: MessageType,
        status: Option<ErrorCode>,
        payload: [u8; Self::PAYLOAD_SIZE],
    ) -> Self {
        Self {
            message_type,
            source,
            destination,
            status: if let Some(status) = status {
                status.get()
            } else {
                0
            },
            payload,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the target message to a byte array.
    ///
    /// # Returns
    ///
    /// A byte array that represents the target message.
    ///
    pub fn to_bytes(self) -> [u8; Self::HEADER_SIZE + Self::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    ///
    /// # Description
    ///
    /// Attempts to convert a byte array to a message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array to convert.
    ///
    /// # Returns
    ///
    /// Upon success, the message is returned. Upon failure, an error is returned instead.
    ///
    pub fn try_from_bytes(
        bytes: [u8; Self::HEADER_SIZE + Self::PAYLOAD_SIZE],
    ) -> Result<Self, Error> {
        Ok(unsafe { mem::transmute::<[u8; config::kernel::IPC_MESSAGE_SIZE], Message>(bytes) })
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            message_type: MessageType::Empty,
            source: MessageSender::KERNEL,
            destination: MessageReceiver::KERNEL,
            status: 0,
            payload: [0; Self::PAYLOAD_SIZE],
        }
    }
}
