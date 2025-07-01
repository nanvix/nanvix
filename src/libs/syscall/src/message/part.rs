// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
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
    pm::ThreadIdentifier,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure represents a part of a Linux Daemon Message.
///
#[repr(C, packed)]
pub struct LinuxDaemonMessagePart {
    /// Total parts.
    pub total_parts: u16,
    /// Part number.
    pub part_number: u16,
    /// Payload size.
    pub payload_size: u8,
    /// Payload.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}
::static_assert::assert_eq_size!(LinuxDaemonMessagePart, LinuxDaemonMessage::PAYLOAD_SIZE);

impl LinuxDaemonMessagePart {
    /// Maximum size of the payload.
    pub const PAYLOAD_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
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
    ///
    /// # Returns
    ///
    /// Upon success, the request message is returned. Upon failure, an error is returned instead.
    ///
    pub fn build_request(
        tid: ThreadIdentifier,
        header: LinuxDaemonMessageHeader,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        Self::build(tid, header, total_parts, part_number, payload_size, payload, false)
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
    pub fn build_response(
        tid: ThreadIdentifier,
        header: LinuxDaemonMessageHeader,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        Self::build(tid, header, total_parts, part_number, payload_size, payload, true)
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a Linux Daemon Message Part.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A Linux Daemon Message Part.
    ///
    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a Linux Daemon Message Part into a byte array.
    ///
    /// # Returns
    ///
    /// A byte array.
    ///
    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
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
    fn build(
        tid: ThreadIdentifier,
        header: LinuxDaemonMessageHeader,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
        is_response: bool,
    ) -> Result<Message, Error> {
        // Check if part number is valid.
        if part_number >= total_parts {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "part number is greater than or equal to total parts",
            ));
        }

        let message: LinuxDaemonMessagePart =
            Self::new(total_parts, part_number, payload_size, payload)?;
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(header, message.into_bytes());
        if is_response {
            Ok(Message::new(
                MessageSender::from(crate::LINUXD),
                MessageReceiver::from(tid),
                MessageType::Ikc,
                None,
                message.into_bytes(),
            ))
        } else {
            Ok(Message::new(
                MessageSender::from(tid),
                MessageReceiver::from(crate::LINUXD),
                MessageType::Ikc,
                None,
                message.into_bytes(),
            ))
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new part of a Linux Daemon Message.
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

impl Debug for LinuxDaemonMessagePart {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "LinuxDaemonMessagePart {{ part_number: {}, total_parts={},  payload_size: {} }}",
            { self.part_number },
            { self.total_parts },
            { self.payload_size }
        )
    }
}
