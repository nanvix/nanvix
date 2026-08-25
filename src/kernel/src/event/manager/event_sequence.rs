// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    #[cfg(feature = "test")]
    pub(super) const fn new(value: u64) -> Self {
        Self(value)
    }

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
    #[cfg(feature = "test")]
    pub(super) fn wrapping_add(self, value: u64) -> Self {
        Self(self.0.wrapping_add(value))
    }
}
