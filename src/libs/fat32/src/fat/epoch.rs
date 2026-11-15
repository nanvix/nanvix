// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Modification epoch of a FAT filesystem.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::cell::Cell;

//==================================================================================================
// Structures
//==================================================================================================

/// A counter that advances whenever a FAT filesystem is, or may be about to be, modified.
///
/// # Description
///
/// fatfs modifies a filesystem through shared references, and the file handles that a
/// [`Fat`](super::Fat) hands out modify it without going through that [`Fat`](super::Fat) again.
/// The filesystem and every file handle opened from it therefore share one counter, and advance it
/// before they modify anything.
///
/// Two readings of the counter that are equal tell the reader that nothing modified the filesystem
/// in between. Two readings that differ tell it that something may have.
pub(super) struct Epoch {
    /// Current value of the counter.
    value: Cell<u64>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Epoch {
    /// Creates a counter that has not advanced yet.
    pub(super) const fn new() -> Self {
        Self {
            value: Cell::new(0),
        }
    }

    /// Returns the current value of the counter.
    pub(super) fn get(&self) -> u64 {
        self.value.get()
    }

    /// Advances the counter.
    pub(super) fn advance(&self) {
        // Wrapping keeps the counter total, though 2^64 modifications are not a practical concern.
        self.value.set(self.value.get().wrapping_add(1));
    }
}
