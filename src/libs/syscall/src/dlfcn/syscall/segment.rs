// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use ::core::ptr::{
    self,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall,
    kcall::mm::{
        mmap,
        munmap,
    },
    mm::{
        AccessPermission,
        Address,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
};
use ::sysalloc::{
    PAGE_ALIGNMENT,
    PAGE_SIZE,
};

//==================================================================================================
// MemorySegment
//==================================================================================================

///
/// # Description
///
/// A structure that represents a memory segment.
///
#[derive(Debug)]
pub struct MemorySegment {
    /// Identifier of the process that owns the segment.
    pid: ProcessIdentifier,
    /// Base address.
    base: VirtualAddress,
    /// Capacity of the segment.
    capacity: usize,
}

impl MemorySegment {
    /// Creates a new memory segment.
    pub fn new(base: VirtualAddress, capacity: usize) -> Result<Self, Error> {
        ::syslog::trace!("new(): base={:#x?}, capacity={:?}", base.into_raw_value(), capacity);

        // Check if base address is not page-aligned.
        if !base.is_aligned(PAGE_ALIGNMENT) {
            let reason: &str = "unaligned base address";
            ::syslog::error!("new(): {}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Check if capacity is zero.
        if capacity == 0 {
            let reason: &str = "zero capacity";
            ::syslog::error!("new(): {}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Check if capacity is page-aligned.
        if capacity % PAGE_SIZE != 0 {
            let reason: &str = "unaligned capacity";
            ::syslog::error!("new(): {}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        let pid: ProcessIdentifier = kcall::pm::getpid()?;

        map_range(pid, base, VirtualAddress::from_raw_value(base.into_raw_value() + capacity))?;

        Ok(MemorySegment {
            pid,
            base,
            capacity,
        })
    }

    /// Loads data into the target memory segment.
    pub fn load(&mut self, offset: usize, bytes: &[u8]) -> Result<(), Error> {
        ::syslog::trace!(
            "load(): base={:#x?}, offset={:#x?}, bytes.len={:?}",
            self.base.into_raw_value(),
            offset,
            bytes.len()
        );
        // Check if bytes exceed capacity.
        if offset + bytes.len() > self.capacity {
            let reason: &str = "bytes exceed capacity";
            ::syslog::error!("load(): {}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Copy data.
        // SAFETY: the following unsafe block is safe because:
        // - `src` is valid for reads of `count` bytes.
        // - `dst` is valid for writes of `count` bytes.
        // - Both `src` and `dst` are properly aligned.
        // - The region of memory beginning at `src` with a size of `count` bytes does not overlap
        //   with the region of memory beginning at `dst` with the same size.
        unsafe {
            let base_addr: usize = bytes.as_ptr() as usize;
            let src: usize = base_addr;
            let dst: usize = offset + self.base.into_raw_value();
            let count: usize = bytes.len();
            ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, count);
        }

        Ok(())
    }
}

impl Drop for MemorySegment {
    fn drop(&mut self) {
        ::syslog::trace!("drop(): base={:X?}, capacity={:X?}", self.base, self.capacity);

        // Unmap pages.
        if let Err(_error) = unmap_range(
            self.pid,
            self.base,
            VirtualAddress::from_raw_value(self.base.into_raw_value() + self.capacity),
        ) {
            ::syslog::warn!("drop(): failed to unmap pages (error={:?})", _error);
        }
    }
}

/// Map pages in the range [start, end).
fn map_range(
    pid: ProcessIdentifier,
    start: VirtualAddress,
    end: VirtualAddress,
) -> Result<(), Error> {
    ::syslog::trace!("map_range(): start={:X?}, end={:X?}", start, end);

    debug_assert!(start.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(end.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(start < end);

    // TODO: use iterator.
    let start: usize = start.into_raw_value();
    let end: usize = end.into_raw_value();
    for vaddr in (start..end).step_by(PAGE_SIZE) {
        debug_assert!(vaddr != end);

        // Attempt to map page.
        let vaddr: VirtualAddress = VirtualAddress::new(vaddr);
        if let Err(error) = mmap(pid, vaddr, AccessPermission::RDWR) {
            // Failed to map page, attempt to rollback.

            ::syslog::error!(
                "map_range(): failed to map page at {:X?}, rolling back (error={:?})",
                vaddr,
                error
            );

            // Attempt to unmap pages.
            if let Err(_error) = unmap_range(pid, VirtualAddress::new(start), vaddr) {
                // Failed to unmap range, warn.
                ::syslog::warn!(
                    "map_range(): failed to unmap pages at {:X?}..{:X?} (error={:?})",
                    start,
                    vaddr,
                    _error
                );
            }

            return Err(error);
        }

        // NOTE: pages allocated with mmap() are always zeroed.
    }

    Ok(())
}

/// Unmap pages in the range [start, end).
fn unmap_range(
    pid: ProcessIdentifier,
    start: VirtualAddress,
    end: VirtualAddress,
) -> Result<(), Error> {
    ::syslog::trace!("unmap_range(): start={:X?}, end={:X?}", start, end);

    debug_assert!(start.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(end.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(start < end);

    let mut ret: Result<(), Error> = Ok(());
    let start: usize = start.into_raw_value();
    let end: usize = end.into_raw_value();
    for vaddr in (start..end).step_by(PAGE_SIZE) {
        debug_assert!(vaddr != end);

        let vaddr: VirtualAddress = VirtualAddress::from_raw_value(vaddr);

        if let Err(error) = munmap(pid, vaddr) {
            ::syslog::error!(
                "unmap_range(): failed to unmap page at {:X?}, skipping (error={:?})",
                vaddr,
                error
            );

            // Save error.
            ret = Err(error);
        }
    }

    ret
}
