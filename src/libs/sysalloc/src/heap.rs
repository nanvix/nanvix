// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::{
    mem,
    mem::PAGE_ALIGNMENT,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall,
    mm::{
        self,
        AccessPermission,
        Address,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
//  Structures
//==================================================================================================

pub struct Heap {
    base: VirtualAddress,
    size: usize,
    capacity: usize,
}

impl Heap {
    pub fn new(base: VirtualAddress, size: usize, capacity: usize) -> Result<Self, Error> {
        ::syslog::trace!("new(): base={:X?}, size={:X?}, capacity={:X?}", base, size, capacity);

        // Check if base address is page-aligned.
        if !base.is_aligned(PAGE_ALIGNMENT) {
            ::syslog::warn!("new(): unaligned base address {:X?}", base);
            return Err(Error::new(ErrorCode::BadAddress, "unaligned base address"));
        }

        // Check if size is zero.
        if size == 0 {
            ::syslog::warn!("new(): zero size");
            return Err(Error::new(ErrorCode::BadAddress, "zero size"));
        }

        // Check if capacity is zero.
        if capacity == 0 {
            ::syslog::warn!("new(): zero capacity");
            return Err(Error::new(ErrorCode::BadAddress, "zero capacity"));
        }

        // Check if capacity is smaller than size.
        if capacity < size {
            ::syslog::warn!("new(): capacity is too small");
            return Err(Error::new(ErrorCode::BadAddress, "capacity is too small"));
        }

        // Map initial pages.
        //
        // Resolve the owning pid with an UNCACHED kernel query. The kernel rejects an mmap whose
        // target pid differs from the calling process (its memory-management capability check), so a
        // mapping must always name the current process. The cached getpid() offers no guarantee
        // here: the pid cache is a per-link-unit `static`, and a C-linked image ends up with several
        // independent copies of it (cpython, for example, links libposix.a and libnvx_crt0.a, each
        // compiled with its own `sys` crate and therefore its own cache). fork() can invalidate only
        // the copy in its own link unit, so it cannot guarantee that the copy this mapping consults
        // was refreshed; that copy may still hold the parent's pid inherited through copy-on-write
        // memory. Querying the kernel directly is independent of every cached copy.
        //
        // Tracked by https://github.com/nanvix/nanvix/issues/2622: once a process image is
        // guaranteed to carry a single, unified pid cache that fork() invalidates, restore the
        // cached getpid() here.
        let pid: ProcessIdentifier = kcall::pm::getpid_uncached()?;
        let start: VirtualAddress = base;
        let end: VirtualAddress = VirtualAddress::from_raw_value(base.into_raw_value() + size);
        map_range(pid, start, end)?;

        Ok(Self {
            base,
            size,
            capacity,
        })
    }

    pub fn base(&self) -> VirtualAddress {
        self.base
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn grow(&mut self, increment: usize) -> Result<(), Error> {
        ::syslog::trace!("grow(): increment={:X?}", increment);

        // Check if increment is page-aligned.
        if !mm::is_aligned(increment, PAGE_ALIGNMENT) {
            ::syslog::warn!("grow(): unaligned increment");
            return Err(Error::new(ErrorCode::BadAddress, "unaligned increment"));
        }

        // Check if increment is zero.
        if increment == 0 {
            ::syslog::warn!("grow(): zero increment");
            return Err(Error::new(ErrorCode::BadAddress, "zero increment"));
        }

        // Check if increment would exceed capacity.
        if self.size + increment > self.capacity {
            ::syslog::warn!(
                "grow(): exceeds capacity (self.size={:X?}, increment={:X?}, self.capacity={:X?})",
                self.size,
                increment,
                self.capacity
            );
            return Err(Error::new(ErrorCode::BadAddress, "exceeds capacity"));
        }

        // Map pages.
        //
        // Resolve the owning pid with an UNCACHED kernel query; see new() for why the cached
        // getpid() cannot be trusted for a capability-sensitive mapping.
        let pid: ProcessIdentifier = kcall::pm::getpid_uncached()?;
        let end: VirtualAddress = self.base + self.size;
        let new_end: VirtualAddress = end + increment;
        map_range(pid, end, new_end)?;

        // Update metadata.
        self.size += increment;

        Ok(())
    }

    /// Shrinks the heap by unmapping tail pages from the end backwards.
    ///
    /// Pages are unmapped in reverse order (highest address first). If a `munmap` call fails,
    /// unmapping stops immediately and `self.size` is updated to reflect the lowest still-mapped
    /// boundary. This guarantees that `self.size` always represents the actual contiguous mapped
    /// extent, preventing inconsistencies between metadata and the page table.
    ///
    /// # Parameters
    ///
    /// - `new_size`: New committed size in bytes. Must be page-aligned and smaller than the
    ///   current size. Values below a single page are clamped.
    pub fn shrink(&mut self, new_size: usize) -> Result<(), Error> {
        ::syslog::trace!("shrink(): new_size={:X?}, current_size={:X?}", new_size, self.size);

        // Check if new size is page-aligned.
        if !mm::is_aligned(new_size, PAGE_ALIGNMENT) {
            ::syslog::warn!("shrink(): unaligned new_size");
            return Err(Error::new(ErrorCode::BadAddress, "unaligned new_size"));
        }

        // Clamp values below a single page.
        let new_size: usize = new_size.max(mem::PAGE_SIZE);

        // Nothing to do if the (possibly clamped) new size is not smaller.
        if new_size >= self.size {
            return Ok(());
        }

        // Resolve the owning pid with an UNCACHED kernel query; see new() for why the cached
        // getpid() cannot be trusted for a capability-sensitive unmapping.
        let pid: ProcessIdentifier = kcall::pm::getpid_uncached()?;

        // Unmap tail pages from the end backwards, stopping at the first failure so that
        // self.size always reflects the actual contiguous mapped extent.
        let base_raw: usize = self.base.into_raw_value();
        let new_end: usize = base_raw + new_size;
        let old_end: usize = base_raw + self.size;

        for page in (new_end..old_end).step_by(mem::PAGE_SIZE).rev() {
            let page_addr: VirtualAddress = VirtualAddress::from_raw_value(page);
            if let Err(error) = kcall::mm::__kcall_munmap(pid, page_addr) {
                ::syslog::warn!(
                    "shrink(): failed to unmap page at {:X?}, stopping (error={:?})",
                    page_addr,
                    error
                );
                self.size = (page + mem::PAGE_SIZE) - base_raw;
                return Err(error);
            }
        }

        // All tail pages unmapped successfully.
        self.size = new_size;
        Ok(())
    }
}

/// Map pages in the range [start, end).
pub fn map_range(
    pid: ProcessIdentifier,
    start: VirtualAddress,
    end: VirtualAddress,
) -> Result<(), Error> {
    ::syslog::trace!("map_range(): start={:X?}, end={:X?}", start, end);

    debug_assert!(start.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(end.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(start < end);

    // If the range is empty, there's nothing to do.
    if start == end {
        return Ok(());
    }

    // Check if start is greater than end to prevent underflow in size calculation. This also guards
    // against invalid ranges where the end address wraps around the address space.
    if start > end {
        let reason: &str = "map_range(): start address is greater than end address";
        ::syslog::warn!("{reason}");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let start: usize = start.into_raw_value();
    let end: usize = end.into_raw_value();
    let npages: usize = (end - start) / mem::PAGE_SIZE;

    // Use batch mmap kcall to map all pages in a single kernel transition.
    if let Err(error) =
        kcall::mm::__kcall_mmap(pid, VirtualAddress::new(start), npages, AccessPermission::RDWR)
    {
        ::syslog::warn!(
            "map_range(): batch mmap failed at {:X?}..{:X?}, npages={} (error={:?})",
            start,
            end,
            npages,
            error
        );
        return Err(error);
    }

    // NOTE: pages allocated with mmap() are always zeroed.

    Ok(())
}

/// Unmap pages in the range [start, end).
pub fn unmap_range(
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
    for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
        debug_assert!(vaddr != end);

        let vaddr: VirtualAddress = VirtualAddress::from_raw_value(vaddr);

        if let Err(error) = kcall::mm::__kcall_munmap(pid, vaddr) {
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
impl Drop for Heap {
    fn drop(&mut self) {
        ::syslog::trace!(
            "drop(): base={:X?}, size={:X?}, capacity={:X?}",
            self.base,
            self.size,
            self.capacity
        );

        // Resolve the owning pid with an UNCACHED kernel query; see new() for the rationale. Drop
        // cannot propagate errors, so skip unmapping if the pid cannot be determined.
        let pid: ProcessIdentifier = match kcall::pm::getpid_uncached() {
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
            VirtualAddress::from_raw_value(self.base.into_raw_value() + self.size),
        ) {
            ::syslog::warn!("drop(): failed to unmap pages (error={:?})", _error);
        }
    }
}
