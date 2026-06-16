// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        AccessPermission,
        Address,
        FrameAddress,
        PageAligned,
        PageDirectoryAddress,
        PageTableAligned,
        PdptAddress,
        PhysicalAddress,
        Pml4Address,
        VirtualAddress,
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
        PageCacheDisableFlag,
        PageDirectoryEntryFlags,
        PageTableEntryFlags,
        PageWriteThroughFlag,
        PresentFlag,
        ReadWriteFlag,
        TableIndex,
        UserSupervisorFlag,
    },
    PGTAB_ALIGNMENT,
};
use ::core::cell::RefCell;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of kernel pages required to clone a `PageMap`.
/// On x86_64: PML4 + PDPT + PD = 3 pages.
pub const PAGE_MAP_CLONE_PAGES: usize = 3;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Architecture-specific page table hierarchy.
///
/// On x86_64, this wraps the hardware 4-level page table hierarchy (PML4 → PDPT → PD → PT).
/// Like the x86 version, it maintains software page table objects for user-space mappings,
/// providing the same API as the x86 `PageMap` — the generic virtual-memory layer does not
/// need architecture-specific code.
///
/// The kernel builds its own page tables during boot (via `virt::init()`), and the trampoline's
/// temporary 2 MiB identity-map is abandoned once `load()` switches CR3 to the per-process PML4.
///
pub struct PageMap {
    /// Physical address of the per-process PML4.
    pml4_paddr: Pml4Address,
    /// Backing pages for hardware paging structures (keeps memory alive).
    /// For boot: BSS-backed PageTableStorage for PML4/PDPT/PD.
    /// For clone: KernelPage-backed PageTableStorage for PML4/PDPT/PD.
    hw_pages: LinkedList<PageTableStorage>,
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
    /// Creates a root page map during boot.
    ///
    /// Allocates a PML4, PDPT, and PD from the static BSS pool and installs the kernel page
    /// tables (from `virt::init()`) into the PD — mirroring the x86 pattern. User space gets
    /// a separate empty PD wired via PDPT[1].
    ///
    pub fn new_boot(
        kernel_page_tables: LinkedList<(PageTableAligned<VirtualAddress>, PageTableStorage)>,
    ) -> Result<Self, Error> {
        // Allocate PML4, PDPT, kernel PD from unified BSS pool.
        // SAFETY: called during single-threaded boot with the trampoline page tables still in CR3;
        // BSS is zero-initialized, so assume_init_mut() is sound.
        unsafe {
            let alloc = crate::mm::virt::page_table_allocator::alloc_page_table_slot;
            let pml4_slot = alloc();
            let pdpt_slot = alloc();
            let pd_kernel_slot = alloc();
            let pd_user_slot = alloc();

            let pml4_addr = Pml4Address::from_raw_value(pml4_slot as *mut _ as usize)?;
            let pdpt_addr = PdptAddress::from_raw_value(pdpt_slot as *mut _ as usize)?;
            let pd_kernel_addr =
                PageDirectoryAddress::from_raw_value(pd_kernel_slot as *mut _ as usize)?;
            let pd_user_addr =
                PageDirectoryAddress::from_raw_value(pd_user_slot as *mut _ as usize)?;

            let pml4: super::pml4::Pml4 = super::pml4::Pml4::from_address(pml4_addr);
            let pdpt: super::pdpt::Pdpt = super::pdpt::Pdpt::from_address(pdpt_addr);

            // PDPT[0] → kernel PD (supervisor-accessible kernel space).
            pdpt.install_pd(0, FrameAddress::from_raw_value(pd_kernel_addr.into_raw_value())?);
            // PDPT[1] → user PD (user-accessible user space).
            pdpt.install_pd(1, FrameAddress::from_raw_value(pd_user_addr.into_raw_value())?);
            // PML4[0] → PDPT.
            pml4.install_pdpt(0, FrameAddress::from_raw_value(pdpt_addr.into_raw_value())?);

            // Install kernel page tables into the kernel PD — same pattern as x86.
            let mut kpt_list: LinkedList<
                Rc<RefCell<(PageTableAligned<VirtualAddress>, PageTableStorage)>>,
            > = LinkedList::new();
            let mut kpts: LinkedList<(PageTableAligned<VirtualAddress>, PageTableStorage)> =
                kernel_page_tables;
            while let Some((vaddr, storage)) = kpts.pop_front() {
                let page_table_address: FrameAddress =
                    FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(
                        storage.as_ptr() as usize,
                    )?)?);
                let pde_idx = ::arch::mem::paging::pd_index(vaddr.into_raw_value());
                Self::install_kernel_pt(pd_kernel_addr, pde_idx, page_table_address);
                kpt_list.push_back(Rc::new(RefCell::new((vaddr, storage))));
            }

            // Keep the BSS pages alive.
            let mut hw_pages: LinkedList<PageTableStorage> = LinkedList::new();
            hw_pages.push_back(PageTableStorage::Bss(pml4_slot));
            hw_pages.push_back(PageTableStorage::Bss(pdpt_slot));
            hw_pages.push_back(PageTableStorage::Bss(pd_kernel_slot));
            hw_pages.push_back(PageTableStorage::Bss(pd_user_slot));

            // Register kernel PD and PML4 (CR3) for lazy identity mapping.
            crate::mm::virt::identity_map::init(pd_kernel_addr, pml4_addr);

            Ok(Self {
                pml4_paddr: pml4_addr,
                hw_pages,
                kernel_page_tables: kpt_list,
                user_page_tables: LinkedList::new(),
            })
        }
    }

    ///
    /// # Description
    ///
    /// Clones a page map for a new process, sharing kernel page tables.
    ///
    /// The provided kernel pages back the PML4, PDPT, and PD structures. Kernel page tables
    /// are shared with the parent via Rc.
    ///
    pub fn new_clone(from: &PageMap, mut pages: LinkedList<KernelPage>) -> Result<Self, Error> {
        if pages.len() < PAGE_MAP_CLONE_PAGES {
            let reason: &str = "insufficient pages for page map clone";
            error!("{reason} (need={}, got={})", PAGE_MAP_CLONE_PAGES, pages.len());
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let pml4_page: KernelPage = pages
            .pop_front()
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "missing PML4 page"))?;
        let pdpt_page: KernelPage = pages
            .pop_front()
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "missing PDPT page"))?;
        let pd_page: KernelPage = pages
            .pop_front()
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "missing PD page"))?;

        let pml4_addr = Pml4Address::from_raw_value(pml4_page.frame_address().into_raw_value())?;
        let pdpt_addr = PdptAddress::from_raw_value(pdpt_page.frame_address().into_raw_value())?;
        let pd_addr =
            PageDirectoryAddress::from_raw_value(pd_page.frame_address().into_raw_value())?;

        // Keep the pages alive as PageTableStorage.
        let mut hw_pages: LinkedList<PageTableStorage> = LinkedList::new();
        hw_pages.push_back(PageTableStorage::KernelPage(pml4_page));
        hw_pages.push_back(PageTableStorage::KernelPage(pdpt_page));
        hw_pages.push_back(PageTableStorage::KernelPage(pd_page));

        // Wire the hierarchy using typed wrappers.
        unsafe {
            let pml4: super::pml4::Pml4 = super::pml4::Pml4::from_address(pml4_addr);
            let pdpt: super::pdpt::Pdpt = super::pdpt::Pdpt::from_address(pdpt_addr);

            // PDPT[1] → new PD (user space).
            pdpt.install_pd(1, FrameAddress::from_raw_value(pd_addr.into_raw_value())?);
            // PML4[0] → PDPT.
            pml4.install_pdpt(0, FrameAddress::from_raw_value(pdpt_addr.into_raw_value())?);
        }

        // Share kernel page tables with the parent via Rc and install into a dedicated kernel PD.
        // Kernel PTs point to the same backing memory — the PDE in each clone's PD references
        // the same physical page table frame.
        let mut kernel_page_tables: LinkedList<
            Rc<RefCell<(PageTableAligned<VirtualAddress>, PageTableStorage)>>,
        > = LinkedList::new();

        // We need a kernel PD for this clone. Allocate from the remaining pages if available,
        // otherwise share the parent's kernel PD via PDPT[0].
        // Since PAGE_MAP_CLONE_PAGES = 3 (PML4 + PDPT + user PD), the kernel PD is shared
        // by pointing PDPT[0] to the parent's kernel PD.
        unsafe {
            // Walk the parent's PML4 → PDPT → PD[0] to find the kernel PD address.
            let parent_pml4_table: crate::hal::mem::Table<::arch::mem::paging::Pml4Entry> =
                crate::hal::mem::Table::from_address(from.pml4_paddr.into_raw_value());
            let pml4_entry = parent_pml4_table
                .read(TableIndex::new(0).unwrap())
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
            let parent_pdpt_table: crate::hal::mem::Table<::arch::mem::paging::PdptEntry> =
                crate::hal::mem::Table::from_address(pml4_entry.frame_address());
            let pdpt_entry = parent_pdpt_table
                .read(TableIndex::new(0).unwrap())
                .ok_or_else(|| Error::new(ErrorCode::BadAddress, "invalid page table entry"))?;
            let kernel_pd_addr = PageDirectoryAddress::from_raw_value(pdpt_entry.frame_address())?;

            // PDPT[0] → parent's kernel PD (shared kernel mapping).
            let pdpt: super::pdpt::Pdpt = super::pdpt::Pdpt::from_address(pdpt_addr);
            pdpt.install_pd(0, FrameAddress::from_raw_value(kernel_pd_addr.into_raw_value())?);
        }

        for entry in from.kernel_page_tables.iter() {
            kernel_page_tables.push_back(Rc::clone(entry));
        }

        Ok(Self {
            pml4_paddr: pml4_addr,
            hw_pages,
            kernel_page_tables,
            user_page_tables: LinkedList::new(),
        })
    }

    /// Returns the CR3 value (physical address of the PML4).
    pub fn cr3_value(&self) -> Result<usize, Error> {
        Ok(self.pml4_paddr.into_raw_value())
    }

    /// Loads this page map into hardware (writes CR3).
    pub unsafe fn load(&self) -> Result<(), Error> {
        core::arch::asm!("mov cr3, {}", in(reg) self.pml4_paddr.into_raw_value(), options(nostack));
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
    /// On x86_64, this walks the 4-level hierarchy to find or allocate the PT covering the
    /// target virtual address, then maps the page. If a new PT is needed, it is allocated via
    /// `page_table_allocator`, installed into the PD, and tracked in `user_page_tables`.
    ///
    pub fn map_user_page<T: Fn() -> Result<PageTableStorage, Error>>(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        frame: FrameAddress,
        access: AccessPermission,
        page_table_allocator: T,
    ) -> Result<(), Error> {
        // SAFETY: pml4_paddr is a valid PML4 address.
        unsafe {
            let pml4: super::pml4::Pml4 = super::pml4::Pml4::from_address(self.pml4_paddr);
            let alloc: super::pml4::Pml4Alloc =
                pml4.map_page(vaddr, frame, access, &page_table_allocator)?;
            if let Some(storage) = alloc.pdpt_page {
                self.hw_pages.push_back(storage);
            }
            if let Some(storage) = alloc.pdpt.pd_page {
                self.hw_pages.push_back(storage);
            }
            if let Some(entry) = alloc.pdpt.pd.pt {
                self.user_page_tables.push_back(entry);
            }
            Ok(())
        }
    }

    /// Unmaps a user page. Returns the frame address if the page was present.
    pub fn unmap_user_page(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<FrameAddress>, Error> {
        // SAFETY: pml4_paddr is a valid PML4 address.
        unsafe {
            let pml4: super::pml4::Pml4 = super::pml4::Pml4::from_address(self.pml4_paddr);
            let (frame, pt_freed) = pml4.unmap_page(vaddr)?;
            if pt_freed {
                let pt_addr: PageTableAligned<VirtualAddress> = Self::pt_address_for(vaddr)?;
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
    }

    /// Looks up the frame address for a user page.
    pub fn lookup_user_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<FrameAddress, Error> {
        // SAFETY: pml4_paddr is a valid PML4 address.
        unsafe {
            let pml4: super::pml4::Pml4 = super::pml4::Pml4::from_address(self.pml4_paddr);
            pml4.lookup_page(vaddr)
        }
    }

    /// Tries to look up a user page. Returns `None` if the page is not present.
    pub fn try_lookup_user_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<FrameAddress>, Error> {
        // SAFETY: pml4_paddr is a valid PML4 address.
        unsafe {
            let pml4: super::pml4::Pml4 = super::pml4::Pml4::from_address(self.pml4_paddr);
            pml4.try_lookup_page(vaddr)
        }
    }

    /// Changes access permissions on a user page.
    pub fn ctrl_user_page(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        // SAFETY: pml4_paddr is a valid PML4 address.
        unsafe {
            let pml4: super::pml4::Pml4 = super::pml4::Pml4::from_address(self.pml4_paddr);
            pml4.ctrl_page(vaddr, access)
        }
    }

    ///
    /// # Description
    ///
    /// Maps a kernel page into the address space.
    ///
    /// Delegates to `Pml4::map_page`, which walks the 4-level hierarchy. Newly allocated
    /// page tables are tracked in `kernel_page_tables` (shared via Rc across clones).
    ///
    pub fn map_kernel_page<T: Fn() -> Result<PageTableStorage, Error>>(
        &mut self,
        frame: FrameAddress,
        vaddr: PageAligned<VirtualAddress>,
        page_table_allocator: T,
    ) -> Result<(), Error> {
        // SAFETY: pml4_paddr is a valid PML4 address.
        unsafe {
            let pml4: super::pml4::Pml4 = super::pml4::Pml4::from_address(self.pml4_paddr);
            let alloc: super::pml4::Pml4Alloc =
                pml4.map_page(vaddr, frame, AccessPermission::RDWR, &page_table_allocator)?;
            if let Some(storage) = alloc.pdpt_page {
                self.hw_pages.push_back(storage);
            }
            if let Some(storage) = alloc.pdpt.pd_page {
                self.hw_pages.push_back(storage);
            }
            if let Some(entry) = alloc.pdpt.pd.pt {
                self.kernel_page_tables
                    .push_back(Rc::new(RefCell::new(entry)));
            }
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
        // SAFETY: pml4_paddr is a valid PML4 address.
        unsafe {
            let pml4: super::pml4::Pml4 = super::pml4::Pml4::from_address(self.pml4_paddr);

            if dry_run {
                if pml4.try_lookup_page(vaddr)?.is_none() {
                    let reason: &str = "page table entry not found";
                    error!("{reason}");
                    return Err(Error::new(ErrorCode::NoSuchEntry, reason));
                }
                return Ok(());
            }

            pml4.ctrl_page(vaddr, access)
        }
    }

    /// Installs a kernel page table frame into the page directory at the given PDE index.
    unsafe fn install_kernel_pt(
        pd_addr: PageDirectoryAddress,
        pde_idx: TableIndex,
        frame: FrameAddress,
    ) {
        let table: crate::hal::mem::Table<::arch::mem::paging::PageDirectoryEntry> =
            crate::hal::mem::Table::from_address(pd_addr.into_raw_value());
        let flags: PageTableEntryFlags = PageTableEntryFlags::new(
            PresentFlag::Present,
            ReadWriteFlag::ReadWrite,
            UserSupervisorFlag::User,
            PageWriteThroughFlag::WriteThrough,
            PageCacheDisableFlag::CacheDisabled,
            AccessedFlag::NotAccessed,
            DirtyFlag::NotDirty,
        );
        table.write(
            pde_idx,
            ::arch::mem::paging::PageDirectoryEntry::new(
                PageDirectoryEntryFlags::from(flags),
                frame.into_frame_number(),
            ),
        );
    }

    /// Computes the [`PageTableAligned<VirtualAddress>`] for the 2 MiB region containing `vaddr`.
    fn pt_address_for(
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<PageTableAligned<VirtualAddress>, Error> {
        let aligned: usize = ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT);
        PageTableAligned::from_raw_value(aligned)
    }
}

impl Drop for PageMap {
    fn drop(&mut self) {
        while let Some((_addr, _storage)) = self.user_page_tables.pop_front() {
            // Storage dropped here, freeing the backing page.
        }
        while let Some(entry) = self.kernel_page_tables.pop_front() {
            drop(entry);
        }
    }
}
