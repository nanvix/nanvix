// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Delivery Epoch
//==================================================================================================

/// Generation that identifies the currently valid delivery transaction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct DeliveryEpoch(u64);

impl DeliveryEpoch {
    ///
    /// # Description
    ///
    /// Computes the delivery epoch that follows this one without wrapping.
    ///
    /// # Returns
    ///
    /// The next delivery epoch, or [`None`] if this epoch is the terminal value.
    ///
    pub(super) fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}
