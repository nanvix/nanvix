// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//  Imports
//==================================================================================================

use crate::error::{
    Error,
    ErrorCode,
};
use ::core::{
    fmt,
    mem,
};

//==================================================================================================
//  Structures
//==================================================================================================

///
/// # Description
///
/// Type that describes what the message is about.
///
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    /// The message is empty.
    Empty,
    /// The message encodes information about an interrupt that occurred.
    Interrupt,
    /// The message encodes information about an exception that occurred.
    Exception,
    /// The message carries information sent by a process to another.
    Ipc,
    /// The message encodes information about a scheduling event.
    SchedulingEvent,
    /// The message carries information sent from one kernel to another.
    Ikc,
}
crate::static_assert_size!(MessageType, 1);

//==================================================================================================
//  Structures
//==================================================================================================

impl MessageType {
    /// The size of a message type.
    pub const SIZE: usize = mem::size_of::<u8>();

    ///
    /// # Description
    ///
    /// Converts the targets message type to a byte array.
    ///
    /// # Returns
    ///
    /// A byte array representing the target message type.
    ///
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        match self {
            MessageType::Empty => [0],
            MessageType::Interrupt => [1],
            MessageType::Exception => [2],
            MessageType::Ipc => [3],
            MessageType::SchedulingEvent => [4],
            MessageType::Ikc => [5],
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to convert a byte array to a message type.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array to convert.
    ///
    /// # Returns
    ///
    /// On success, the message type encoded in the byte array is returned. On error, an error is
    /// returned instead.
    ///
    pub fn try_from_bytes(bytes: [u8; Self::SIZE]) -> Result<Self, Error> {
        match bytes {
            [0] => Ok(MessageType::Empty),
            [1] => Ok(MessageType::Interrupt),
            [2] => Ok(MessageType::Exception),
            [3] => Ok(MessageType::Ipc),
            [4] => Ok(MessageType::SchedulingEvent),
            [5] => Ok(MessageType::Ikc),
            _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid message type")),
        }
    }
}

impl fmt::Debug for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageType::Empty => write!(f, "empty"),
            MessageType::Interrupt => write!(f, "interrupt"),
            MessageType::Exception => write!(f, "exception"),
            MessageType::Ipc => write!(f, "inter-process communication"),
            MessageType::SchedulingEvent => write!(f, "scheduling event"),
            MessageType::Ikc => write!(f, "inter-kernel communication"),
        }
    }
}
