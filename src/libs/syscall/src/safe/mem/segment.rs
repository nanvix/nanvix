// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use ::arch::mem::{
    PAGE_ALIGNMENT,
    PAGE_SIZE,
};
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
        __kcall_mmap,
        __kcall_mprotect,
        __kcall_munmap,
    },
    mm::{
        AccessPermission,
        Address,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
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
    /// Base address.
    base: VirtualAddress,
    /// Capacity of the segment.
    capacity: usize,
}

impl MemorySegment {
    ///
    /// # Description
    ///
    /// Creates a new memory segment.
    ///
    /// # Parameters
    ///
    /// - `base`: Base address of the segment.
    /// - `capacity`: Capacity of the segment in bytes.
    /// - `access`: Access permissions for the segment.
    ///
    /// # Returns
    ///
    /// On success, this function returns a `MemorySegment` with the specified base address and capacity.
    /// On failure, it returns an `Error` indicating the reason for the failure.
    ///
    pub fn new(
        base: VirtualAddress,
        capacity: usize,
        access: AccessPermission,
    ) -> Result<Self, Error> {
        ::syslog::trace!(
            "new(): base={:#x?}, capacity={:?}, access={:?}",
            base.into_raw_value(),
            capacity,
            access
        );

        // Check if base address is not page-aligned.
        if !base.is_aligned(PAGE_ALIGNMENT) {
            let reason: &str = "unaligned base address";
            ::syslog::warn!("new(): {}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Check if capacity is zero.
        if capacity == 0 {
            let reason: &str = "zero capacity";
            ::syslog::warn!("new(): {}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Check if capacity is page-aligned.
        if !capacity.is_multiple_of(PAGE_SIZE) {
            let reason: &str = "unaligned capacity";
            ::syslog::warn!("new(): {}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Resolve the owning pid. The kernel rejects an mmap whose target pid differs from the
        // calling process (its memory-management capability check), so the mapping must name the
        // current process. The cached getpid() is correct here: CACHED_PID is a single per-image
        // instance, and every fork path invalidates it, so a forked child re-resolves to its own
        // identity before mapping.
        let pid: ProcessIdentifier = kcall::pm::getpid()?;

        map_range(
            pid,
            base,
            VirtualAddress::from_raw_value(base.into_raw_value() + capacity),
            access,
        )?;

        Ok(MemorySegment { base, capacity })
    }

    ///
    /// # Description
    ///
    /// Returns the base address of the memory segment.
    ///
    /// # Returns
    ///
    /// The base address of the memory segment.
    ///
    pub fn base(&self) -> VirtualAddress {
        self.base
    }

    ///
    /// # Description
    ///
    /// Returns the capacity of the memory segment.
    ///
    /// # Returns
    ///
    /// The capacity of the memory segment in bytes.
    ///
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    ///
    /// # Description
    ///
    /// Loads data into the target memory segment.
    ///
    /// # Parameters
    ///
    /// - `offset`: Offset in the segment where the data should be loaded.
    /// - `bytes`: Slice of bytes to be loaded into the segment.
    ///
    /// # Returns
    ///
    /// On success, this function returns empty. On failure, it returns an `Error` indicating the
    /// reason for the failure.
    ///
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
            ::syslog::warn!("load(): {}", reason);
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

    ///
    /// # Description
    ///
    /// Changes protection of the memory segment.
    ///
    /// # Parameters
    ///
    /// - `prot`: New protection flags.
    ///
    /// # Returns
    ///
    /// On success, this function returns empty. On failure, it returns an `Error` indicating the
    /// reason for the failure.
    ///
    pub fn set_protection(&mut self, prot: AccessPermission) -> Result<(), Error> {
        ::syslog::trace!(
            "set_protection(): base={:#x?}, capacity={:?}, prot={:?}",
            self.base.into_raw_value(),
            self.capacity,
            prot
        );

        // Resolve the owning pid; see new() for why the cached getpid() is correct for a
        // capability-sensitive mapping.
        let pid: ProcessIdentifier = kcall::pm::getpid()?;

        protect_range(
            pid,
            self.base,
            VirtualAddress::from_raw_value(self.base.into_raw_value() + self.capacity),
            prot,
        )
    }
}

impl Drop for MemorySegment {
    fn drop(&mut self) {
        ::syslog::trace!("drop(): base={:X?}, capacity={:X?}", self.base, self.capacity);

        // Resolve the owning pid; see new() for the rationale. Drop cannot propagate errors, so
        // skip unmapping if the pid cannot be determined.
        let pid: ProcessIdentifier = match kcall::pm::getpid() {
            Ok(pid) => pid,
            Err(_error) => {
                ::syslog::warn!("drop(): failed to query pid, skipping unmap (error={:?})", _error);
                return;
            },
        };

        // Unmap pages.
        if let Err(_error) = unmap_range(
            pid,
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
    access: AccessPermission,
) -> Result<(), Error> {
    ::syslog::trace!("map_range(): start={start:X?}, end={end:X?}, access={access:?}");

    debug_assert!(start.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(end.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(start < end);

    let start: usize = start.into_raw_value();
    let end: usize = end.into_raw_value();

    // Compute the length of the range using checked arithmetic to avoid wraparound.
    let len: usize = match end.checked_sub(start) {
        Some(len) if len != 0 && len % PAGE_SIZE == 0 => len,
        _ => {
            ::syslog::warn!("map_range(): invalid range {:X?}..{:X?}", start, end);

            return Err(Error::new(ErrorCode::InvalidArgument, "invalid range"));
        },
    };

    // Compute the number of pages to map in a single kernel transition.
    let npages: usize = len / PAGE_SIZE;

    // Attempt to map all pages at once.
    // NOTE: pages allocated with __kcall_mmap() are always zeroed.
    if let Err(error) = __kcall_mmap(pid, VirtualAddress::new(start), npages, access) {
        ::syslog::warn!(
            "map_range(): failed to map pages at {:X?}..{:X?} (error={:?})",
            start,
            end,
            error
        );

        return Err(error);
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

        if let Err(error) = __kcall_munmap(pid, vaddr) {
            ::syslog::warn!(
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

/// Changes protection of pages in the range [start, end).
fn protect_range(
    pid: ProcessIdentifier,
    start: VirtualAddress,
    end: VirtualAddress,
    prot: AccessPermission,
) -> Result<(), Error> {
    ::syslog::trace!("protect_range(): start={:X?}, end={:X?}, prot={:?}", start, end, prot);

    debug_assert!(start.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(end.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(start < end);

    let start: usize = start.into_raw_value();
    let end: usize = end.into_raw_value();
    for vaddr in (start..end).step_by(PAGE_SIZE) {
        debug_assert!(vaddr != end);

        let vaddr: VirtualAddress = VirtualAddress::from_raw_value(vaddr);
        if let Err(error) = __kcall_mprotect(pid, vaddr, prot) {
            ::syslog::warn!(
                "protect_range(): failed to change protection of page at {:X?}, skipping \
                 (error={:?})",
                vaddr,
                error
            );
            return Err(error);
        }
    }

    Ok(())
}
