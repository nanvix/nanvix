// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(verus_keep_ghost_body)]
use super::page_table::PageTable;
#[cfg(verus_keep_ghost_body)]
use crate::mm::PageTableStorage;
use crate::{
    hal::mem::{
        AccessPermission,
        Address,
        FrameAddress,
        PageAligned,
        PageTableAddress,
        PhysicalAddress,
    },
    mm::GetPageDirectoryStorage,
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
    PteWord,
    ReadWriteFlag,
    UserSupervisorFlag,
};
use ::core::ops::DerefMut;
use ::sys::error::{
    Error,
    ErrorCode,
};

include!("page_directory.spec.rs");

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a page directory.
///
#[verus_verify]
pub struct PageDirectory<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageDirectoryStorage,
{
    /// Entries.
    entries: T,
    /// Specification tokens for page-directory entries.
    #[cfg(verus_keep_ghost_body)]
    pub permissions: Tracked<Map<nat, NanvixPdeToken>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

#[verus_verify]
impl<T> PageDirectory<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageDirectoryStorage,
{
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        with
            Tracked(raw_permissions):
                Tracked<Map<nat, PointsTo<PteWord>>>,
        requires
            raw_permissions.dom().len() == ::arch::mem::PAGE_TABLE_LENGTH,
            forall|i: nat| raw_permissions.dom().contains(i)
                <==> 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH,
            forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
                let permission = #[trigger] raw_permissions[i];

                permission.ptr()@.addr as int
                    == entries.get_storage().base_address()
                        + i * 4
                    && permission.is_uninit()
            },
        ensures
            result.inv(),
            forall|i: nat| 0 <= i < result.permissions.dom().len()
                ==> {
                    &&& result.permissions[i].is_init()
                    &&& result.permissions[i].expected() == 0
                },
    )]
    pub fn new(entries: T) -> Self {
        let mut pgdir: PageDirectory<T> = PageDirectory {
            entries,
            #[cfg(verus_keep_ghost_body)]
            permissions: Tracked::new(mint_nanvix_pde_tokens(raw_permissions)),
        };
        pgdir.clean();
        pgdir
    }
}

impl<T> PageDirectory<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageDirectoryStorage,
{
    #[verus_verify(external_body)]
    #[verus_spec(
        with
            Ghost(page_table):
                Ghost<&PageTable<PageTableStorage>>,
        requires
            page_table.ready_for_mmu(),
            page_table.physical_base() == paddr@,
    )]
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
                PageWriteThroughFlag::NotWriteThrough,
                PageCacheDisableFlag::CacheEnabled,
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
                PageSizeFlag::Standard,
            ),
            paddr.into_frame_number(),
        );

        // Write page directory entry
        proof_with! {
            Ghost(Some(page_table))
        };
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
        proof_with! {
            Ghost(None)
        };
        self.write_pde(pgtable_address, pde);

        // Invalidate the TLB entry for this page table range so the CPU does not use a
        // stale PDE pointing to the freed page table.
        // SAFETY: called from kernel mode after modifying a PDE.
        unsafe { ::arch::mem::paging::invlpg(pgtable_address.into_raw_value()) };

        Ok(paddr)
    }

    fn clean(&mut self) {
        // for pde in self.entries.iter_mut() {
        //     *pde = 0;
        // }
        self.env_interaction_clear_page_directory();
    }

    pub fn read_pde(&self, vaddr: PageTableAddress) -> Option<PageDirectoryEntry> {
        let pde_idx: usize = vaddr.get_pde_index();
        // PageDirectoryEntry::from_raw_value(self.entries[pde_idx])
        PageDirectoryEntry::from_raw_value(self.env_interaction_read_page_directory_entry(pde_idx))
    }

    #[verus_verify(external_body)]
    #[verus_spec(
        with
            Ghost(page_table):
                Ghost<Option<&PageTable<PageTableStorage>>>,
    )]
    fn write_pde(&mut self, vaddr: PageTableAddress, pde: PageDirectoryEntry) {
        let pde_idx: usize = vaddr.get_pde_index();
        // self.entries[pde_idx] = pde.into_raw_value();
        proof_with! {
            Ghost(page_table)
        };
        self.env_interaction_write_page_directory_entry(pde_idx, pde.into_raw_value());
    }

    pub fn physical_address(&self) -> Result<FrameAddress, Error> {
        let vaddr: usize = self.entries.as_ptr() as usize;
        let paddr: usize = crate::hal::platform::virt_to_phys(vaddr);
        Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(paddr)?)?))
    }
}
