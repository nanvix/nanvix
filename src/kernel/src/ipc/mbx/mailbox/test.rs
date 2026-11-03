// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::Mailbox;
use crate::pm::DeliverySequence;
use ::sys::pm::ThreadIdentifier;

//==================================================================================================
//  Test Helpers
//==================================================================================================

impl Mailbox {
    ///
    /// # Description
    ///
    /// Reports whether a delivery sequence still identifies the oldest mailbox message eligible
    /// for a thread. This exposes the mailbox token invariant to in-kernel tests.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the receiving thread.
    /// - `sequence`: Delivery sequence captured by the token under test.
    ///
    /// # Returns
    ///
    /// `true` if `sequence` identifies the current eligible message, otherwise `false`.
    ///
    pub(crate) fn test_token_is_current(
        &self,
        tid: ThreadIdentifier,
        sequence: DeliverySequence,
    ) -> bool {
        matches!(
            self.oldest_eligible(tid),
            Some((_, selected_sequence)) if selected_sequence == sequence
        )
    }
}
