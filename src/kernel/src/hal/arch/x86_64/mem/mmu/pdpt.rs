// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(dead_code)]
//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::{
    AccessPermission,
    Address,
    FrameAddress,
    PageAligned,
    PhysicalAddress,
};
use ::alloc::boxed::Box;
use ::arch::mem::{
    self,
    paging::{
        AccessedFlag,
        FrameNumber,
        NoExecuteFlag,
        PageCacheDisableFlag,
        PageWriteThroughFlag,
        PdptEntry,
        PdptEntryFlags,
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

/// Storage for the PDPT (Page Directory Pointer Table).
/// Each entry is 8 bytes (u64) and there are 512 entries per table.
pub enum PdptStorage {
    Heap(Box<[u64; mem::PAGE_SIZE / core::mem::size_of::<u64>()]>),
}

///
/// # Description
///
/// A type that represents a Page Directory Pointer Table (second level of 4-level paging).
/// In x86_64, each PDPT entry points to a page directory (PD).
///
pub struct Pdpt {
    /// Entries.
    entries: PdptStorage,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl PdptStorage {
    pub fn new() -> Self {
        Self::Heap(Box::new([0; mem::PAGE_SIZE / core::mem::size_of::<u64>()]))
    }
}

impl Deref for PdptStorage {
    type Target = [u64];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Heap(entries) => entries.deref(),
        }
    }
}

impl DerefMut for PdptStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Heap(entries) => entries.deref_mut(),
        }
    }
}

/// Extracts the PDPT entry index from a virtual address (bits 30-38).
fn pdpte_index(vaddr: usize) -> usize {
    (vaddr >> mem::PGDIR_SHIFT) & (mem::PAGE_TABLE_ENTRIES - 1)
}

impl Pdpt {
    pub fn new(entries: PdptStorage) -> Self {
        let mut pdpt: Pdpt = Pdpt { entries };
        pdpt.clean();
        pdpt
    }

    /// Maps a page directory into the PDPT.
    pub fn map(
        &mut self,
        vaddr: usize,
        paddr: FrameAddress,
        supervisor: bool,
        access: AccessPermission,
    ) -> Result<(), Error> {
        let pdpte: PdptEntry = match self.read_pdpte(vaddr) {
            Some(pdpte) => pdpte,
            None => {
                let reason: &str = "failed to read PDPT entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if pdpte.is_present() {
            let reason: &str = "PDPT entry is busy";
            error!("{reason}");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        let pdpte: PdptEntry = PdptEntry::new(
            PdptEntryFlags::new(
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
                NoExecuteFlag::Execute,
            ),
            paddr.into_frame_number(),
        );

        self.write_pdpte(vaddr, pdpte);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps a page directory from the PDPT.
    ///
    pub fn unmap(&mut self, vaddr: usize) -> Result<FrameAddress, Error> {
        let pdpte: PdptEntry = match self.read_pdpte(vaddr) {
            Some(pdpte) => pdpte,
            None => {
                let reason: &str = "failed to read PDPT entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if !pdpte.is_present() {
            let reason: &str = "PDPT entry is not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        let paddr: FrameAddress = FrameAddress::from_frame_number(pdpte.frame())?;

        let pdpte: PdptEntry = PdptEntry::new(
            PdptEntryFlags::new(
                PresentFlag::NotPresent,
                ReadWriteFlag::ReadOnly,
                UserSupervisorFlag::User,
                PageWriteThroughFlag::WriteThrough,
                PageCacheDisableFlag::CacheDisabled,
                AccessedFlag::NotAccessed,
                NoExecuteFlag::Execute,
            ),
            FrameNumber::NULL,
        );

        self.write_pdpte(vaddr, pdpte);

        Ok(paddr)
    }

    ///
    /// # Description
    ///
    /// Looks up a PDPT entry to retrieve the physical address of a page directory.
    ///
    pub fn lookup(&self, vaddr: usize) -> Result<FrameAddress, Error> {
        let pdpte: PdptEntry = match self.read_pdpte(vaddr) {
            Some(pdpte) => pdpte,
            None => {
                let reason: &str = "failed to read PDPT entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if !pdpte.is_present() {
            let reason: &str = "PDPT entry is not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        let paddr: FrameAddress = FrameAddress::from_frame_number(pdpte.frame())?;

        Ok(paddr)
    }

    pub fn clean(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = 0;
        }
    }

    fn read_pdpte(&self, vaddr: usize) -> Option<PdptEntry> {
        let idx: usize = pdpte_index(vaddr);
        PdptEntry::from_raw_value(self.entries[idx])
    }

    fn write_pdpte(&mut self, vaddr: usize, pdpte: PdptEntry) {
        let idx: usize = pdpte_index(vaddr);
        self.entries[idx] = pdpte.into_raw_value();
    }

    pub fn physical_address(&self) -> Result<FrameAddress, Error> {
        Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(
            self.entries.as_ptr() as usize,
        )?)?))
    }
}
