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
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct DeliverySequence(u64);

impl DeliverySequence {
    ///
    /// # Description
    ///
    /// Creates a delivery sequence with an explicit value for the owning delivery broker.
    ///
    /// # Parameters
    ///
    /// - `value`: Raw sequence value.
    ///
    /// # Returns
    ///
    /// A delivery sequence containing `value`.
    ///
    pub(super) const fn new(value: u64) -> Self {
        Self(value)
    }

    ///
    /// # Description
    ///
    /// Computes the delivery sequence that follows this one without wrapping.
    ///
    /// # Returns
    ///
    /// The next delivery sequence, or [`None`] if this sequence is the terminal value.
    ///
    // `map` uses an explicit closure instead of the `Self` tuple-struct
    // constructor as a bare function value, which the Verus frontend cannot lower.
    #[allow(clippy::redundant_closure)]
    pub(super) fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(|value| Self(value))
    }
}
