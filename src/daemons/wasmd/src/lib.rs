// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![forbid(clippy::all)]
#![no_std]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem;
use ::nvx::{
    ipc::Message,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

#[repr(u8)]
pub enum WasmdMessageHeader {
    /// A message that contains a Wasm module.
    Wasm = 1,
}
nvx::sys::static_assert_size!(WasmdMessageHeader, 1);

impl TryFrom<u8> for WasmdMessageHeader {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(WasmdMessageHeader::Wasm),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid wasmd message")),
        }
    }
}

impl From<&WasmdMessageHeader> for u8 {
    fn from(value: &WasmdMessageHeader) -> Self {
        match value {
            WasmdMessageHeader::Wasm => 1,
        }
    }
}

pub struct WasmdMessage {
    pub header: WasmdMessageHeader,
    pub payload: [u8; Self::PAYLOAD_SIZE],
}

impl WasmdMessage {
    /// Total size of a WASMD message.
    pub const SIZE: usize = Message::PAYLOAD_SIZE;
    /// The size of the message's payload.
    pub const PAYLOAD_SIZE: usize = Self::SIZE - mem::size_of::<WasmdMessageHeader>();

    ///
    /// # Description
    ///
    /// Instantiates a new WASMD message.
    ///
    /// # Parameters
    ///
    /// - `header`: Message header.
    /// - `payload`: Message payload.
    ///
    /// # Returns
    ///
    /// A WASMD message.
    ///
    pub fn new(header: WasmdMessageHeader, payload: [u8; Self::PAYLOAD_SIZE]) -> Self {
        Self { header, payload }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A WASMD message.
    ///
    pub fn from_bytes(bytes: [u8; Self::SIZE]) -> Result<Self, Error> {
        // Check if message is valid.
        let _header: WasmdMessageHeader = WasmdMessageHeader::try_from(bytes[0])?;
        // Re-interpret bytes as a message.
        let message: WasmdMessage = unsafe { core::mem::transmute(bytes) };
        Ok(message)
    }

    ///
    /// # Description
    ///
    /// Converts a message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        unsafe { core::mem::transmute(self) }
    }
}

#[repr(C, packed)]
pub struct LoadMessage {
    pub len: u32,
    pub payload: [u8; Self::PAYLOAD_SIZE],
}

::nvx::sys::static_assert_size!(LoadMessage, LoadMessage::SIZE);

impl LoadMessage {
    /// Total size of a LOAD message.
    pub const SIZE: usize = WasmdMessage::PAYLOAD_SIZE;
    /// The size of the message's payload.
    pub const PAYLOAD_SIZE: usize = Self::SIZE - mem::size_of::<u32>();

    ///
    /// # Description
    ///
    /// Instantiates a new LOAD message.
    ///
    /// # Parameters
    ///
    /// - `len`: Length of the segment.
    /// - `payload`: Message payload.
    ///
    /// # Returns
    ///
    /// A LOAD message.
    ///
    pub fn new(len: u32, payload: [u8; Self::PAYLOAD_SIZE]) -> Self {
        Self { len, payload }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A LOAD message.
    ///
    pub fn from_bytes(bytes: [u8; Self::SIZE]) -> Self {
        // Re-interpret bytes as a message.
        unsafe { core::mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        unsafe { core::mem::transmute(self) }
    }
}
