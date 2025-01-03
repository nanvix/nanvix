// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//  Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::typ::MessageType,
    pm::ProcessIdentifier,
    sys::config,
};
use ::core::mem;

//==================================================================================================
//  Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents a message that can be sent between processes.
///
/// # Notes
///
/// - All fields in this structure are intentionally public to enable zero-copy message parsing.
///
#[derive(Debug)]
#[repr(C, packed)]
pub struct Message {
    /// Type of the message.
    pub message_type: MessageType,
    /// Process that sent the message.
    pub source: ProcessIdentifier,
    /// Process that should receive the message.
    pub destination: ProcessIdentifier,
    /// Message status.
    pub status: i32,
    /// Payload of the message.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}
crate::static_assert_size!(Message, config::kernel::IPC_MESSAGE_SIZE);

//==================================================================================================
//  Implementations
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
    /// - `source`: The source process.
    /// - `destination`: The destination process.
    /// - `message_type`: The type of the message.
    /// - `status`: Error status of the message (`None` for success).
    /// - `payload`: The message payload.
    ///
    /// # Returns
    ///
    /// The new message.
    ///
    pub fn new(
        source: ProcessIdentifier,
        destination: ProcessIdentifier,
        message_type: MessageType,
        status: Option<ErrorCode>,
        payload: [u8; Self::PAYLOAD_SIZE],
    ) -> Self {
        Self {
            message_type,
            source,
            destination,
            status: if let Some(status) = status {
                status.into_errno()
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
            source: ProcessIdentifier::KERNEL,
            destination: ProcessIdentifier::KERNEL,
            status: 0,
            payload: [0; Self::PAYLOAD_SIZE],
        }
    }
}
