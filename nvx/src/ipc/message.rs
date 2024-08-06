// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem;
use ::error::{
    Error,
    ErrorCode,
};
use ::kcall::ipc::Message;

//==================================================================================================
// System Message Header
//==================================================================================================

///
/// # Description
///
/// A type that encodes the header of a system message.
///
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum SystemMessageHeader {
    /// Process management message.
    ProcessManagement = 1,
    /// Memory management message.
    MemoryManagement = 2,
    /// Filesystem management message.
    FilesystemManagement = 3,
}

impl TryFrom<u8> for SystemMessageHeader {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(SystemMessageHeader::ProcessManagement),
            2 => Ok(SystemMessageHeader::MemoryManagement),
            3 => Ok(SystemMessageHeader::FilesystemManagement),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid system message")),
        }
    }
}

impl From<&SystemMessageHeader> for u8 {
    fn from(value: &SystemMessageHeader) -> Self {
        match value {
            SystemMessageHeader::ProcessManagement => 1,
            SystemMessageHeader::MemoryManagement => 2,
            SystemMessageHeader::FilesystemManagement => 3,
        }
    }
}

//==================================================================================================
// System Message
//==================================================================================================

///
/// # Description
///
/// A type that encodes a system message.
///
#[repr(C, packed)]
pub struct SystemMessage {
    /// Message header.
    pub header: SystemMessageHeader,
    /// Message payload.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}

static_assert_size!(SystemMessage, Message::SIZE);

impl SystemMessage {
    /// Size of payload.
    pub const PAYLOAD_SIZE: usize = Message::SIZE - mem::size_of::<SystemMessageHeader>();

    ///
    /// # Description
    ///
    /// Instantiates a new system message.
    ///
    /// # Parameters
    ///
    /// - `header`: Message header.
    /// - `payload`: Message payload.
    ///
    /// # Returns
    ///
    /// A system message.
    ///
    pub fn new(header: SystemMessageHeader, payload: [u8; Self::PAYLOAD_SIZE]) -> Self {
        Self { header, payload }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a system message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A system message.
    ///
    pub fn from_bytes(bytes: [u8; Message::SIZE]) -> Result<Self, Error> {
        // Check if message header is valid.
        let _header: SystemMessageHeader = SystemMessageHeader::try_from(bytes[0])?;

        let message: SystemMessage = unsafe { mem::transmute(bytes) };

        Ok(message)
    }

    ///
    /// # Description
    ///
    /// Converts a system message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; Message::SIZE] {
        unsafe { mem::transmute(self) }
    }
}
