// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// Test-only constructors and arithmetic for in-kernel tests.
#[cfg(feature = "test")]
mod test;

//==================================================================================================
// Event Sequence
//==================================================================================================

/// Monotonic sequence number assigned to an interrupt or exception event.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct EventSequence(u64);

impl EventSequence {
    ///
    /// # Description
    ///
    /// Computes the event sequence that follows this one without wrapping.
    ///
    /// # Returns
    ///
    /// The next event sequence, or [`None`] if this sequence is the terminal value.
    ///
    pub(super) fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }

    ///
    /// # Description
    ///
    /// Converts this full-width sequence into the target-width value stored in an event
    /// descriptor identifier.
    ///
    /// # Returns
    ///
    /// This sequence truncated to the width of [`usize`].
    ///
    pub(super) fn descriptor_id(self) -> usize {
        self.0 as usize
    }
}
