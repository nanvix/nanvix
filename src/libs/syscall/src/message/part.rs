// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::core::{
    fmt::Debug,
    mem,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
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
/// This structure represents a part of a System Call Message.
///
#[repr(C, packed)]
pub struct SystemCallMessagePart {
    /// Total parts.
    pub total_parts: u16,
    /// Part number.
    pub part_number: u16,
    /// Payload size.
    pub payload_size: u8,
    /// Payload.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}
::static_assert::assert_eq_size!(SystemCallMessagePart, SystemCallMessage::PAYLOAD_SIZE);

impl SystemCallMessagePart {
    /// Maximum size of the payload.
    pub const PAYLOAD_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<u8>()
        - mem::size_of::<u16>()
        - mem::size_of::<u16>();

    ///
    /// # Description
    ///
    /// Builds a request message that encodes a message part.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    /// - `header`: Message header.
    /// - `total_parts`: Total number of parts.
    /// - `part_number`: Part number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    /// - `destination`: Process identifier of the destination daemon.
    /// - `message_type`: Message type to use.
    ///
    /// # Returns
    ///
    /// Upon success, the request message is returned. Upon failure, an error is returned instead.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn build_request(
        tid: ThreadIdentifier,
        header: SystemCallMessageHeader,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Result<Message, Error> {
        Self::build(
            tid,
            header,
            total_parts,
            part_number,
            payload_size,
            payload,
            false,
            destination,
            message_type,
        )
    }

    ///
    /// # Description
    ///
    /// Builds a response message that encodes a message part.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    /// - `header`: Message header.
    /// - `total_parts`: Total number of parts.
    /// - `part_number`: Part number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the response message is returned. Upon failure, an error is returned instead.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn build_response(
        tid: ThreadIdentifier,
        header: SystemCallMessageHeader,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Result<Message, Error> {
        Self::build(
            tid,
            header,
            total_parts,
            part_number,
            payload_size,
            payload,
            true,
            source,
            message_type,
        )
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a System Call Message Part.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A System Call Message Part.
    ///
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a System Call Message Part into a byte array.
    ///
    /// # Returns
    ///
    /// A byte array.
    ///
    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    ///
    /// # Description
    ///
    /// Builds a message that encodes a message part.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    /// - `header`: Message header.
    /// - `total_parts`: Total number of parts.
    /// - `part_number`: Part number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the message is returned. Upon failure, an error is returned instead.
    #[allow(clippy::too_many_arguments)]
    fn build(
        tid: ThreadIdentifier,
        header: SystemCallMessageHeader,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
        is_response: bool,
        daemon: ProcessIdentifier,
        message_type: MessageType,
    ) -> Result<Message, Error> {
        // Check if part number is valid.
        if part_number >= total_parts {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "part number is greater than or equal to total parts",
            ));
        }

        let message: SystemCallMessagePart =
            Self::new(total_parts, part_number, payload_size, payload)?;
        let message: SystemCallMessage = SystemCallMessage::new(header, message.into_bytes());
        if is_response {
            Ok(Message::new(
                MessageSender::new(daemon, ThreadIdentifier::NONE),
                MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
                message_type,
                None,
                message.into_bytes(),
            ))
        } else {
            Ok(Message::new(
                MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
                MessageReceiver::new(daemon, ThreadIdentifier::NONE),
                message_type,
                None,
                message.into_bytes(),
            ))
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new part of a System Call Message.
    ///
    /// # Parameters
    ///
    /// - `total_parts`: Total number of parts.
    /// - `part_number`: Part number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    fn new(
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
    ) -> Result<Self, Error> {
        // Check if payload size is invalid.
        if payload_size as usize > payload.len() {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid payload size"));
        }

        Ok(Self {
            total_parts,
            part_number,
            payload_size,
            payload,
        })
    }
}

impl Debug for SystemCallMessagePart {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "SystemCallMessagePart {{ part_number: {}, total_parts={},  payload_size: {} }}",
            { self.part_number },
            { self.total_parts },
            { self.payload_size }
        )
    }
}
