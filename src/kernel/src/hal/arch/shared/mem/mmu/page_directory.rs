// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::page_table::PageTable;
use crate::hal::mem::{
    AccessPermission,
    Address,
    FrameAddress,
    PageAligned,
    PageDirectoryAddress,
    PageTableAddress,
    PageTableAligned,
    PhysicalAddress,
    Table,
    VirtualAddress,
};
use ::arch::mem::{
    paging::{
        AccessedFlag,
        CopyOnWriteFlag,
        DirtyFlag,
        FrameNumber,
        PageCacheDisableFlag,
        PageDirectoryEntry,
        PageDirectoryEntryFlags,
        PageSizeFlag,
        PageTableEntry,
        PageTableEntryFlags,
        PageWriteThroughFlag,
        PresentFlag,
        ReadWriteFlag,
        TableIndex,
        UserSupervisorFlag,
    },
    PAGE_TABLE_LENGTH,
    PGTAB_ALIGNMENT,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a page directory.
///
/// Wraps a [`Table<PageDirectoryEntry>`] for typed entry access. Does not own the backing
/// memory — the caller is responsible for keeping the underlying page alive.
///
/// Used directly on both x86 (2-level paging, root page directory) and x86_64 (the PD level
/// of the 4-level PML4 → PDPT → PD → PT hierarchy).
///
pub struct PageDirectory {
    /// Typed table view over the backing storage.
    table: Table<PageDirectoryEntry>,
}

/// Pages allocated during a [`PageDirectory::map_page`] call.
pub struct PdAlloc<S: ::core::ops::DerefMut<Target = [::arch::mem::PteWord]>> {
    /// Newly allocated PT storage (if PDE was absent).
    pub pt: Option<(PageTableAligned<VirtualAddress>, S)>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl PageDirectory {
    ///
    /// # Description
    ///
    /// Creates a [`PageDirectory`] from a base address without initializing entries.
    ///
    /// # Safety
    ///
    /// `base` must be a valid, page-aligned, identity-mapped address backed by at least one
    /// page of writable memory that outlives this [`PageDirectory`].
    ///
    pub unsafe fn from_address(base: PageDirectoryAddress) -> Self {
        Self {
            table: Table::from_address(base.into_raw_value()),
        }
    }

    //==============================================================================================
    // Page-Level Operations (Hardware Lookup)
    //==============================================================================================
    //
    // These methods walk the PD→PT hierarchy using hardware table reads/writes. No linked list
    // search is needed for read-only operations — the PDE gives the PT base address directly.
    //
    // For `map_page`, the caller provides a page allocator; any newly allocated PT is returned
    // so the caller can track ownership. For `unmap_page`, the caller is told whether the PT
    // became empty (and thus was cleared from the PDE) so it can drop the backing page.
    //

    ///
    /// # Description
    ///
    /// Looks up the frame address for a user page through hardware page table walk.
    ///
    pub fn lookup_page(&self, vaddr: PageAligned<VirtualAddress>) -> Result<FrameAddress, Error> {
        let pde_idx = Self::pde_index_for(vaddr.into_raw_value());
        // SAFETY: pde_idx is within bounds.
        let pde: PageDirectoryEntry = unsafe {
            self.table
                .read(pde_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pde.is_present() {
            let reason: &str = "page directory entry not present";
            error!("{reason} (vaddr={vaddr:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        let pte_idx = Self::pte_index_for(vaddr.into_raw_value());
        // SAFETY: PDE is present.
        let pt: Table<PageTableEntry> = unsafe { Self::pt_table_for(&pde) };
        let pte: PageTableEntry = unsafe {
            pt.read(pte_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pte.is_present() {
            let reason: &str = "page not present";
            error!("{reason} (vaddr={vaddr:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        FrameAddress::from_raw_value(pte.frame_address())
    }

    ///
    /// # Description
    ///
    /// Tries to look up a user page through hardware. Returns `None` if not present.
    ///
    pub fn try_lookup_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<FrameAddress>, Error> {
        let pde_idx = Self::pde_index_for(vaddr.into_raw_value());
        // SAFETY: pde_idx is within bounds.
        let pde: PageDirectoryEntry = unsafe {
            self.table
                .read(pde_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pde.is_present() {
            return Ok(None);
        }

        let pte_idx = Self::pte_index_for(vaddr.into_raw_value());
        // SAFETY: PDE is present.
        let pt: Table<PageTableEntry> = unsafe { Self::pt_table_for(&pde) };
        let pte: PageTableEntry = unsafe {
            pt.read(pte_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pte.is_present() {
            return Ok(None);
        }

        // Construct frame address directly from raw PTE value to avoid MEMORY_SIZE range
        // checks — MMIO frames may legitimately lie above physical memory.
        let raw_frame: usize = pte.frame_address();
        let phys_addr: PhysicalAddress = unsafe {
            PhysicalAddress::from_mmio_address(VirtualAddress::from_raw_value(raw_frame))?
        };
        Ok(Some(FrameAddress::new(PageAligned::from_address(phys_addr)?)))
    }

    ///
    /// # Description
    ///
    /// Changes access permissions on a user page through hardware page table walk.
    ///
    pub fn ctrl_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        let pde_idx = Self::pde_index_for(vaddr.into_raw_value());
        // SAFETY: pde_idx is within bounds.
        let pde: PageDirectoryEntry = unsafe {
            self.table
                .read(pde_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pde.is_present() {
            let reason: &str = "page table not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        let pte_idx = Self::pte_index_for(vaddr.into_raw_value());
        // SAFETY: PDE is present.
        let pt: Table<PageTableEntry> = unsafe { Self::pt_table_for(&pde) };
        let pte: PageTableEntry = unsafe {
            pt.read(pte_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pte.is_present() {
            let reason: &str = "page not present";
            error!("{reason} (vaddr={vaddr:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        let mut new_pte: PageTableEntry = pte;
        new_pte.set_read_write(if access.is_writable() {
            ReadWriteFlag::ReadWrite
        } else {
            ReadWriteFlag::ReadOnly
        });
        new_pte.set_user_supervisor(UserSupervisorFlag::User);
        unsafe { pt.write(pte_idx, new_pte) };
        unsafe { ::arch::mem::paging::invlpg(vaddr.into_raw_value()) };

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Maps a user page through hardware. Allocates a page table if the PDE slot is empty.
    ///
    /// # Returns
    ///
    /// On success, returns `Some((addr, pt))` if a new page table was allocated, or `None`
    /// if an existing one was reused. The caller is responsible for keeping the returned
    /// page table alive for ownership tracking.
    ///
    pub fn map_page<S, T>(
        &self,
        vaddr: PageAligned<VirtualAddress>,
        frame: FrameAddress,
        access: AccessPermission,
        page_table_allocator: &T,
    ) -> Result<PdAlloc<S>, Error>
    where
        S: ::core::ops::DerefMut<Target = [::arch::mem::PteWord]>,
        T: Fn() -> Result<S, Error>,
    {
        let pgtable_vaddr: PageTableAligned<VirtualAddress> = Self::pt_address_for(vaddr)?;

        // Let ensure_pt handle the PDE check and allocate a PT only when needed.
        let mut allocated_storage: Option<S> = None;
        let mut alloc = || -> Result<PageTableAddress, Error> {
            let storage: S = page_table_allocator()?;
            let addr: PageTableAddress =
                PageTableAddress::from_raw_value(storage.as_ptr() as usize)?;
            allocated_storage = Some(storage);
            Ok(addr)
        };

        // SAFETY: PD is valid; allocator returns valid page-aligned addresses.
        let pt_addr: PageTableAddress =
            unsafe { self.ensure_pt(vaddr.into_raw_value(), true, &mut alloc)? };

        // Write PTE.
        let mut pt: PageTable = unsafe { PageTable::from_address(pt_addr) };
        let pte_idx = Self::pte_index_for(vaddr.into_raw_value());
        pt.map_entry(pte_idx, frame, false, access.is_writable(), false, true)?;
        unsafe { ::arch::mem::paging::invlpg(vaddr.into_raw_value()) };

        Ok(PdAlloc {
            pt: allocated_storage.map(|s| (pgtable_vaddr, s)),
        })
    }

    ///
    /// # Description
    ///
    /// Unmaps a user page through hardware. If the page table becomes empty, clears the PDE.
    ///
    /// # Returns
    ///
    /// On success, returns `(Some(frame), pt_freed)` where `frame` is the unmapped frame
    /// address and `pt_freed` indicates whether the page table became empty and the PDE was
    /// cleared. Returns `(None, false)` if the page was not present.
    ///
    pub fn unmap_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(Option<FrameAddress>, bool), Error> {
        let pde_idx = Self::pde_index_for(vaddr.into_raw_value());

        // SAFETY: pde_idx is within bounds.
        let pde: PageDirectoryEntry = unsafe {
            self.table
                .read(pde_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pde.is_present() {
            return Ok((None, false));
        }

        // Read PTE.
        let pt: Table<PageTableEntry> = unsafe { Self::pt_table_for(&pde) };
        let pte_idx = Self::pte_index_for(vaddr.into_raw_value());
        let pte: PageTableEntry = unsafe {
            pt.read(pte_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pte.is_present() {
            return Ok((None, false));
        }

        // Clear PTE.
        let frame_address: FrameAddress = FrameAddress::from_raw_value(pte.frame_address())?;
        unsafe {
            pt.write(
                pte_idx,
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
        unsafe { ::arch::mem::paging::invlpg(vaddr.into_raw_value()) };

        // Check if the PT is now empty and clear PDE if so.
        let pt_freed: bool = unsafe { self.is_pt_empty(pde_idx)? };
        if pt_freed {
            // SAFETY: pde_idx is within bounds.
            unsafe {
                self.table.write(
                    pde_idx,
                    PageDirectoryEntry::new(
                        PageDirectoryEntryFlags::new(
                            PresentFlag::NotPresent,
                            ReadWriteFlag::ReadOnly,
                            UserSupervisorFlag::Supervisor,
                            PageWriteThroughFlag::NotWriteThrough,
                            PageCacheDisableFlag::CacheDisabled,
                            AccessedFlag::NotAccessed,
                            DirtyFlag::NotDirty,
                            PageSizeFlag::Standard,
                        ),
                        FrameNumber::NULL,
                    ),
                )
            };
        }

        Ok((Some(frame_address), pt_freed))
    }

    /// Ensures a page table exists for the PDE at the given virtual address.
    ///
    /// If the PDE is absent, allocates a fresh PT via `page_allocator`. If the PDE maps a
    /// 2 MiB large page, splits it into 512 × 4 KiB entries. If the PDE already points to a
    /// normal PT, returns it (promoting to user if requested).
    ///
    /// # Safety
    ///
    /// The page directory must be valid. `page_allocator` must return a zeroed, identity-mapped,
    /// page-aligned address that will remain valid for the required lifetime.
    pub unsafe fn ensure_pt<F: FnMut() -> Result<PageTableAddress, Error>>(
        &self,
        vaddr: usize,
        user: bool,
        page_allocator: &mut F,
    ) -> Result<PageTableAddress, Error> {
        let pde_idx = Self::pde_index_for(vaddr);
        let entry: PageDirectoryEntry = self
            .table
            .read(pde_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;

        if entry.is_present() && entry.is_large_page() {
            // Split 2 MiB → 512 × 4 KiB.
            let base_2m: u64 = entry.large_page_address();
            let flags_4k: PageTableEntryFlags = entry.flags_without_ps();

            let pt_addr: PageTableAddress = page_allocator()?;
            let pt: Table<PageTableEntry> = Table::from_address(pt_addr.into_raw_value());
            for i in 0..PAGE_TABLE_LENGTH {
                let frame: FrameNumber = FrameNumber::from_raw_value(
                    (base_2m as usize + i * ::arch::mem::PAGE_SIZE) / ::arch::mem::PAGE_SIZE,
                )
                .ok_or_else(|| {
                    Error::new(ErrorCode::BadAddress, "frame number out of range in split")
                })?;
                pt.write(TableIndex::try_from(i)?, PageTableEntry::new(flags_4k, frame));
            }

            let mut new_pd_flags: PageTableEntryFlags = flags_4k;
            if user {
                new_pd_flags.set_user_supervisor(UserSupervisorFlag::User);
            }
            self.table.write(
                pde_idx,
                PageDirectoryEntry::new(
                    PageDirectoryEntryFlags::from(new_pd_flags),
                    FrameNumber::from_raw_value(pt_addr.into_raw_value() / ::arch::mem::PAGE_SIZE)
                        .ok_or_else(|| {
                            Error::new(ErrorCode::BadAddress, "PT frame number out of range")
                        })?,
                ),
            );

            Ok(pt_addr)
        } else if entry.is_present() {
            if user {
                self.table.write(pde_idx, entry.ensure_user(true));
            }
            PageTableAddress::from_raw_value(entry.frame_address())
        } else {
            let pt_addr: PageTableAddress = page_allocator()?;
            let flags: PageTableEntryFlags = PageTableEntryFlags::new(
                PresentFlag::Present,
                ReadWriteFlag::ReadWrite,
                if user {
                    UserSupervisorFlag::User
                } else {
                    UserSupervisorFlag::Supervisor
                },
                PageWriteThroughFlag::NotWriteThrough,
                PageCacheDisableFlag::CacheEnabled,
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
            );
            self.table.write(
                pde_idx,
                PageDirectoryEntry::new(
                    PageDirectoryEntryFlags::from(flags),
                    FrameNumber::from_raw_value(pt_addr.into_raw_value() / ::arch::mem::PAGE_SIZE)
                        .ok_or_else(|| {
                            Error::new(ErrorCode::BadAddress, "PT frame number out of range")
                        })?,
                ),
            );
            Ok(pt_addr)
        }
    }

    /// Computes the PDE index from a virtual address.
    fn pde_index_for(vaddr: usize) -> TableIndex {
        ::arch::mem::paging::pd_index(vaddr)
    }

    /// Computes the PTE index from a virtual address.
    fn pte_index_for(vaddr: usize) -> TableIndex {
        ::arch::mem::paging::pt_index(vaddr)
    }

    /// Computes the page-table-aligned region address containing `vaddr`.
    fn pt_address_for(
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<PageTableAligned<VirtualAddress>, Error> {
        let aligned: usize = ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT);
        PageTableAligned::from_raw_value(aligned)
    }

    /// Constructs a hardware page table view from a present PDE.
    ///
    /// # Safety
    ///
    /// The PDE must be present and point to a valid, identity-mapped page table page.
    unsafe fn pt_table_for(pde: &PageDirectoryEntry) -> Table<PageTableEntry> {
        Table::from_address(pde.frame_address())
    }

    /// Checks whether the page table referenced by the PDE at `pde_idx` has any present entries.
    ///
    /// # Safety
    ///
    /// `pde_idx` must be within bounds and the PDE must be present.
    unsafe fn is_pt_empty(&self, pde_idx: TableIndex) -> Result<bool, Error> {
        let pde: PageDirectoryEntry = self
            .table
            .read(pde_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        if !pde.is_present() {
            return Ok(true);
        }
        let pt: Table<PageTableEntry> = Self::pt_table_for(&pde);
        for i in 0..PAGE_TABLE_LENGTH {
            if pt
                .read(TableIndex::try_from(i)?)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
                .is_present()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

//==================================================================================================
// Copy-on-Write Operations
//==================================================================================================

impl PageDirectory {
    /// Reads the page table entry for `vaddr`.
    ///
    /// Returns `Ok(None)` if the page table is absent or the page is not present.
    pub fn try_read_pte(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<PageTableEntry>, Error> {
        let pde_idx = Self::pde_index_for(vaddr.into_raw_value());
        // SAFETY: pde_idx is within bounds.
        let pde: PageDirectoryEntry = unsafe {
            self.table
                .read(pde_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pde.is_present() {
            return Ok(None);
        }
        let pte_idx = Self::pte_index_for(vaddr.into_raw_value());
        // SAFETY: PDE is present.
        let pt: Table<PageTableEntry> = unsafe { Self::pt_table_for(&pde) };
        let pte: PageTableEntry = unsafe {
            pt.read(pte_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pte.is_present() {
            return Ok(None);
        }
        Ok(Some(pte))
    }

    /// Walks to the present PTE for `vaddr`, applies `f`, writes it back and flushes the TLB.
    fn modify_pte<F>(
        &self,
        vaddr: PageAligned<VirtualAddress>,
        f: F,
    ) -> Result<PageTableEntry, Error>
    where
        F: FnOnce(PageTableEntry) -> Result<PageTableEntry, Error>,
    {
        let pde_idx = Self::pde_index_for(vaddr.into_raw_value());
        // SAFETY: pde_idx is within bounds.
        let pde: PageDirectoryEntry = unsafe {
            self.table
                .read(pde_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        if !pde.is_present() {
            let reason: &str = "page table not present";
            error!("{reason} (vaddr={vaddr:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }
        let pte_idx = Self::pte_index_for(vaddr.into_raw_value());
        // SAFETY: PDE is present.
        let pt: Table<PageTableEntry> = unsafe { Self::pt_table_for(&pde) };
        let pte: PageTableEntry = unsafe {
            pt.read(pte_idx)
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?
        };
        let old: PageTableEntry = pte;
        let new_pte: PageTableEntry = f(pte)?;
        // SAFETY: pte_idx is within bounds.
        unsafe { pt.write(pte_idx, new_pte) };
        // SAFETY: called from kernel mode after modifying a PTE.
        unsafe { ::arch::mem::paging::invlpg(vaddr.into_raw_value()) };
        Ok(old)
    }

    /// Marks the user page at `vaddr` copy-on-write: clears writable and sets the CoW bit.
    pub fn mark_cow_page(&self, vaddr: PageAligned<VirtualAddress>) -> Result<(), Error> {
        self.modify_pte(vaddr, |mut pte| {
            if !pte.is_present() {
                let reason: &str = "page is not present";
                error!("mark_cow_page(): {reason} (vaddr={vaddr:?})");
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            }
            if !pte.flags().is_writable() {
                let reason: &str = "page is already read-only";
                error!("mark_cow_page(): {reason} (vaddr={vaddr:?})");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
            pte.set_read_write(ReadWriteFlag::ReadOnly);
            pte.set_cow(CopyOnWriteFlag::CopyOnWrite);
            Ok(pte)
        })?;
        Ok(())
    }

    /// Clears the copy-on-write mark on the user page at `vaddr` and restores writability.
    pub fn unmark_cow_page(&self, vaddr: PageAligned<VirtualAddress>) -> Result<(), Error> {
        self.modify_pte(vaddr, |mut pte| {
            if !pte.is_present() {
                let reason: &str = "page is not present";
                error!("unmark_cow_page(): {reason} (vaddr={vaddr:?})");
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            }
            if !pte.is_cow() {
                let reason: &str = "page is not copy-on-write";
                error!("unmark_cow_page(): {reason} (vaddr={vaddr:?})");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
            pte.set_cow(CopyOnWriteFlag::NotCopyOnWrite);
            pte.set_read_write(ReadWriteFlag::ReadWrite);
            Ok(pte)
        })?;
        Ok(())
    }

    /// Repoints the copy-on-write page at `vaddr` to `new_frame`, restoring writability and
    /// clearing the CoW bit. Returns the previously mapped frame.
    pub fn replace_cow_frame_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
        new_frame: FrameAddress,
    ) -> Result<FrameAddress, Error> {
        let old: PageTableEntry = self.modify_pte(vaddr, |pte| {
            if !pte.is_present() {
                let reason: &str = "page is not present";
                error!("replace_cow_frame_page(): {reason} (vaddr={vaddr:?})");
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            }
            if !pte.is_cow() {
                let reason: &str = "page is not copy-on-write";
                error!("replace_cow_frame_page(): {reason} (vaddr={vaddr:?})");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
            if pte.flags().is_writable() {
                let reason: &str = "copy-on-write page is unexpectedly writable";
                error!("replace_cow_frame_page(): {reason} (vaddr={vaddr:?})");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
            let mut new_flags: PageTableEntryFlags = pte.flags();
            new_flags.set_read_write(ReadWriteFlag::ReadWrite);
            new_flags.set_cow(CopyOnWriteFlag::NotCopyOnWrite);
            Ok(PageTableEntry::new(new_flags, new_frame.into_frame_number()))
        })?;
        FrameAddress::from_frame_number(old.frame_number())
    }
}
