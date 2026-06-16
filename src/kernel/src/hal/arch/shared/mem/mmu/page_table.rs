// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::{
    FrameAddress,
    PageTableAddress,
    Table,
};
use ::arch::mem::{
    paging::{
        AccessedFlag,
        DirtyFlag,
        FrameNumber,
        PageCacheDisableFlag,
        PageTableEntry,
        PageTableEntryFlags,
        PageWriteThroughFlag,
        PresentFlag,
        ReadWriteFlag,
        TableIndex,
        UserSupervisorFlag,
    },
    PAGE_TABLE_LENGTH,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Type Aliases
//==================================================================================================

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A view over a hardware page table.
///
/// Wraps a [`Table<PageTableEntry>`] for typed entry access. Does not own the backing
/// memory — the caller is responsible for keeping the underlying page alive.
///
pub struct PageTable {
    /// Number of pages mapped in the page table.
    nmapped: usize,
    /// Typed table view over the backing storage.
    table: Table<PageTableEntry>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl PageTable {
    ///
    /// # Description
    ///
    /// Creates a page table view from a raw base address.
    ///
    /// # Safety
    ///
    /// `base` must be a valid, page-aligned, identity-mapped address backed by at least one
    /// page of writable memory that outlives this `PageTable`.
    ///
    pub unsafe fn from_address(base: PageTableAddress) -> Self {
        Self {
            nmapped: 0,
            table: Table::from_address(base.into_raw_value()),
        }
    }

    /// Zeroes all entries in this page table.
    pub fn clean(&mut self) {
        for i in 0..PAGE_TABLE_LENGTH {
            let idx = TableIndex::try_from(i).expect("index within bounds");
            // SAFETY: index is within bounds.
            unsafe {
                self.table.write(
                    idx,
                    PageTableEntry::new(
                        PageTableEntryFlags::new(
                            PresentFlag::NotPresent,
                            ReadWriteFlag::ReadOnly,
                            UserSupervisorFlag::Supervisor,
                            PageWriteThroughFlag::NotWriteThrough,
                            PageCacheDisableFlag::CacheDisabled,
                            AccessedFlag::NotAccessed,
                            DirtyFlag::NotDirty,
                        ),
                        FrameNumber::NULL,
                    ),
                )
            };
        }
        self.nmapped = 0;
    }

    ///
    /// # Description
    ///
    /// Maps a single page table entry.
    ///
    /// # Parameters
    ///
    /// - `pte_idx`: Index of the PTE to write.
    /// - `frame`: Physical frame address to map.
    /// - `supervisor`: Whether the page is supervisor-only.
    /// - `writable`: Whether the page is writable.
    /// - `writethrough`: Whether write-through caching is enabled.
    /// - `cache_disabled`: Whether caching is disabled.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if the entry is already present.
    ///
    pub fn map_entry(
        &mut self,
        pte_idx: TableIndex,
        frame: FrameAddress,
        supervisor: bool,
        writable: bool,
        writethrough: bool,
        cache_disabled: bool,
    ) -> Result<(), Error> {
        // SAFETY: pte_idx is assumed to be within bounds by the caller (PageDirectory).
        let pte: PageTableEntry = unsafe {
            self.table
                .read(pte_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if pte.is_present() {
            let reason: &str = "page table entry is busy";
            error!("map_entry(): {reason} (pte_idx={})", pte_idx.into_raw());
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        let frame: FrameNumber =
            FrameNumber::from_raw_value(frame.into_raw_value() / ::arch::mem::PAGE_SIZE)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "frame number out of range"))?;
        let new_pte: PageTableEntry = PageTableEntry::new(
            PageTableEntryFlags::new(
                PresentFlag::Present,
                if writable {
                    ReadWriteFlag::ReadWrite
                } else {
                    ReadWriteFlag::ReadOnly
                },
                if supervisor {
                    UserSupervisorFlag::Supervisor
                } else {
                    UserSupervisorFlag::User
                },
                if writethrough {
                    PageWriteThroughFlag::WriteThrough
                } else {
                    PageWriteThroughFlag::NotWriteThrough
                },
                if cache_disabled {
                    PageCacheDisableFlag::CacheDisabled
                } else {
                    PageCacheDisableFlag::CacheEnabled
                },
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
            ),
            frame,
        );
        // SAFETY: pte_idx is within bounds.
        unsafe { self.table.write(pte_idx, new_pte) };
        self.nmapped += 1;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Bulk-fills page table entries for contiguous identity-mapped physical memory.
    ///
    /// Each entry maps physical frame at `base_address + i * PAGE_SIZE` as a supervisor,
    /// read-write, write-through, cache-disabled page.
    ///
    /// # Parameters
    ///
    /// - `start_index`: First entry index to fill.
    /// - `count`: Number of consecutive entries to fill.
    /// - `base_address`: Page-aligned physical address of the first frame.
    /// - `supervisor`: Whether the pages are supervisor-only.
    /// - `skip_pte_verification`: If `true`, skip the check that all target entries are not
    ///   present.
    ///
    /// # Returns
    ///
    /// Upon success, the number of frames mapped is returned. Upon failure, a tuple containing
    /// the number of frames that were successfully mapped and the error is returned.
    ///
    pub fn fill(
        &mut self,
        start_index: usize,
        count: usize,
        base_address: FrameAddress,
        supervisor: bool,
        skip_pte_verification: bool,
    ) -> Result<usize, (usize, Error)> {
        // Bounds check.
        let end: usize = start_index.checked_add(count).ok_or_else(|| {
            let reason: &str = "index overflow";
            error!("fill(): {}", reason);
            (0, Error::new(ErrorCode::InvalidArgument, reason))
        })?;
        if end > PAGE_TABLE_LENGTH {
            let reason: &str = "index out of bounds";
            error!(
                "fill(): {} (start_index={}, count={}, max={})",
                reason, start_index, count, PAGE_TABLE_LENGTH
            );
            return Err((0, Error::new(ErrorCode::InvalidArgument, reason)));
        }

        // Verify that all target entries are not present.
        if !skip_pte_verification {
            for i in start_index..end {
                // SAFETY: index is within bounds (checked above).
                let idx = TableIndex::try_from(i).map_err(|e| (0, Error::from(e)))?;
                let pte: PageTableEntry = unsafe {
                    self.table.read(idx).ok_or_else(|| {
                        (0, Error::new(ErrorCode::BadAddress, "invalid page table entry"))
                    })?
                };
                if pte.is_present() {
                    let reason: &str = "page table entry is busy";
                    error!("fill(): {}", reason);
                    return Err((0, Error::new(ErrorCode::ResourceBusy, reason)));
                }
            }
        }

        // Build and write each page table entry.
        let base_pa: usize = base_address.into_raw_value();
        for i in 0..count {
            let pa: usize = base_pa + i * ::arch::mem::PAGE_SIZE;
            let frame: FrameNumber = FrameNumber::from_raw_value(pa / ::arch::mem::PAGE_SIZE)
                .ok_or_else(|| {
                    let reason: &str = "frame number out of range";
                    error!("fill(): {}", reason);
                    (i, Error::new(ErrorCode::BadAddress, reason))
                })?;
            let new_pte: PageTableEntry = PageTableEntry::new(
                PageTableEntryFlags::new(
                    PresentFlag::Present,
                    ReadWriteFlag::ReadWrite,
                    if supervisor {
                        UserSupervisorFlag::Supervisor
                    } else {
                        UserSupervisorFlag::User
                    },
                    PageWriteThroughFlag::WriteThrough,
                    PageCacheDisableFlag::CacheDisabled,
                    AccessedFlag::NotAccessed,
                    DirtyFlag::NotDirty,
                ),
                frame,
            );
            // SAFETY: start_index + i is within bounds (checked above).
            let idx = TableIndex::try_from(start_index + i).map_err(|e| (i, Error::from(e)))?;
            unsafe { self.table.write(idx, new_pte) };
            self.nmapped += 1;
        }

        Ok(count)
    }
}
