// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::{
    AccessPermission,
    Address,
    FrameAddress,
    PageAligned,
    PageTableAddress,
    PhysicalAddress,
};
use ::alloc::boxed::Box;
use ::arch::mem::{
    self,
    paging::{
        AccessedFlag,
        DirtyFlag,
        FrameNumber,
        NoExecuteFlag,
        PageCacheDisableFlag,
        PageDirectoryEntry,
        PageDirectoryEntryFlags,
        PageWriteThroughFlag,
        PresentFlag,
        ReadWriteFlag,
        UserSupervisorFlag,
    },
};
use ::core::ops::{
    Deref,
    DerefMut,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

/// Storage for the page directory (PD level of 4-level paging).
/// Each entry is 8 bytes (u64) and there are 512 entries per table.
pub enum PageDirectoryStorage {
    Heap(Box<[u64; mem::PAGE_SIZE / core::mem::size_of::<u64>()]>),
}

///
/// # Description
///
/// A type that represents a page directory (third level of 4-level paging).
/// In x86_64, each PD entry points to a page table (PT).
///
/// TODO: Full 4-level paging support requires PML4 and PDPT management above this level.
///
pub struct PageDirectory {
    /// Entries.
    entries: PageDirectoryStorage,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl PageDirectoryStorage {
    pub fn new() -> Self {
        Self::Heap(Box::new([0; mem::PAGE_SIZE / core::mem::size_of::<u64>()]))
    }
}

impl Deref for PageDirectoryStorage {
    type Target = [u64];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Heap(entries) => entries.deref(),
        }
    }
}

impl DerefMut for PageDirectoryStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Heap(entries) => entries.deref_mut(),
        }
    }
}

impl PageDirectory {
    pub fn new(entries: PageDirectoryStorage) -> Self {
        let mut pgdir: PageDirectory = PageDirectory { entries };
        pgdir.clean();
        pgdir
    }

    pub fn map(
        &mut self,
        vaddr: PageTableAddress,
        paddr: FrameAddress,
        supervisor: bool,
        access: AccessPermission,
    ) -> Result<(), Error> {
        let pde: PageDirectoryEntry = match self.read_pde(vaddr) {
            Some(pde) => pde,
            None => {
                let reason: &str = "failed to read page directory entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if pde.is_present() {
            let reason: &str = "page directory entry is busy";
            error!("{reason}");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        let pde: PageDirectoryEntry = PageDirectoryEntry::new(
            PageDirectoryEntryFlags::new(
                PresentFlag::Present,
                if access.is_writable() {
                    ReadWriteFlag::ReadWrite
                } else {
                    ReadWriteFlag::ReadOnly
                },
                if supervisor {
                    UserSupervisorFlag::Supervisor
                } else {
                    UserSupervisorFlag::User
                },
                PageWriteThroughFlag::WriteThrough,
                PageCacheDisableFlag::CacheDisabled,
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
                NoExecuteFlag::Execute,
            ),
            paddr.into_frame_number(),
        );

        self.write_pde(vaddr, pde);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps a page table from the page directory.
    ///
    pub fn unmap(&mut self, pgtable_address: PageTableAddress) -> Result<FrameAddress, Error> {
        let pde: PageDirectoryEntry = match self.read_pde(pgtable_address) {
            Some(pde) => pde,
            None => {
                let reason: &str = "failed to read page directory entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if !pde.is_present() {
            let reason: &str = "page directory entry is not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        let paddr: FrameAddress = FrameAddress::from_frame_number(pde.frame())?;

        let pde: PageDirectoryEntry = PageDirectoryEntry::new(
            PageDirectoryEntryFlags::new(
                PresentFlag::NotPresent,
                ReadWriteFlag::ReadOnly,
                UserSupervisorFlag::User,
                PageWriteThroughFlag::WriteThrough,
                PageCacheDisableFlag::CacheDisabled,
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
                NoExecuteFlag::Execute,
            ),
            FrameNumber::NULL,
        );

        self.write_pde(pgtable_address, pde);

        Ok(paddr)
    }

    pub fn clean(&mut self) {
        for pde in self.entries.iter_mut() {
            *pde = 0;
        }
    }

    pub fn read_pde(&self, vaddr: PageTableAddress) -> Option<PageDirectoryEntry> {
        let pde_idx: usize = vaddr.get_pde_index();
        PageDirectoryEntry::from_raw_value(self.entries[pde_idx])
    }

    fn write_pde(&mut self, vaddr: PageTableAddress, pde: PageDirectoryEntry) {
        let pde_idx: usize = vaddr.get_pde_index();
        self.entries[pde_idx] = pde.into_raw_value();
    }

    pub fn physical_address(&self) -> Result<FrameAddress, Error> {
        Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(
            self.entries.as_ptr() as usize,
        )?)?))
    }
}
