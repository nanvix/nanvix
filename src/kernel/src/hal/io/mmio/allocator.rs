// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    io::{
        IoMemoryRegion,
        MmioTag,
    },
    mem::{
        TruncatedMemoryRegion,
        VirtualAddress,
    },
};
use ::alloc::{
    collections::VecDeque,
    rc::Rc,
};
use ::core::{
    cell::RefCell,
    cmp::Ordering,
};
use ::sorted_vec::SortedVec;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::Address,
};

//==================================================================================================
// Type Aliases
//==================================================================================================

/// Shared channel for returning deallocated regions.
pub(super) type ReturnChannel =
    Rc<RefCell<VecDeque<(MmioTag, TruncatedMemoryRegion<VirtualAddress>)>>>;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A tagged MMIO region entry, ordered by its [`MmioTag`].
///
#[derive(Debug, Clone)]
struct MmioEntry {
    /// Unique tag associated with the region.
    tag: MmioTag,
    /// Backing truncated memory region.
    region: TruncatedMemoryRegion<VirtualAddress>,
}

impl PartialEq for MmioEntry {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
    }
}

impl Eq for MmioEntry {}

impl PartialOrd for MmioEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MmioEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tag.cmp(&other.tag)
    }
}

///
/// # Description
///
/// I/O memory allocator that tracks and hands out registered I/O regions.
///
/// # Notes
///
/// This allocator is expected to manage a small, bounded set of MMIO regions.
/// Accordingly, it intentionally uses [`SortedVec`] for `available` and `allocated`
/// entries to keep the representation compact and iteration simple. Insertions and
/// removals are `O(n)` due to element shifting, which is acceptable under the
/// expected small-N workload. If the number of tracked regions or the frequency of
/// `allocate()`/`reclaim()` grows substantially, these collections should be
/// revisited in favor of a tree/map-based structure.
///
pub struct IoMemoryAllocator {
    /// Regions available for allocation.
    ///
    /// Kept in a [`SortedVec`] because the allocator is expected to hold only a
    /// small number of MMIO regions.
    available: SortedVec<MmioEntry>,
    /// Currently allocated regions (tracked for overlap checking).
    ///
    /// Kept in a [`SortedVec`] for the same small-N reason as `available`.
    allocated: SortedVec<MmioEntry>,
    /// Channel for receiving returned regions.
    return_channel: ReturnChannel,
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl Default for IoMemoryAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for IoMemoryAllocator {
    fn drop(&mut self) {
        // Reclaim any pending regions before dropping.
        self.reclaim();
    }
}
//==================================================================================================
// Implementations
//==================================================================================================

impl IoMemoryAllocator {
    ///
    /// # Description
    ///
    /// Creates a new I/O memory allocator instance with an empty region list.
    ///
    /// # Returns
    ///
    /// This function returns a fresh [`IoMemoryAllocator`].
    ///
    pub fn new() -> Self {
        Self {
            available: SortedVec::new(),
            allocated: SortedVec::new(),
            return_channel: Rc::new(RefCell::new(VecDeque::new())),
        }
    }

    ///
    /// # Description
    ///
    /// Registers an I/O memory region that can be allocated later.
    ///
    /// # Parameters
    ///
    /// - `tag`: Unique tag that identifies the region.
    /// - `region`: The truncated memory region to register.
    ///
    /// # Returns
    ///
    /// This function returns `Ok(())` on success or an [`Error`] when the region is already
    /// registered.
    ///
    /// # Errors
    ///
    /// - [`ErrorCode::EntryExists`]: The region tag is already registered or the region overlaps
    ///   with an existing one.
    ///
    pub fn register(
        &mut self,
        tag: MmioTag,
        region: TruncatedMemoryRegion<VirtualAddress>,
    ) -> Result<(), Error> {
        trace!("tag={:?}, region={:?}", tag, region);

        // Reclaim any pending returned regions first.
        self.reclaim();

        // Check if tag already registered in available or allocated collections.
        if self.available.lookup_by(&tag, |entry| entry.tag).is_some()
            || self.allocated.lookup_by(&tag, |entry| entry.tag).is_some()
        {
            let reason: &str = "tag already registered";
            error!("{reason}");
            return Err(Error::new(ErrorCode::EntryExists, reason));
        }

        // Check for overlapping regions (inclusive ranges) in both collections.
        let start: usize = region.start().into_raw_value();
        let end: usize = compute_inclusive_end(start, region.size())?;

        for entry in self.available.iter().chain(self.allocated.iter()) {
            let reg_start: usize = entry.region.start().into_raw_value();
            let reg_end: usize = compute_inclusive_end(reg_start, entry.region.size())?;
            let overlaps: bool = !(end < reg_start || start > reg_end);
            if overlaps {
                let reason: &str = "region overlaps existing entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::EntryExists, reason));
            }
        }

        self.available.insert(MmioEntry { tag, region });
        trace!("registered mmio region: tag={:?}", tag);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Allocates an I/O address from the memory allocator.
    ///
    /// # Parameters
    ///
    /// - `tag`: The unique tag of the I/O region to allocate.
    ///
    /// # Returns
    ///
    /// This function returns the corresponding [`IoMemoryRegion`] when the tag is registered and
    /// not already allocated.
    ///
    /// # Errors
    ///
    /// - [`ErrorCode::EntryExists`]: The region is already allocated.
    /// - [`ErrorCode::NoSuchEntry`]: The region is not registered.
    ///
    pub fn allocate(&mut self, tag: MmioTag) -> Result<IoMemoryRegion, Error> {
        // Reclaim any pending returned regions first.
        self.reclaim();

        // Try to move region from available to allocated.
        match self.available.remove_by(&tag, |entry| entry.tag) {
            Some(entry) => {
                let region: TruncatedMemoryRegion<VirtualAddress> = entry.region.clone();
                self.allocated.insert(entry);
                Ok(IoMemoryRegion::new(tag, region, Rc::clone(&self.return_channel)))
            },
            None => {
                // Check if it's already allocated or simply not registered.
                if self.allocated.lookup_by(&tag, |entry| entry.tag).is_some() {
                    let reason: &str = "region already allocated";
                    error!("{reason}");
                    Err(Error::new(ErrorCode::EntryExists, reason))
                } else {
                    // NOTE: This is logged at `warn!` level to make absent regions visible during
                    // optional probes (e.g. RAMFS probe when nanvixd is launched without
                    // `-ramfs`). Callers receive `ErrorCode::NoSuchEntry` and may still treat this
                    // as fatal.
                    let reason: &str = "region not registered";
                    warn!("{reason}: tag={:?}", tag);
                    Err(Error::new(ErrorCode::NoSuchEntry, reason))
                }
            },
        }
    }

    ///
    /// # Description
    ///
    /// Reclaims regions that have been returned via the return channel.
    ///
    /// This function processes all pending returned regions and moves them back to the available
    /// pool. It is called automatically during allocation and registration operations.
    ///
    fn reclaim(&mut self) {
        let mut channel: core::cell::RefMut<
            '_,
            VecDeque<(MmioTag, TruncatedMemoryRegion<VirtualAddress>)>,
        > = self.return_channel.borrow_mut();
        while let Some((tag, region)) = channel.pop_front() {
            trace!("reclaiming region: tag={:?}", tag);
            // Remove from allocated and add back to available.
            self.allocated.remove_by(&tag, |entry| entry.tag);
            self.available.insert(MmioEntry { tag, region });
        }
    }

    ///
    /// # Description
    ///
    /// Returns the number of registered I/O memory regions (both available and allocated).
    ///
    /// # Returns
    ///
    /// The total count of registered regions.
    ///
    #[must_use]
    pub fn len(&self) -> usize {
        self.available.len() + self.allocated.len()
    }

    ///
    /// # Description
    ///
    /// Checks whether the allocator has no registered regions.
    ///
    /// # Returns
    ///
    /// `true` if there are no registered regions, `false` otherwise.
    ///
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.available.is_empty() && self.allocated.is_empty()
    }

    ///
    /// # Description
    ///
    /// Returns the number of available (unallocated) I/O memory regions.
    ///
    /// # Returns
    ///
    /// The count of available regions.
    ///
    #[must_use]
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    ///
    /// # Description
    ///
    /// Returns the number of currently allocated I/O memory regions.
    ///
    /// # Returns
    ///
    /// The count of allocated regions.
    ///
    #[must_use]
    pub fn allocated_count(&self) -> usize {
        self.allocated.len()
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the inclusive end address of a memory region.
///
/// # Parameters
///
/// - `start`: The start address of the region.
/// - `size`: The size of the region in bytes.
///
/// # Returns
///
/// This function returns the inclusive end address on success, or an [`Error`] if arithmetic
/// overflow or underflow occurs.
///
fn compute_inclusive_end(start: usize, size: usize) -> Result<usize, Error> {
    let size_minus_one: usize = match size.checked_sub(1) {
        Some(val) => val,
        None => {
            let reason: &str = "region size underflow";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        },
    };
    match start.checked_add(size_minus_one) {
        Some(end) => Ok(end),
        None => {
            let reason: &str = "region end address overflow";
            error!("{reason}");
            Err(Error::new(ErrorCode::InvalidArgument, reason))
        },
    }
}
