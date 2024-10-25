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
use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
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
    /// Part number.
    pub part_number: u32,
    /// Payload size.
    pub payload_size: u8,
    /// Payload.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}
::nvx::sys::static_assert_size!(LinuxDaemonMessagePart, LinuxDaemonMessage::PAYLOAD_SIZE);

impl LinuxDaemonMessagePart {
    /// Maximum size of the payload.
    pub const PAYLOAD_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<u8>() - mem::size_of::<u32>();

    ///
    /// # Description
    ///
    /// Builds a request message that encodes a message part.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `header`: Message header.
    /// - `part_number`: Part number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the request message is returned. Upon failure, an error is returned instead.
    ///
    pub fn build_request(
        pid: ProcessIdentifier,
        header: LinuxDaemonMessageHeader,
        part_number: u32,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        Self::build(pid, header, part_number, payload_size, payload, false)
    }

    ///
    /// # Description
    ///
    /// Builds a response message that encodes a message part.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `header`: Message header.
    /// - `part_number`: Part number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the response message is returned. Upon failure, an error is returned instead.
    ///
    pub fn build_response(
        pid: ProcessIdentifier,
        header: LinuxDaemonMessageHeader,
        part_number: u32,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        Self::build(pid, header, part_number, payload_size, payload, true)
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
    /// - `pid`: Process identifier.
    /// - `header`: Message header.
    /// - `part_number`: Part number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the message is returned. Upon failure, an error is returned instead.
    fn build(
        pid: ProcessIdentifier,
        header: LinuxDaemonMessageHeader,
        part_number: u32,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
        is_response: bool,
    ) -> Result<Message, Error> {
        let message: LinuxDaemonMessagePart = Self::new(part_number, payload_size, payload)?;
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(header, message.into_bytes());
        if is_response {
            Ok(Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes()))
        } else {
            Ok(Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes()))
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new part of a Linux Daemon Message.
    ///
    /// # Parameters
    ///
    /// - `part_number`: Part number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    fn new(
        part_number: u32,
        payload_size: u8,
        payload: [u8; Self::PAYLOAD_SIZE],
    ) -> Result<Self, Error> {
        // Check if payload size is invalid.
        if payload_size as usize > payload.len() {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid payload size"));
        }

        Ok(Self {
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
            "LinuxDaemonMessagePart {{ part_number: {}, payload_size: {} }}",
            { self.part_number },
            { self.payload_size }
        )
    }
}
