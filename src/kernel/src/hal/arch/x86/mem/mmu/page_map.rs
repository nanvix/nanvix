// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// TODO: remove this.
#![allow(clippy::type_complexity)]

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
            PageTableAligned,
            PhysicalAddress,
            Table,
            VirtualAddress,
        },
    },
    mm::{
        KernelPage,
        PageTableStorage,
    },
};
use ::alloc::{
    collections::LinkedList,
    rc::Rc,
};
use ::arch::mem::{
    paging::{
        AccessedFlag,
        DirtyFlag,
        FrameNumber,
        PageCacheDisableFlag,
        PageDirectoryEntry,
        PageDirectoryEntryFlags,
        PageSizeFlag,
        PageTableEntry,
        PageWriteThroughFlag,
        PresentFlag,
        ReadWriteFlag,
        TableIndex,
        UserSupervisorFlag,
    },
    PAGE_TABLE_LENGTH,
    PGTAB_ALIGNMENT,
};
use ::core::{
    cell::RefCell,
    ops::ControlFlow,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of kernel pages required to clone a `PageMap`.
/// On x86, only the page directory needs a backing page.
pub const PAGE_MAP_CLONE_PAGES: usize = 1;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Architecture-specific page table hierarchy.
///
/// On x86, this wraps a two-level page directory and page table structure. The walk logic
/// (PD → PT) is fully encapsulated here so that the generic virtual-memory layer does not
/// need architecture-specific code.
///
pub struct PageMap {
    /// Physical address of the root page directory.
    pd_paddr: PageDirectoryAddress,
    /// Backing pages for hardware paging structures (keeps memory alive).
    _hw_pages: LinkedList<PageTableStorage>,
    /// Kernel page tables (shared across all address spaces via Rc).
    kernel_page_tables:
        LinkedList<Rc<RefCell<(PageTableAligned<VirtualAddress>, PageTableStorage)>>>,
    /// User page tables (owned by this address space).
    user_page_tables: LinkedList<(PageTableAligned<VirtualAddress>, PageTableStorage)>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl PageMap {
    ///
    /// # Description
    ///
    /// Creates a root page map from BSS storage during boot.
    ///
    pub fn new_boot(
        kernel_page_tables: LinkedList<(PageTableAligned<VirtualAddress>, PageTableStorage)>,
    ) -> Result<Self, Error> {
        // Allocate PD from unified BSS pool.
        // SAFETY: called during early single-threaded init;
        // BSS is zero-initialized, so assume_init_mut() is sound.
        let pd_storage: PageTableStorage = PageTableStorage::Bss(unsafe {
            crate::mm::virt::page_table_allocator::alloc_page_table_slot()
        });
        let pd_paddr: PageDirectoryAddress =
            PageDirectoryAddress::from_raw_value(pd_storage.as_ptr() as usize)?;
        // SAFETY: pd_paddr is valid and identity-mapped.
        unsafe { Self::clean_page_directory(pd_paddr) };

        let mut hw_pages: LinkedList<PageTableStorage> = LinkedList::new();
        hw_pages.push_back(pd_storage);

        let mut kpage_tables: LinkedList<
            Rc<RefCell<(PageTableAligned<VirtualAddress>, PageTableStorage)>>,
        > = LinkedList::new();
        let mut list: LinkedList<(PageTableAligned<VirtualAddress>, PageTableStorage)> =
            kernel_page_tables;
        while let Some((vaddr, storage)) = list.pop_front() {
            let page_table_address: FrameAddress = FrameAddress::new(PageAligned::from_address(
                PhysicalAddress::from_raw_value(storage.as_ptr() as usize)?,
            )?);
            let pde_idx = ::arch::mem::paging::pd_index(vaddr.into_raw_value());
            unsafe { Self::install_kernel_pt(pd_paddr, pde_idx, page_table_address) };
            kpage_tables.push_back(Rc::new(RefCell::new((vaddr, storage))));
        }

        // Register kernel PD (also the CR3 root on x86) for lazy identity mapping.
        crate::mm::virt::identity_map::init(pd_paddr, pd_paddr)?;

        Ok(Self {
            pd_paddr,
            _hw_pages: hw_pages,
            kernel_page_tables: kpage_tables,
            user_page_tables: LinkedList::new(),
        })
    }

    ///
    /// # Description
    ///
    /// Clones a page map for a new process, sharing kernel page tables.
    ///
    pub fn new_clone(from: &PageMap, mut pages: LinkedList<KernelPage>) -> Result<Self, Error> {
        let pgdir_page: KernelPage = match pages.pop_front() {
            Some(p) => p,
            None => {
                let reason: &str = "no pages provided for page directory";
                error!("{reason}");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        let pd_storage: PageTableStorage = PageTableStorage::KernelPage(pgdir_page);
        let pd_paddr: PageDirectoryAddress =
            PageDirectoryAddress::from_raw_value(pd_storage.as_ptr() as usize)?;
        // SAFETY: pd_paddr is valid and identity-mapped.
        unsafe { Self::clean_page_directory(pd_paddr) };

        let mut hw_pages: LinkedList<PageTableStorage> = LinkedList::new();
        hw_pages.push_back(pd_storage);

        let mut kernel_page_tables: LinkedList<
            Rc<RefCell<(PageTableAligned<VirtualAddress>, PageTableStorage)>>,
        > = LinkedList::new();
        for entry in from.kernel_page_tables.iter() {
            let borrowed = entry.borrow();
            let page_table_address: FrameAddress = FrameAddress::new(PageAligned::from_address(
                PhysicalAddress::from_raw_value(borrowed.1.as_ptr() as usize)?,
            )?);
            let pde_idx = ::arch::mem::paging::pd_index(borrowed.0.into_raw_value());
            unsafe { Self::install_kernel_pt(pd_paddr, pde_idx, page_table_address) };
            kernel_page_tables.push_back(entry.clone());
        }

        Ok(Self {
            pd_paddr,
            _hw_pages: hw_pages,
            kernel_page_tables,
            user_page_tables: LinkedList::new(),
        })
    }

    /// Returns the CR3 value (physical address of the page directory).
    pub fn cr3_value(&self) -> Result<usize, Error> {
        Ok(self.pd_paddr.into_raw_value())
    }

    /// Loads this page map into hardware (writes CR3 and enables paging).
    pub unsafe fn load(&self) -> Result<(), Error> {
        core::arch::asm!(
            "mov {0}, %eax",
            "mov %eax, %cr3",
            "mov %cr0, %eax",
            "or $0x80000000, %eax",
            "mov %eax, %cr0",
            in(reg) self.pd_paddr.into_raw_value(),
            options(nostack, att_syntax)
        );
        Ok(())
    }

    /// Copies data from user space to kernel space using physical memory access.
    ///
    /// # Safety
    ///
    /// The caller must ensure that both pointers and the physical address are valid and the copy
    /// region does not overflow.
    pub unsafe fn copy_from_user(
        &self,
        kernel_dst: *mut u8,
        _user_src_va: usize,
        user_src_pa: usize,
        size: usize,
    ) {
        crate::mm::virt::identity_map::phys_memcpy(kernel_dst, user_src_pa as *const u8, size)
            .expect("phys_memcpy failed in copy_from_user");
    }

    /// Copies data from kernel space to user space using physical memory access.
    ///
    /// # Safety
    ///
    /// The caller must ensure that both pointers and the physical address are valid and the copy
    /// region does not overflow.
    pub unsafe fn copy_to_user(
        &self,
        _user_dst_va: usize,
        user_dst_pa: usize,
        kernel_src: *const u8,
        size: usize,
    ) {
        let dst: *mut u8 = user_dst_pa as *mut u8;
        let result: Result<(), _> = if size.is_multiple_of(::core::mem::size_of::<u32>()) {
            crate::mm::virt::identity_map::phys_memcpy32(dst, kernel_src, size)
        } else {
            crate::mm::virt::identity_map::phys_memcpy(dst, kernel_src, size)
        };
        result.expect("phys_memcpy failed in copy_to_user");
    }

    /// Fills user-space memory with a value using physical memory access.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the physical address is valid and the region does not overflow.
    #[allow(dead_code)]
    pub unsafe fn memset_user(&self, _user_va: usize, user_pa: usize, value: u8, size: usize) {
        crate::mm::virt::identity_map::phys_memset32(user_pa as *mut u8, value, size)
            .expect("phys_memset32 failed in memset_user");
    }

    ///
    /// # Description
    ///
    /// Maps a user page into the address space.
    ///
    pub fn map_user_page<T: Fn() -> Result<PageTableStorage, Error>>(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        frame: FrameAddress,
        access: AccessPermission,
        page_table_allocator: T,
    ) -> Result<(), Error> {
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };
        let alloc = pgdir.map_page(vaddr, frame, access, &page_table_allocator)?;
        if let Some(entry) = alloc.pt {
            self.user_page_tables.push_back(entry);
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps a user page. Returns the frame address if the page was present.
    ///
    pub fn unmap_user_page(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<FrameAddress>, Error> {
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };
        let (frame, pt_freed) = pgdir.unmap_page(vaddr)?;
        if pt_freed {
            let pt_addr: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
                ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
            )?;
            if let Some(at) = self
                .user_page_tables
                .iter()
                .position(|(addr, _)| *addr == pt_addr)
            {
                self.user_page_tables.remove(at);
            }
        }
        Ok(frame)
    }

    /// Looks up the frame address for a user page.
    pub fn lookup_user_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<FrameAddress, Error> {
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };
        pgdir.lookup_page(vaddr)
    }

    /// Tries to look up a user page. Returns `None` if the page is not present.
    pub fn try_lookup_user_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<FrameAddress>, Error> {
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };
        pgdir.try_lookup_page(vaddr)
    }

    /// Changes access permissions on a user page.
    pub fn ctrl_user_page(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };
        pgdir.ctrl_page(vaddr, access)
    }

    ///
    /// # Description
    ///
    /// Maps a kernel page into the address space.
    ///
    pub fn map_kernel_page<T: Fn() -> Result<PageTableStorage, Error>>(
        &mut self,
        frame: FrameAddress,
        vaddr: PageAligned<VirtualAddress>,
        page_table_allocator: T,
    ) -> Result<(), Error> {
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };
        let alloc = pgdir.map_page(vaddr, frame, AccessPermission::RDWR, &page_table_allocator)?;
        if let Some(entry) = alloc.pt {
            self.kernel_page_tables
                .push_back(Rc::new(RefCell::new(entry)));
        }
        Ok(())
    }

    /// Changes access permissions on a kernel page.
    pub fn ctrl_kernel_page(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
        dry_run: bool,
    ) -> Result<(), Error> {
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };

        if dry_run {
            // Verify the page exists via page directory lookup.
            if pgdir.try_lookup_page(vaddr)?.is_none() {
                let reason: &str = "page table entry not found";
                error!("{reason}");
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            }
            return Ok(());
        }

        // Permission change on identity-mapped kernel page.
        pgdir.ctrl_page(vaddr, access)
    }

    /// Zeroes all PDE entries in the page directory at `pd_paddr`.
    unsafe fn clean_page_directory(pd_addr: PageDirectoryAddress) {
        let table: Table<PageDirectoryEntry> = Table::from_address(pd_addr.into_raw_value());
        for i in 0..PAGE_TABLE_LENGTH {
            let idx = TableIndex::try_from(i).expect("index within bounds");
            table.write(
                idx,
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
            );
        }
    }

    /// Installs a page table frame into the page directory at the given PDE index.
    unsafe fn install_kernel_pt(
        pd_addr: PageDirectoryAddress,
        pde_idx: TableIndex,
        frame: FrameAddress,
    ) {
        let table: Table<PageDirectoryEntry> = Table::from_address(pd_addr.into_raw_value());
        let pde: PageDirectoryEntry = PageDirectoryEntry::new(
            PageDirectoryEntryFlags::new(
                PresentFlag::Present,
                ReadWriteFlag::ReadWrite,
                UserSupervisorFlag::User,
                PageWriteThroughFlag::WriteThrough,
                PageCacheDisableFlag::CacheDisabled,
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
                PageSizeFlag::Standard,
            ),
            frame.into_frame_number(),
        );
        table.write(pde_idx, pde);
    }
}

//==================================================================================================
// Copy-on-Write / Iteration Support
//==================================================================================================

impl PageMap {
    /// Reads the user page-table entry for `vaddr` (None if not mapped/present).
    pub fn try_lookup_user_pte(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<PageTableEntry>, Error> {
        // SAFETY: pd_paddr is valid and identity-mapped.
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };
        pgdir.try_read_pte(vaddr)
    }

    /// Marks the user page at `vaddr` copy-on-write.
    pub fn mark_user_page_cow(&mut self, vaddr: PageAligned<VirtualAddress>) -> Result<(), Error> {
        // SAFETY: pd_paddr is valid and identity-mapped.
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };
        pgdir.mark_cow_page(vaddr)
    }

    /// Clears the copy-on-write mark on the user page at `vaddr`.
    pub fn unmark_user_page_cow(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        // SAFETY: pd_paddr is valid and identity-mapped.
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };
        pgdir.unmark_cow_page(vaddr)
    }

    /// Repoints the copy-on-write page at `vaddr` to `new_frame`. Returns the old frame.
    pub fn replace_user_page_cow_frame(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        new_frame: FrameAddress,
    ) -> Result<FrameAddress, Error> {
        // SAFETY: pd_paddr is valid and identity-mapped.
        let pgdir: PageDirectory = unsafe { PageDirectory::from_address(self.pd_paddr) };
        pgdir.replace_cow_frame_page(vaddr, new_frame)
    }

    /// Iterates all present user mappings, invoking `f(vaddr, pte)`. Returning
    /// `Ok(ControlFlow::Break(()))` stops the walk early.
    pub fn try_for_each_user_mapping<F>(&self, mut f: F) -> Result<(), Error>
    where
        F: FnMut(PageAligned<VirtualAddress>, PageTableEntry) -> Result<ControlFlow<()>, Error>,
    {
        for (pt_vaddr, storage) in self.user_page_tables.iter() {
            let base: usize = pt_vaddr.into_raw_value();
            // SAFETY: the page table storage is valid and identity-mapped.
            let pt: Table<PageTableEntry> =
                unsafe { Table::from_address(storage.as_ptr() as usize) };
            for i in 0..PAGE_TABLE_LENGTH {
                let idx = TableIndex::try_from(i)?;
                // SAFETY: idx is within bounds.
                let pte: PageTableEntry = match unsafe { pt.read(idx) } {
                    Some(p) => p,
                    None => continue,
                };
                if !pte.is_present() {
                    continue;
                }
                let raw_vaddr: usize = base
                    .checked_add(
                        i.checked_mul(::arch::mem::PAGE_SIZE).ok_or_else(|| {
                            Error::new(ErrorCode::BadAddress, "pte offset overflow")
                        })?,
                    )
                    .ok_or_else(|| {
                        Error::new(ErrorCode::BadAddress, "user mapping vaddr overflow")
                    })?;
                let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(raw_vaddr)?;
                if f(vaddr, pte)?.is_break() {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

impl Drop for PageMap {
    fn drop(&mut self) {
        while let Some((_vaddr, _storage)) = self.user_page_tables.pop_front() {
            // Storage dropped here, freeing the backing page.
        }
        while let Some(entry) = self.kernel_page_tables.pop_front() {
            drop(entry);
        }
    }
}
