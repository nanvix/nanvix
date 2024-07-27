// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use alloc::boxed::Box;

use crate::{
    arch::mem::{
        self,
        paging::{
            AccessedFlag,
            DirtyFlag,
            PageCacheDisableFlag,
            PageDirectoryEntry,
            PageDirectoryEntryFlags,
            PageWriteThroughFlag,
            PresentFlag,
            ReadWriteFlag,
            UserSupervisorFlag,
        },
    },
    error::{
        Error,
        ErrorCode,
    },
    hal::mem::{
        AccessPermission,
        Address,
        FrameAddress,
        PageAligned,
        PageTableAddress,
        PhysicalAddress,
    },
};
use core::ops::{
    Deref,
    DerefMut,
};

//==================================================================================================
// Structures
//==================================================================================================

pub enum PageDirectoryStorage {
    Heap(Box<[u32; mem::PAGE_SIZE / core::mem::size_of::<u32>()]>),
}

///
/// # Description
///
/// A type that represents a page directory.
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
        Self::Heap(Box::new([0; mem::PAGE_SIZE / core::mem::size_of::<u32>()]))
    }
}

impl Deref for PageDirectoryStorage {
    type Target = [u32];

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
        trace!("map(): vaddr={:?}, paddr={:?}", vaddr, paddr);

        // Obtain a cached copy of the page directory entry.
        let pde: PageDirectoryEntry = match self.read_pde(vaddr) {
            Some(pde) => pde,
            None => {
                let reason: &str = "failed to read page directory entry";
                error!("map(): {}", reason);
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page directory entry is busy.
        if pde.is_present() {
            let reason: &str = "page directory entry is busy";
            error!("map(): {}", reason);
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        // Construct page directory entry
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
            ),
            paddr.into_frame_number(),
        );

        // Write page directory entry
        self.write_pde(vaddr, pde);

        Ok(())
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
