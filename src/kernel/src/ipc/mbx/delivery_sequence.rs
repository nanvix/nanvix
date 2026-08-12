// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Delivery Sequence
//==================================================================================================

///
/// # Description
///
/// Monotonic sequence number assigned to an item in the ordered delivery domain.
///
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct DeliverySequence(u64);

impl DeliverySequence {
    /// Creates a delivery sequence with the given value.
    #[cfg(feature = "test")]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the next sequence number, or [`None`] if the sequence is exhausted.
    pub fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}
