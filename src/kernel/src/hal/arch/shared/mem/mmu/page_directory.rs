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
use ::arch::mem::paging::{
    AccessedFlag,
    DirtyFlag,
    FrameNumber,
    PageCacheDisableFlag,
    PageDirectoryEntry,
    PageDirectoryEntryFlags,
    PageSizeFlag,
    PageWriteThroughFlag,
    PresentFlag,
    ReadWriteFlag,
    UserSupervisorFlag,
};
use ::core::ops::DerefMut;
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
pub struct PageDirectory<T: DerefMut<Target = [u32]>> {
    /// Entries.
    entries: T,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T: DerefMut<Target = [u32]>> PageDirectory<T> {
    pub fn new(entries: T) -> Self {
        let mut pgdir: PageDirectory<T> = PageDirectory { entries };
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
        // Obtain a cached copy of the page directory entry.
        let pde: PageDirectoryEntry = match self.read_pde(vaddr) {
            Some(pde) => pde,
            None => {
                let reason: &str = "failed to read page directory entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page directory entry is busy.
        if pde.is_present() {
            let reason: &str = "page directory entry is busy";
            error!("{reason}");
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
                PageSizeFlag::Standard,
            ),
            paddr.into_frame_number(),
        );

        // Write page directory entry
        self.write_pde(vaddr, pde);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps a page from the page directory.
    ///
    /// # Parameters
    ///
    /// - `pgtable_address`: Page table address.
    ///
    /// # Returns
    ///
    /// Upon successful completion, `Ok(())` is returned. Upon failure, an error is returned
    /// instead.
    ///
    pub fn unmap(&mut self, pgtable_address: PageTableAddress) -> Result<FrameAddress, Error> {
        // Obtain a cached copy of the page directory entry.
        let pde: PageDirectoryEntry = match self.read_pde(pgtable_address) {
            Some(pde) => pde,
            None => {
                let reason: &str = "failed to read page directory entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page directory entry is present.
        if !pde.is_present() {
            let reason: &str = "page directory entry is not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        // Retrieve frame address.
        let paddr: FrameAddress = FrameAddress::from_frame_number(pde.frame_number())?;

        // Construct page directory entry.
        let pde: PageDirectoryEntry = PageDirectoryEntry::new(
            PageDirectoryEntryFlags::new(
                PresentFlag::NotPresent,
                ReadWriteFlag::ReadOnly,
                UserSupervisorFlag::User,
                PageWriteThroughFlag::WriteThrough,
                PageCacheDisableFlag::CacheDisabled,
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
                PageSizeFlag::Standard,
            ),
            FrameNumber::NULL,
        );

        // Write page directory entry.
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
