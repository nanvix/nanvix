// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    io::{
        mmio::allocator::ReturnChannel,
        MmioTag,
    },
    mem::{
        AccessPermission,
        PageAligned,
        TruncatedMemoryRegion,
        VirtualAddress,
    },
};
use ::core::mem::ManuallyDrop;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A handle to an allocated I/O memory region. When dropped, the region is automatically returned
/// to the allocator's pool of available regions.
///
/// The region is guaranteed to be valid for the entire lifetime of this handle. The inner data is
/// wrapped in [`ManuallyDrop`] to allow taking ownership during [`Drop`] without requiring
/// `Option`.
///
pub struct IoMemoryRegion {
    /// Unique tag associated with the region.
    tag: MmioTag,
    /// Backing truncated memory region wrapped in ManuallyDrop for ownership transfer in Drop.
    region: ManuallyDrop<TruncatedMemoryRegion<VirtualAddress>>,
    /// Channel for returning the region to the allocator on drop.
    return_channel: ReturnChannel,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl IoMemoryRegion {
    ///
    /// # Description
    ///
    /// Returns the unique tag associated with the MMIO region.
    ///
    /// # Returns
    ///
    /// This function returns the [`MmioTag`] that identifies the region.
    ///
    #[must_use]
    pub fn tag(&self) -> MmioTag {
        self.tag
    }

    ///
    /// # Description
    ///
    /// Returns the base address of the MMIO region.
    ///
    /// # Returns
    ///
    /// This function returns the page-aligned base [`VirtualAddress`].
    ///
    #[must_use]
    pub fn base(&self) -> PageAligned<VirtualAddress> {
        self.region.start()
    }

    ///
    /// # Description
    ///
    /// Returns the access permissions of the MMIO region.
    ///
    /// # Returns
    ///
    /// This function returns the [`AccessPermission`] associated with the region.
    ///
    #[must_use]
    pub fn perm(&self) -> AccessPermission {
        self.region.perm()
    }

    ///
    /// # Description
    ///
    /// Returns the size in bytes of the MMIO region.
    ///
    /// # Returns
    ///
    /// This function returns the size, in bytes, of the region.
    ///
    #[must_use]
    pub fn size(&self) -> usize {
        self.region.size()
    }

    ///
    /// # Description
    ///
    /// Creates a new MMIO region wrapper from a truncated memory region.
    ///
    /// # Parameters
    ///
    /// - `tag`: Unique tag that identifies the region.
    /// - `region`: The truncated memory region to wrap.
    /// - `return_channel`: Channel for returning the region to the allocator.
    ///
    /// # Returns
    ///
    /// This function returns a new [`IoMemoryRegion`].
    ///
    pub(super) fn new(
        tag: MmioTag,
        region: TruncatedMemoryRegion<VirtualAddress>,
        return_channel: ReturnChannel,
    ) -> Self {
        Self {
            tag,
            region: ManuallyDrop::new(region),
            return_channel,
        }
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl Drop for IoMemoryRegion {
    fn drop(&mut self) {
        // SAFETY: This is the Drop implementation, so no other code can access `self.region`
        // after this point. We take ownership of the inner value to return it to the allocator.
        let region: TruncatedMemoryRegion<VirtualAddress> =
            unsafe { ManuallyDrop::take(&mut self.region) };
        trace!("returning region to allocator: tag={:?}", self.tag);
        self.return_channel
            .borrow_mut()
            .push_back((self.tag, region));
    }
}

impl core::fmt::Debug for IoMemoryRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        write!(f, "{:?} @ {:?}", self.tag(), self.base())
    }
}

impl PartialEq for IoMemoryRegion {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
    }
}

impl Eq for IoMemoryRegion {}

impl PartialOrd for IoMemoryRegion {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IoMemoryRegion {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.tag.cmp(&other.tag)
    }
}
