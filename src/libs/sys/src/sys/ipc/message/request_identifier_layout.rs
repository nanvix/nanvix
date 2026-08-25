// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::SystemMessageHeader;
use ::core::mem;

//==================================================================================================
// Structures
//==================================================================================================

/// Layout witness for the common wire prefix of messages that carry a request identifier.
#[repr(C, packed)]
struct RequestIdentifierWirePrefix {
    _system_message_header: [u8; mem::size_of::<SystemMessageHeader>()],
    _protocol_message_header: [u8; mem::size_of::<SystemMessageHeader>()],
    _identifier: u32,
}

//==================================================================================================
// Constants
//==================================================================================================

/// Byte offset of the request identifier in a correlated message payload.
pub const REQUEST_IDENTIFIER_OFFSET: usize =
    mem::offset_of!(RequestIdentifierWirePrefix, _identifier);

/// Size of the request identifier in bytes.
pub(crate) const REQUEST_IDENTIFIER_SIZE: usize = mem::size_of::<u32>();

/// Size of the complete correlated-message prefix in bytes.
pub const REQUEST_IDENTIFIER_PREFIX_SIZE: usize = mem::size_of::<RequestIdentifierWirePrefix>();

::static_assert::assert_eq_size!(
    RequestIdentifierWirePrefix,
    REQUEST_IDENTIFIER_OFFSET + REQUEST_IDENTIFIER_SIZE
);
