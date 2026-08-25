// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::DeliverySequence;
use ::sys::pm::ThreadIdentifier;

//==================================================================================================
// Enumerations
//==================================================================================================

/// Token for a selected IPC or lifecycle message.
pub(crate) enum UncommittedMessageToken {
    /// Selected lifecycle notification.
    Lifecycle(DeliverySequence),
    /// Selected mailbox message.
    Mailbox {
        /// Receiving thread used to select the eligible mailbox entry.
        tid: ThreadIdentifier,
        /// Sequence number of the selected mailbox entry.
        sequence: DeliverySequence,
    },
}
