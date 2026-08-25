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
    /// The message encodes information about an interrupt that occurred.
    Interrupt = 1,
    /// The message encodes information about an exception that occurred.
    Exception = 2,
    /// The message carries information sent by a process to another.
    Ipc = 3,
    /// The message encodes information about a process termination event.
    ProcessTerminationEvent = 4,
    /// The message carries information sent from one kernel to another.
    Ikc = 5,
    /// The message signals completion of a bulk pull transfer.
    PullResponse = 6,
    /// The message encodes information about a process creation event.
    ProcessCreationEvent = 7,
    /// The message encodes information about a thread termination event.
    ThreadTerminationEvent = 8,
}
::static_assert::assert_eq_size!(MessageType, 1);

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
            MessageType::Interrupt => [1],
            MessageType::Exception => [2],
            MessageType::Ipc => [3],
            MessageType::ProcessTerminationEvent => [4],
            MessageType::Ikc => [5],
            MessageType::PullResponse => [6],
            MessageType::ProcessCreationEvent => [7],
            MessageType::ThreadTerminationEvent => [8],
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
            [1] => Ok(MessageType::Interrupt),
            [2] => Ok(MessageType::Exception),
            [3] => Ok(MessageType::Ipc),
            [4] => Ok(MessageType::ProcessTerminationEvent),
            [5] => Ok(MessageType::Ikc),
            [6] => Ok(MessageType::PullResponse),
            [7] => Ok(MessageType::ProcessCreationEvent),
            [8] => Ok(MessageType::ThreadTerminationEvent),
            _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid message type")),
        }
    }
}

impl fmt::Debug for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageType::Interrupt => write!(f, "interrupt"),
            MessageType::Exception => write!(f, "exception"),
            MessageType::Ipc => write!(f, "inter-process communication"),
            MessageType::ProcessTerminationEvent => write!(f, "process termination event"),
            MessageType::Ikc => write!(f, "inter-kernel communication"),
            MessageType::PullResponse => write!(f, "pull response"),
            MessageType::ProcessCreationEvent => write!(f, "process creation event"),
            MessageType::ThreadTerminationEvent => write!(f, "thread termination event"),
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::alloc::format;

    #[test]
    fn message_type_discriminants_round_trip() {
        let cases: [(MessageType, u8); 8] = [
            (MessageType::Interrupt, 1),
            (MessageType::Exception, 2),
            (MessageType::Ipc, 3),
            (MessageType::ProcessTerminationEvent, 4),
            (MessageType::Ikc, 5),
            (MessageType::PullResponse, 6),
            (MessageType::ProcessCreationEvent, 7),
            (MessageType::ThreadTerminationEvent, 8),
        ];

        for (message_type, raw) in cases {
            assert_eq!(message_type.to_bytes(), [raw]);
            assert_eq!(
                MessageType::try_from_bytes([raw]).expect("valid message type should parse"),
                message_type
            );
        }
    }

    #[test]
    fn invalid_message_type_discriminants_are_rejected() {
        assert!(MessageType::try_from_bytes([0]).is_err());
        assert!(MessageType::try_from_bytes([9]).is_err());
        assert!(MessageType::try_from_bytes([u8::MAX]).is_err());
    }

    #[test]
    fn thread_termination_message_type_is_formatted() {
        assert_eq!(
            format!("{:?}", MessageType::ThreadTerminationEvent),
            "thread termination event"
        );
    }
}
