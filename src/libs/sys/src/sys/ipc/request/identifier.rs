// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::super::message::{
    Message,
    REQUEST_IDENTIFIER_OFFSET,
    REQUEST_IDENTIFIER_SIZE,
};

//==================================================================================================
// Structures
//==================================================================================================

/// Identifies one request within a thread's in-flight request window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct RequestIdentifier(u32);
::static_assert::assert_eq_size!(RequestIdentifier, REQUEST_IDENTIFIER_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl RequestIdentifier {
    /// Identifier used by messages that do not expect a response.
    pub const NONE: Self = Self(0);

    /// Byte offset of the identifier in a request/response message payload.
    const OFFSET: usize = REQUEST_IDENTIFIER_OFFSET;

    /// Size of a request identifier in bytes.
    pub const SIZE: usize = REQUEST_IDENTIFIER_SIZE;

    /// Creates an identifier from its wire representation.
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the wire representation of this identifier.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Reads an identifier from a message payload.
    pub fn read_from(message: &Message) -> Self {
        Self(u32::from_ne_bytes([
            message.payload[Self::OFFSET],
            message.payload[Self::OFFSET + 1],
            message.payload[Self::OFFSET + 2],
            message.payload[Self::OFFSET + 3],
        ]))
    }

    /// Writes this identifier to a message payload.
    pub fn write_to(self, message: &mut Message) {
        message.payload[Self::OFFSET..Self::OFFSET + Self::SIZE]
            .copy_from_slice(&self.0.to_ne_bytes());
    }
}
