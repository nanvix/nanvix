// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// Test-only fixtures for in-kernel tests.
#[cfg(feature = "test")]
mod test;

//==================================================================================================
// Imports
//==================================================================================================

use super::uncommitted_message_token::UncommittedMessageToken;
use ::sys::ipc::Message;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "test")]
pub(crate) use test::new_test_message;

//==================================================================================================
// Structures
//==================================================================================================

/// Message selected from the ordered IPC and lifecycle delivery domain.
pub(crate) struct UncommittedMessage {
    /// Message to copy to user space.
    message: Message,
    /// Token that commits the selected message after a successful copy.
    token: UncommittedMessageToken,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl UncommittedMessage {
    ///
    /// # Description
    ///
    /// Creates an uncommitted message from selected message data and its private commit token.
    ///
    /// # Parameters
    ///
    /// - `message`: Message selected for delivery.
    /// - `token`: Token that commits the selected mailbox or lifecycle item.
    ///
    /// # Returns
    ///
    /// An uncommitted message containing `message` and `token`.
    ///
    pub(super) fn new(message: Message, token: UncommittedMessageToken) -> Self {
        Self { message, token }
    }

    ///
    /// # Description
    ///
    /// Decomposes this uncommitted selection into its message and private commit token.
    ///
    /// # Returns
    ///
    /// The selected message and the token that commits it.
    ///
    pub(crate) fn into_parts(self) -> (Message, UncommittedMessageToken) {
        (self.message, self.token)
    }
}
