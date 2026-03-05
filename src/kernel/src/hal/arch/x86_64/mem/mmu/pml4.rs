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
        Pml4Entry,
        Pml4EntryFlags,
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

/// Storage for the PML4 (Page Map Level 4).
/// Each entry is 8 bytes (u64) and there are 512 entries per table.
pub enum Pml4Storage {
    Heap(Box<[u64; mem::PAGE_SIZE / core::mem::size_of::<u64>()]>),
}

///
/// # Description
///
/// A type that represents a PML4 table (top level of 4-level paging).
/// In x86_64, each PML4 entry points to a PDPT.
///
pub struct Pml4 {
    /// Entries.
    entries: Pml4Storage,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Pml4Storage {
    pub fn new() -> Self {
        Self::Heap(Box::new([0; mem::PAGE_SIZE / core::mem::size_of::<u64>()]))
    }
}

impl Deref for Pml4Storage {
    type Target = [u64];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Heap(entries) => entries.deref(),
        }
    }
}

impl DerefMut for Pml4Storage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Heap(entries) => entries.deref_mut(),
        }
    }
}

/// Extracts the PML4 entry index from a virtual address (bits 39-47).
fn pml4e_index(vaddr: usize) -> usize {
    (vaddr >> mem::PDPT_SHIFT) & (mem::PAGE_TABLE_ENTRIES - 1)
}

impl Pml4 {
    pub fn new(entries: Pml4Storage) -> Self {
        let mut pml4: Pml4 = Pml4 { entries };
        pml4.clean();
        pml4
    }

    /// Maps a PDPT into the PML4.
    pub fn map(
        &mut self,
        vaddr: usize,
        paddr: FrameAddress,
        supervisor: bool,
        access: AccessPermission,
    ) -> Result<(), Error> {
        let pml4e: Pml4Entry = match self.read_pml4e(vaddr) {
            Some(pml4e) => pml4e,
            None => {
                let reason: &str = "failed to read PML4 entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if pml4e.is_present() {
            let reason: &str = "PML4 entry is busy";
            error!("{reason}");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        let pml4e: Pml4Entry = Pml4Entry::new(
            Pml4EntryFlags::new(
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

        self.write_pml4e(vaddr, pml4e);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps a PDPT from the PML4.
    ///
    pub fn unmap(&mut self, vaddr: usize) -> Result<FrameAddress, Error> {
        let pml4e: Pml4Entry = match self.read_pml4e(vaddr) {
            Some(pml4e) => pml4e,
            None => {
                let reason: &str = "failed to read PML4 entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if !pml4e.is_present() {
            let reason: &str = "PML4 entry is not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        let paddr: FrameAddress = FrameAddress::from_frame_number(pml4e.frame())?;

        let pml4e: Pml4Entry = Pml4Entry::new(
            Pml4EntryFlags::new(
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

        self.write_pml4e(vaddr, pml4e);

        Ok(paddr)
    }

    ///
    /// # Description
    ///
    /// Looks up a PML4 entry to retrieve the physical address of a PDPT.
    ///
    pub fn lookup(&self, vaddr: usize) -> Result<FrameAddress, Error> {
        let pml4e: Pml4Entry = match self.read_pml4e(vaddr) {
            Some(pml4e) => pml4e,
            None => {
                let reason: &str = "failed to read PML4 entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if !pml4e.is_present() {
            let reason: &str = "PML4 entry is not present";
            error!("{reason}");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        let paddr: FrameAddress = FrameAddress::from_frame_number(pml4e.frame())?;

        Ok(paddr)
    }

    pub fn clean(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = 0;
        }
    }

    fn read_pml4e(&self, vaddr: usize) -> Option<Pml4Entry> {
        let idx: usize = pml4e_index(vaddr);
        Pml4Entry::from_raw_value(self.entries[idx])
    }

    fn write_pml4e(&mut self, vaddr: usize, pml4e: Pml4Entry) {
        let idx: usize = pml4e_index(vaddr);
        self.entries[idx] = pml4e.into_raw_value();
    }

    pub fn physical_address(&self) -> Result<FrameAddress, Error> {
        Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(
            self.entries.as_ptr() as usize,
        )?)?))
    }
}
