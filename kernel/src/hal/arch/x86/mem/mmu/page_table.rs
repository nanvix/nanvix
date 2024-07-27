// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem::{
        self,
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
            UserSupervisorFlag,
        },
    },
    hal::mem::{
        AccessPermission,
        Address,
        FrameAddress,
        PageAddress,
        PageAligned,
        PhysicalAddress,
    },
};
use ::alloc::boxed::Box;
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

pub enum PageTableStorage {
    Heap(Box<[u32; mem::PAGE_SIZE / core::mem::size_of::<u32>()]>),
}

///
/// # Description
///
/// A type that represents a page table.
///
pub struct PageTable {
    /// Entries.
    entries: PageTableStorage,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl PageTableStorage {
    pub fn new() -> Self {
        Self::Heap(Box::new([0; mem::PAGE_SIZE / core::mem::size_of::<u32>()]))
    }
}

impl Deref for PageTableStorage {
    type Target = [u32];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Heap(entries) => entries.deref(),
        }
    }
}

impl DerefMut for PageTableStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Heap(entries) => entries.deref_mut(),
        }
    }
}

impl PageTable {
    pub fn new(entries: PageTableStorage) -> Self {
        let mut page_table: PageTable = Self { entries };
        page_table.clean();
        page_table
    }

    /// Maps a physical address into a virtual address in the target page table.
    pub fn map(
        &mut self,
        vaddr: PageAddress,
        paddr: FrameAddress,
        supervisor: bool,
        writethrough: bool,
        cache: bool,
        access: AccessPermission,
    ) -> Result<(), Error> {
        // Obtain a cached copy of the page table entry.
        let pte: PageTableEntry = match self.read_pte(vaddr) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!("map(): {}", reason);
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page table entry is busy.
        if pte.is_present() {
            let reason: &str = "page table entry is busy";
            error!("map(): {}", reason);
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        // Construct page table entry.
        let pte: PageTableEntry = PageTableEntry::new(
            PageTableEntryFlags::new(
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
                if writethrough {
                    PageWriteThroughFlag::WriteThrough
                } else {
                    PageWriteThroughFlag::NotWriteThrough
                },
                if cache {
                    PageCacheDisableFlag::CacheEnabled
                } else {
                    PageCacheDisableFlag::CacheDisabled
                },
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
            ),
            paddr.into_frame_number(),
        );

        // Write page table entry.
        self.write_pte(vaddr, pte);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps a page address from the target page table.
    ///
    /// # Parameters
    ///
    /// - `page_address`: Page address to unmap.
    ///
    /// # Return Values
    ///
    ///Upon success, the frame address that was associated with the given page address is returned
    ///and the concerned page is unmapped from the target page table. Upon failure, an error is
    ///returned instead.
    ///
    pub fn unmap(&mut self, page_address: PageAddress) -> Result<FrameAddress, Error> {
        // Obtain a cached copy of the page table entry.
        let pte: PageTableEntry = match self.read_pte(page_address) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!("unmap(): {}", reason);
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page is not present.
        if !pte.is_present() {
            let reason: &str = "page is not present";
            error!("unmap(): {}", reason);
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        // Retrieve frame address.
        let paddr: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;

        // Construct page table entry.
        let pte: PageTableEntry = PageTableEntry::new(
            PageTableEntryFlags::new(
                PresentFlag::NotPresent,
                ReadWriteFlag::ReadOnly,
                UserSupervisorFlag::User,
                PageWriteThroughFlag::NotWriteThrough,
                PageCacheDisableFlag::CacheDisabled,
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
            ),
            FrameNumber::NULL,
        );

        // Write page table entry.
        self.write_pte(page_address, pte);

        Ok(paddr)
    }

    ///
    /// # Description
    ///
    /// Looks up a page address in the target page table.
    ///
    /// # Parameters
    ///
    /// - `page_address`: Page address to lookup.
    ///
    /// # Return Values
    ///
    /// Upon success, the frame address associated with the target page is returned. Upon failure,
    /// an error is returned instead.
    ///
    pub fn lookup(&self, page_address: PageAddress) -> Result<FrameAddress, Error> {
        // Obtain a cached copy of the page table entry.
        let pte: PageTableEntry = match self.read_pte(page_address) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!("lookup(): {}", reason);
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page is not present.
        if !pte.is_present() {
            let reason: &str = "page is not present";
            error!("lookup(): {}", reason);
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        // Retrieve frame address.
        let paddr: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;

        Ok(paddr)
    }

    /// Changes access permissions on a page.
    pub fn ctrl(
        &mut self,
        supervisor: bool,
        page_address: PageAddress,
        access: AccessPermission,
    ) -> Result<(), Error> {
        // Obtain a cached copy of the page table entry.
        let mut pte: PageTableEntry = match self.read_pte(page_address) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!("change_access_permissions(): {}", reason);
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page is not present.
        if !pte.is_present() {
            let reason: &str = "page is not present";
            error!("change_access_permissions(): {}", reason);
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        // Modify page table entry.
        if access.is_writable() {
            pte.set_read_write(ReadWriteFlag::ReadWrite);
        } else {
            pte.set_read_write(ReadWriteFlag::ReadOnly);
        }
        if supervisor {
            pte.set_user_supervisor(UserSupervisorFlag::Supervisor);
        } else {
            pte.set_user_supervisor(UserSupervisorFlag::User);
        }

        // Write page table entry.
        self.write_pte(page_address, pte);

        Ok(())
    }

    fn clean(&mut self) {
        for pte in self.entries.iter_mut() {
            *pte = 0;
        }
    }

    fn read_pte(&self, vaddr: PageAddress) -> Option<PageTableEntry> {
        let pte_idx: usize = vaddr.get_pte_index();
        let pte: Option<PageTableEntry> = PageTableEntry::from_raw_value(self.entries[pte_idx]);
        pte
    }

    fn write_pte(&mut self, vaddr: PageAddress, pte: PageTableEntry) {
        let pte_idx: usize = vaddr.get_pte_index();
        self.entries[pte_idx] = pte.into_raw_value();
    }

    pub fn physical_address(&self) -> Result<FrameAddress, Error> {
        Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(
            self.entries.as_ptr() as usize,
        )?)?))
    }
}
