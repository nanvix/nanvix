// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::UncommittedMessage;
use crate::pm::uncommitted_message_token::UncommittedMessageToken;
use ::sys::ipc::Message;

//==================================================================================================
// Standalone Functions
//==================================================================================================

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
pub(crate) fn new_test_message(
    message: Message,
    token: UncommittedMessageToken,
) -> UncommittedMessage {
    UncommittedMessage::new(message, token)
}
