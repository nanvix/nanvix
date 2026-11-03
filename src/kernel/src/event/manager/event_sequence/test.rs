// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::EventSequence;

//==================================================================================================
// Test Helpers
//==================================================================================================

impl EventSequence {
    ///
    /// # Description
    ///
    /// Creates an event sequence with an explicit value for in-kernel tests.
    ///
    /// # Parameters
    ///
    /// - `value`: Raw sequence value.
    ///
    /// # Returns
    ///
    /// An event sequence containing `value`.
    ///
    pub(in crate::event::manager) const fn new(value: u64) -> Self {
        Self(value)
    }

    ///
    /// # Description
    ///
    /// Adds an offset using wrapping arithmetic for in-kernel boundary tests.
    ///
    /// # Parameters
    ///
    /// - `value`: Offset to add to this sequence.
    ///
    /// # Returns
    ///
    /// The event sequence produced by wrapping addition.
    ///
    pub(in crate::event::manager) fn wrapping_add(self, value: u64) -> Self {
        Self(self.0.wrapping_add(value))
    }
}
