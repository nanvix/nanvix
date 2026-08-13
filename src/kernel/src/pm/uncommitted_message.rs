// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::uncommitted_message_token::UncommittedMessageToken;
use ::sys::ipc::Message;

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

///
/// # Description
///
/// Creates an uncommitted message for in-kernel tests outside the process-management module.
///
/// # Parameters
///
/// - `message`: Message selected by the test fixture.
/// - `token`: Token associated with the selected test item.
///
/// # Returns
///
/// An uncommitted message containing `message` and `token`.
///
#[cfg(feature = "test")]
pub(crate) fn new_test_message(
    message: Message,
    token: UncommittedMessageToken,
) -> UncommittedMessage {
    UncommittedMessage::new(message, token)
}
