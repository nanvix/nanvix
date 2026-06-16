// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::x86::mem::mmu::page_directory::PageDirectory,
        mem::{
            AccessPermission,
            Address,
            FrameAddress,
            PageAligned,
            PageDirectoryAddress,
            PdptAddress,
            Table,
            VirtualAddress,
        },
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
    PdptEntry,
    PdptEntryFlags,
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
/// A type that represents a Page Directory Pointer Table (PDPT) in the x86_64 4-level hierarchy.
///
/// Wraps a [`Table<PdptEntry>`] and provides typed operations. When a PD does
/// not yet exist for the target virtual address region, it is allocated via the caller-provided
/// page allocator and the backing [`PageTableStorage`] is returned to the caller for lifetime management.
///
pub struct Pdpt(Table<PdptEntry>);

/// Pages allocated during a [`Pdpt::map_page`] call.
pub struct PdptAlloc {
    /// Newly allocated PD storage (if PDPT entry was absent).
    pub pd_page: Option<PageTableStorage>,
    /// Allocations from the PD level.
    pub pd: super::page_directory::PdAlloc<PageTableStorage>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Pdpt {
    ///
    /// # Description
    ///
    /// Creates a [`Pdpt`] from a physical base address.
    ///
    /// # Safety
    ///
    /// `paddr` must be a valid, 4 KiB-aligned, identity-mapped physical address.
    ///
    pub unsafe fn from_address(paddr: PdptAddress) -> Self {
        Self(Table::from_address(paddr.into_raw_value()))
    }

    ///
    /// # Description
    ///
    /// Installs a pre-existing PD frame into the PDPT at `index`.
    ///
    /// # Safety
    ///
    /// `index` must be `< 512` and `frame` must point to a valid page.
    ///
    pub unsafe fn install_pd(&self, index: usize, frame: FrameAddress) {
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
            .write(idx, PdptEntry::new(PdptEntryFlags::from(flags), frame.into_frame_number()));
    }

    //==============================================================================================
    // Page-Level Operations (delegate to PageDirectory)
    //==============================================================================================

    /// Maps a user page by delegating through PD → PT.
    ///
    /// Returns `(Option<PageTableStorage>, Option<(PageTableAligned<VirtualAddress>, PageTable)>)`:
    /// - The first is newly allocated PD storage (if PDPT entry was absent).
    /// - The second is a newly allocated PT (if PDE was absent).
    /// The caller is responsible for keeping both alive.
    pub unsafe fn map_page<T: Fn() -> Result<PageTableStorage, Error>>(
        &self,
        vaddr: PageAligned<VirtualAddress>,
        frame: FrameAddress,
        access: AccessPermission,
        page_allocator: &T,
    ) -> Result<PdptAlloc, Error> {
        let (pd, allocated_storage): (PageDirectory, Option<PageTableStorage>) =
            self.ensure_pd(vaddr.into_raw_value(), page_allocator)?;
        let pd_alloc: super::page_directory::PdAlloc<PageTableStorage> =
            pd.map_page(vaddr, frame, access, page_allocator)?;
        Ok(PdptAlloc {
            pd_page: allocated_storage,
            pd: pd_alloc,
        })
    }

    /// Unmaps a user page by delegating through PD → PT.
    ///
    /// Returns `(Option<FrameAddress>, bool)` — the unmapped frame and whether the PT was freed.
    pub unsafe fn unmap_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(Option<FrameAddress>, bool), Error> {
        let pdpt_idx = ::arch::mem::paging::pdpt_index(vaddr.into_raw_value());
        let entry: PdptEntry = self
            .0
            .read(pdpt_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        if !entry.is_present() {
            return Ok((None, false));
        }
        let pd: PageDirectory = PageDirectory::from_address(PageDirectoryAddress::from_raw_value(
            entry.frame_address(),
        )?);
        pd.unmap_page(vaddr)
    }

    /// Looks up a user page via hardware walk through PD → PT.
    pub unsafe fn lookup_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<FrameAddress, Error> {
        let pdpt_idx = ::arch::mem::paging::pdpt_index(vaddr.into_raw_value());
        let entry: PdptEntry = self
            .0
            .read(pdpt_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        if !entry.is_present() {
            let reason: &str = "PDPT entry not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }
        let pd: PageDirectory = PageDirectory::from_address(PageDirectoryAddress::from_raw_value(
            entry.frame_address(),
        )?);
        pd.lookup_page(vaddr)
    }

    /// Tries to look up a user page via hardware. Returns `None` if not present.
    pub unsafe fn try_lookup_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<FrameAddress>, Error> {
        let pdpt_idx = ::arch::mem::paging::pdpt_index(vaddr.into_raw_value());
        let entry: PdptEntry = self
            .0
            .read(pdpt_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        if !entry.is_present() {
            return Ok(None);
        }
        let pd: PageDirectory = PageDirectory::from_address(PageDirectoryAddress::from_raw_value(
            entry.frame_address(),
        )?);
        pd.try_lookup_page(vaddr)
    }

    /// Changes access permissions on a user page via hardware walk through PD → PT.
    pub unsafe fn ctrl_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        let pdpt_idx = ::arch::mem::paging::pdpt_index(vaddr.into_raw_value());
        let entry: PdptEntry = self
            .0
            .read(pdpt_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        if !entry.is_present() {
            let reason: &str = "PDPT entry not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }
        let pd: PageDirectory = PageDirectory::from_address(PageDirectoryAddress::from_raw_value(
            entry.frame_address(),
        )?);
        pd.ctrl_page(vaddr, access)
    }

    /// Ensures the PD exists for `vaddr`, allocating one if the PDPT entry is absent.
    ///
    /// If the entry is already present, ensures the user bit is set. Returns the
    /// [`PageDirectory`] and optionally the allocated [`PageTableStorage`] if a new PD
    /// was created.
    pub(crate) unsafe fn ensure_pd<T: Fn() -> Result<PageTableStorage, Error>>(
        &self,
        vaddr: usize,
        page_allocator: &T,
    ) -> Result<(PageDirectory, Option<PageTableStorage>), Error> {
        let pdpt_idx = ::arch::mem::paging::pdpt_index(vaddr);
        let entry: PdptEntry = self
            .0
            .read(pdpt_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;

        let allocated: Option<PageTableStorage> = if !entry.is_present() {
            let storage: PageTableStorage = page_allocator()?;
            let pd_paddr: u64 = storage.as_ptr() as u64;

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
                pdpt_idx,
                PdptEntry::new(
                    PdptEntryFlags::from(flags),
                    FrameNumber::from_raw_value(pd_paddr as usize / ::arch::mem::PAGE_SIZE)
                        .ok_or_else(|| {
                            Error::new(ErrorCode::BadAddress, "PD frame number out of range")
                        })?,
                ),
            );

            Some(storage)
        } else {
            // Ensure user bit is set on existing entry.
            self.0.write(pdpt_idx, entry.ensure_user(true));
            None
        };

        let entry: PdptEntry = self
            .0
            .read(pdpt_idx)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
        Ok((
            PageDirectory::from_address(PageDirectoryAddress::from_raw_value(
                entry.frame_address(),
            )?),
            allocated,
        ))
    }
}
