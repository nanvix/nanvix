// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::pdpt::Pdpt;
use crate::{
    hal::mem::{
        AccessPermission,
        Address,
        FrameAddress,
        PageAligned,
        PdptAddress,
        Pml4Address,
        Table,
        VirtualAddress,
    },
    mm::PageTableStorage,
};
use ::arch::mem::paging::{
    AccessedFlag,
    DirtyFlag,
    FrameNumber,
    PageCacheDisableFlag,
    PageTableEntryFlags,
    PageWriteThroughFlag,
    Pml4Entry,
    Pml4EntryFlags,
    PresentFlag,
    ReadWriteFlag,
    TableIndex,
    UserSupervisorFlag,
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
/// A type that represents a Page Map Level 4 (PML4) in the x86_64 4-level hierarchy.
///
/// Wraps a [`Table<Pml4Entry>`] and provides typed `map`/`unmap` operations that delegate
/// down through [`Pdpt`] → [`Pd`](super::page_directory::Pd), mirroring the x86 [`PageDirectory`]
/// pattern at each level of the hierarchy.
///
pub struct Pml4(Table<Pml4Entry>);

/// Pages allocated during a [`Pml4::map_page`] call.
pub struct Pml4Alloc {
    /// Newly allocated PDPT page (if PML4 entry was absent).
    pub pdpt_page: Option<PageTableStorage>,
    /// Allocations from the PDPT level.
    pub pdpt: super::pdpt::PdptAlloc,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Pml4 {
    ///
    /// # Description
    ///
    /// Creates a [`Pml4`] from a physical base address.
    ///
    /// # Safety
    ///
    /// `paddr` must be a valid, 4 KiB-aligned, identity-mapped physical address.
    ///
    pub unsafe fn from_address(paddr: Pml4Address) -> Self {
        Self(Table::from_address(paddr.into_raw_value()))
    }

    ///
    /// # Description
    ///
    /// Installs a pre-existing PDPT frame into the PML4 at `index`.
    ///
    /// # Safety
    ///
    /// `index` must be `< 512` and `frame` must point to a valid, zeroed PDPT page.
    ///
    pub unsafe fn install_pdpt(&self, index: usize, frame: FrameAddress) {
        let idx = TableIndex::try_from(index).expect("index within bounds");
        let flags: PageTableEntryFlags = PageTableEntryFlags::new(
            PresentFlag::Present,
            ReadWriteFlag::ReadWrite,
            UserSupervisorFlag::User,
            PageWriteThroughFlag::NotWriteThrough,
            PageCacheDisableFlag::CacheEnabled,
            AccessedFlag::NotAccessed,
            DirtyFlag::NotDirty,
        );
        self.0
            .write(idx, Pml4Entry::new(Pml4EntryFlags::from(flags), frame.into_frame_number()));
    }

    //==============================================================================================
    // Page-Level Operations (delegate to Pdpt → PageDirectory → PageTable)
    //==============================================================================================

    /// Maps a user page by delegating through PDPT → PD → PT.
    ///
    /// Allocates intermediate page tables on demand. All allocated backing pages are returned
    /// in [`MapPageAlloc`] — the caller keeps them alive for ownership management.
    pub unsafe fn map_page<T: Fn() -> Result<PageTableStorage, Error>>(
        &self,
        vaddr: PageAligned<VirtualAddress>,
        frame: FrameAddress,
        access: AccessPermission,
        page_allocator: &T,
    ) -> Result<Pml4Alloc, Error> {
        let (pdpt, allocated_pdpt_page): (Pdpt, Option<PageTableStorage>) =
            self.ensure_pdpt(vaddr.into_raw_value(), page_allocator)?;
        let pdpt_alloc: super::pdpt::PdptAlloc =
            pdpt.map_page(vaddr, frame, access, page_allocator)?;
        Ok(Pml4Alloc {
            pdpt_page: allocated_pdpt_page,
            pdpt: pdpt_alloc,
        })
    }

    /// Unmaps a user page by delegating through PDPT → PD → PT.
    ///
    /// Returns `(Option<FrameAddress>, bool)` — the unmapped frame and whether the PT was freed.
    pub unsafe fn unmap_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(Option<FrameAddress>, bool), Error> {
        let pml4_idx = ::arch::mem::paging::pml4_index(vaddr.into_raw_value());
        let entry: Pml4Entry = self
            .0
            .read(pml4_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        if !entry.is_present() {
            return Ok((None, false));
        }
        let pdpt: Pdpt = Pdpt::from_address(PdptAddress::from_raw_value(entry.frame_address())?);
        pdpt.unmap_page(vaddr)
    }

    /// Looks up a user page via hardware walk through PDPT → PD → PT.
    pub unsafe fn lookup_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<FrameAddress, Error> {
        let pml4_idx = ::arch::mem::paging::pml4_index(vaddr.into_raw_value());
        let entry: Pml4Entry = self
            .0
            .read(pml4_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        if !entry.is_present() {
            let reason: &str = "PML4 entry not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }
        let pdpt: Pdpt = Pdpt::from_address(PdptAddress::from_raw_value(entry.frame_address())?);
        pdpt.lookup_page(vaddr)
    }

    /// Tries to look up a user page via hardware. Returns `None` if not present.
    pub unsafe fn try_lookup_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<FrameAddress>, Error> {
        let pml4_idx = ::arch::mem::paging::pml4_index(vaddr.into_raw_value());
        let entry: Pml4Entry = self
            .0
            .read(pml4_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        if !entry.is_present() {
            return Ok(None);
        }
        let pdpt: Pdpt = Pdpt::from_address(PdptAddress::from_raw_value(entry.frame_address())?);
        pdpt.try_lookup_page(vaddr)
    }

    /// Changes access permissions on a user page via hardware walk through PDPT → PD → PT.
    pub unsafe fn ctrl_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        let pml4_idx = ::arch::mem::paging::pml4_index(vaddr.into_raw_value());
        let entry: Pml4Entry = self
            .0
            .read(pml4_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        if !entry.is_present() {
            let reason: &str = "PML4 entry not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }
        let pdpt: Pdpt = Pdpt::from_address(PdptAddress::from_raw_value(entry.frame_address())?);
        pdpt.ctrl_page(vaddr, access)
    }

    //==============================================================================================
    // Internal Helpers
    //==============================================================================================

    /// Ensures the PDPT exists for `vaddr`, allocating one if the PML4 entry is absent.
    ///
    /// Returns the [`Pdpt`] and an optional [`PageTableStorage`] if a new PDPT was allocated.
    pub(crate) unsafe fn ensure_pdpt<T: Fn() -> Result<PageTableStorage, Error>>(
        &self,
        vaddr: usize,
        page_allocator: &T,
    ) -> Result<(Pdpt, Option<PageTableStorage>), Error> {
        let pml4_idx = ::arch::mem::paging::pml4_index(vaddr);
        let entry: Pml4Entry = self
            .0
            .read(pml4_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;

        let allocated_page: Option<PageTableStorage> = if !entry.is_present() {
            let storage: PageTableStorage = page_allocator()?;
            let pdpt_paddr: u64 = storage.as_ptr() as u64;

            let flags: PageTableEntryFlags = PageTableEntryFlags::new(
                PresentFlag::Present,
                ReadWriteFlag::ReadWrite,
                UserSupervisorFlag::User,
                PageWriteThroughFlag::NotWriteThrough,
                PageCacheDisableFlag::CacheEnabled,
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
            );
            self.0.write(
                pml4_idx,
                Pml4Entry::new(
                    Pml4EntryFlags::from(flags),
                    FrameNumber::from_raw_value(pdpt_paddr as usize / ::arch::mem::PAGE_SIZE)
                        .ok_or_else(|| {
                            Error::new(ErrorCode::BadAddress, "PDPT frame number out of range")
                        })?,
                ),
            );

            Some(storage)
        } else {
            self.0.write(pml4_idx, entry.ensure_user(true));
            None
        };

        let entry: Pml4Entry = self
            .0
            .read(pml4_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        Ok((
            Pdpt::from_address(PdptAddress::from_raw_value(entry.frame_address())?),
            allocated_page,
        ))
    }
}
