// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// TODO: remove this.
#![allow(clippy::type_complexity)]

//==================================================================================================
// Imports
//==================================================================================================
use crate::{
    hal::{
        arch::x86::mem::mmu::{
            self,
            page_directory::PageDirectory,
            page_table::PageTable,
        },
        mem::{
            AccessPermission,
            Address,
            FrameAddress,
            PageAddress,
            PageAligned,
            PageDirectoryAddress,
            PageTableAddress,
            PageTableAligned,
            PhysicalAddress,
            VirtualAddress,
        },
    },
    mm::{
        phys::{
            KernelFrame,
            PhysMemoryManager,
            UserFrame,
        },
        virt::{
            kpage::KernelPage,
            PageDirectoryStorage,
            PageTableStorage,
        },
    },
};
use ::alloc::{
    collections::LinkedList,
    rc::Rc,
};
use ::arch::mem::{
    self,
    paging::{
        PageDirectoryEntry,
        PageTableEntry,
        PteWord,
    },
    PAGE_ALIGNMENT,
    PAGE_TABLE_LENGTH,
    PGTAB_ALIGNMENT,
};
use ::core::{
    cell::RefCell,
    mem::{
        ManuallyDrop,
        MaybeUninit,
    },
    ops::ControlFlow,
};
use ::sys::{
    config,
    error::{
        Error,
        ErrorCode,
    },
};
use ::vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use ::vstd::raw_ptr::PointsTo;

//==================================================================================================
// Constants
//==================================================================================================

// TODO: `USER_BASE` should be aligned to a page boundary.

// TODO: `USER_BASE` should be aligned to a page table boundary.

//==================================================================================================
// Virtual Memory Space
//==================================================================================================

/// A type that represents a virtual memory space.
pub struct Vmem {
    /// Underlying page directory.
    pgdir: PageDirectory<PageDirectoryStorage>,
    /// List of kernel page tables.
    kernel_page_tables: LinkedList<Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>>,
    /// List of kernel pages mapped in the virtual address space.
    /// NOTE: this currently excludes kernel pages that are identity mapped.
    kernel_pages: LinkedList<Rc<RefCell<KernelPage>>>,
    /// List of user page tables.
    user_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    /// Physical address of the per-process hardware 4-level page table (PML4) on x86_64.
    ///
    /// x86_64 hardware uses 4-level paging, but the `pgdir` above is a 2-level (32-bit-style)
    /// structure used for the kernel's logical bookkeeping. On x86_64 the value actually loaded
    /// into `CR3` is this hardware PML4, into which user mappings are mirrored. A value of `0`
    /// denotes the kernel address space (which runs on the VMM-provided boot PML4).
    #[cfg(target_arch = "x86_64")]
    hw_pml4: u64,
    /// Proof ownership for process-private x86_64 hardware paging pages.
    #[cfg(all(target_arch = "x86_64", verus_keep_ghost_body))]
    hw_pages: Tracked<Map<u64, crate::hal::arch::x86::mem::mmu::hwpt::NanvixHwPageToken>>,
    /// Shared proof handle for boot hierarchy and page-table pool authority.
    #[cfg(all(target_arch = "x86_64", verus_keep_ghost_body))]
    hwpt_manager: Tracked<crate::hal::arch::x86::mem::mmu::hwpt::HwptManagerHandle>,
}

impl Vmem {
    /// Initializes a new virtual memory space.
    #[verus_verify(external_body)]
    #[cfg_attr(
        target_arch = "x86_64",
        verus_spec(
            with
                Tracked(hwpt_manager):
                    Tracked<
                        crate::hal::arch::x86::mem::mmu::hwpt::HwptManagerHandle
                    >
        )
    )]
    pub fn new(
        mut kernel_pages: LinkedList<KernelPage>,
        mut kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) -> Result<Self, Error> {
        trace!("kernel_pages.len()={}", kernel_pages.len());

        // Create a clean page directory.
        // SAFETY: this constructor is only used during early single-threaded init;
        // BSS is zero-initialized, so assume_init_mut() is sound for integer arrays.
        proof_decl! {
            let tracked pgdir_slot_permissions:
                super::page_table_allocator::PageTableSlotPermissions;
        }
        let pgdir_entries: &'static mut [PteWord; PAGE_TABLE_LENGTH] = unsafe {
            proof_with! {
                => Tracked(pgdir_slot_permissions)
            };
            super::page_table_allocator::allocate_page_table_slot()
        }
        .map_err(|e| {
            error!("Vmem::new(): page directory allocation failed: {}", e);
            Error::new(ErrorCode::OutOfMemory, "BSS page directory allocation failed")
        })?;
        proof_decl! {
            let ghost pgdir_base_address = pgdir_slot_permissions.base;
            let tracked pgdir_raw_permissions = pgdir_slot_permissions.entries;
        }
        let pgdir_storage: PageDirectoryStorage = PageDirectoryStorage::Bss {
            entries: pgdir_entries,
            #[cfg(verus_keep_ghost_body)]
            base_address: Ghost::new(pgdir_base_address),
        };
        proof_with! {
            Tracked(pgdir_raw_permissions)
        };
        let mut pgdir: PageDirectory<PageDirectoryStorage> = PageDirectory::new(pgdir_storage);

        // Map and store root page tables.
        let mut kpage_tables: LinkedList<
            Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>,
        > = LinkedList::new();
        while let Some((vaddr, page_table)) = kernel_page_tables.pop_front() {
            let page_table_address: FrameAddress = page_table.physical_address()?;
            // FIXME: do not be so open about permissions.
            proof_with! {
                Ghost(&page_table)
            };
            pgdir.map(vaddr, page_table_address, false, AccessPermission::RDWR)?;
            kpage_tables.push_back(Rc::new(RefCell::new((vaddr, page_table))));
        }

        // Register the kernel page directory for lazy identity mapping and set CR3.
        {
            use crate::hal::mem::PageDirectoryAddress;
            use ::arch::cpu::cr3::{
                Cr3Register,
                PageDirectoryBaseAddress as Cr3PageDirectoryBaseAddress,
                PageLevelCacheDisableFlag,
                PageLevelWriteThroughFlag,
            };
            let pd_paddr_raw: usize = pgdir.physical_address()?.into_raw_value();
            let pd_paddr: PageDirectoryAddress =
                PageDirectoryAddress::from_raw_value(pd_paddr_raw)?;
            let kernel_cr3: Cr3Register = Cr3Register {
                page_level_write_through: PageLevelWriteThroughFlag::Disabled,
                page_level_cache_disable: PageLevelCacheDisableFlag::Enabled,
                paging_structure_base_address: Cr3PageDirectoryBaseAddress::new(
                    pd_paddr_raw as u32,
                )
                .ok_or_else(|| {
                    Error::new(
                        ErrorCode::BadAddress,
                        "kernel page directory address is not 4 KB aligned",
                    )
                })?,
            };
            super::identity_map_init(pd_paddr, kernel_cr3)?;
        }

        // Store root pages.
        let mut kpages: LinkedList<Rc<RefCell<KernelPage>>> = LinkedList::new();
        while let Some(entry) = kernel_pages.pop_front() {
            kpages.push_back(Rc::new(RefCell::new(entry)));
        }

        #[cfg(target_arch = "x86_64")]
        proof_decl! {
            let tracked hw_pages:
                Map<
                    u64,
                    crate::hal::arch::x86::mem::mmu::hwpt::NanvixHwPageToken,
                > = Map::tracked_empty();
        }
        let vmem: Self = Self {
            pgdir,
            kernel_page_tables: kpage_tables,
            kernel_pages: kpages,
            user_page_tables: LinkedList::new(),
            // The kernel address space runs on the VMM-provided boot PML4 (see `hw_pml4`).
            #[cfg(target_arch = "x86_64")]
            hw_pml4: 0,
            #[cfg(all(target_arch = "x86_64", verus_keep_ghost_body))]
            hw_pages: Tracked::new(hw_pages),
            #[cfg(all(target_arch = "x86_64", verus_keep_ghost_body))]
            hwpt_manager: Tracked::new(hwpt_manager),
        };
        Ok(vmem)
    }

    /// Clones the target virtual memory space.
    #[verus_verify(external_body)]
    #[verus_spec(
        with
            Tracked(pgdir_raw_permissions):
                Tracked<Map<nat, PointsTo<PteWord>>>,
        requires
            pgdir_raw_permissions.dom().len() == PAGE_TABLE_LENGTH,
            forall|i: nat| pgdir_raw_permissions.dom().contains(i)
                <==> 0 <= i < PAGE_TABLE_LENGTH,
            forall|i: nat| 0 <= i < PAGE_TABLE_LENGTH ==> {
                let permission = #[trigger] pgdir_raw_permissions[i];
                &&& permission.ptr()@.addr as int
                    == pgdir_page.base_address() + i * 4
                &&& permission.is_uninit()
            },
    )]
    pub fn clone(from: &Vmem, pgdir_page: KernelPage) -> Result<Vmem, Error> {
        // Create a clean page directory backed by a kernel page from the pool.
        proof_with! {
            Tracked(pgdir_raw_permissions)
        };
        let mut pgdir: PageDirectory<PageDirectoryStorage> =
            PageDirectory::new(PageDirectoryStorage::KernelPage(pgdir_page));

        // Map and store root page tables.
        let mut kernel_page_tables: LinkedList<
            Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>,
        > = LinkedList::new();
        for entry in from.kernel_page_tables.iter() {
            let page_table_entry = entry.borrow();
            let page_table_address: FrameAddress = page_table_entry.1.physical_address()?;
            // FIXME: do not be so open about permissions.
            proof_with! {
                Ghost(&page_table_entry.1)
            };
            pgdir.map(page_table_entry.0, page_table_address, false, AccessPermission::RDWR)?;
            kernel_page_tables.push_back(entry.clone());
        }

        // Store root pages.
        let mut kernel_pages: LinkedList<Rc<RefCell<KernelPage>>> = LinkedList::new();
        for entry in from.kernel_pages.iter() {
            kernel_pages.push_back(entry.clone());
        }

        // Sync all present kernel identity-mapping PDEs into the new page directory. These
        // PDEs are pre-allocated at boot and point to BSS page tables shared across all
        // address spaces; copying them here ensures the new process can access kernel memory.
        {
            let target_pd_paddr: PageDirectoryAddress =
                PageDirectoryAddress::from_raw_value(pgdir.physical_address()?.into_raw_value())?;
            super::sync_kernel_pdes(target_pd_paddr)?;
        }

        // On x86_64, allocate a fresh per-process hardware PML4. It shares the kernel's low-memory
        // mapping (and maps the LAPIC) and starts with an empty user space; user mappings are
        // mirrored into it as they are added (see `hw_map_user`/`hw_unmap_user`).
        #[cfg(target_arch = "x86_64")]
        proof_decl! {
            let tracked hwpt_manager:
                crate::hal::arch::x86::mem::mmu::hwpt::HwptManagerHandle =
                    from.hwpt_manager.clone();
            let tracked mut hw_pages:
                Map<
                    u64,
                    crate::hal::arch::x86::mem::mmu::hwpt::NanvixHwPageToken,
                > = Map::tracked_empty();
        }
        #[cfg(target_arch = "x86_64")]
        let hw_pml4: u64 = unsafe {
            proof_with! {
                Tracked(&hwpt_manager),
                Tracked(&mut hw_pages)
            };
            crate::hal::arch::x86::mem::mmu::hwpt::create_user_pml4()
        };

        let vmem: Self = Self {
            pgdir,
            kernel_page_tables,
            kernel_pages,
            user_page_tables: LinkedList::new(),
            #[cfg(target_arch = "x86_64")]
            hw_pml4,
            #[cfg(all(target_arch = "x86_64", verus_keep_ghost_body))]
            hw_pages: Tracked::new(hw_pages),
            #[cfg(all(target_arch = "x86_64", verus_keep_ghost_body))]
            hwpt_manager: Tracked::new(hwpt_manager),
        };
        Ok(vmem)
    }

    #[cfg_attr(not(target_arch = "x86_64"), verus_verify(external_body))]
    pub fn load(&self) -> Result<(), Error> {
        let pgdir_addr: FrameAddress = self.pgdir.physical_address()?;
        unsafe {
            #[cfg(not(target_arch = "x86_64"))]
            proof_with! {
                Ghost(&self.pgdir)
            };
            mmu::load_page_directory(pgdir_addr.into_raw_value())
        };
        Ok(())
    }

    /// Returns a reference to the underlying page directory.
    pub fn pgdir(&self) -> &PageDirectory<PageDirectoryStorage> {
        &self.pgdir
    }

    ///
    /// # Description
    ///
    /// Returns the physical address to load into `CR3` for this address space.
    ///
    /// On x86_64 this is the per-process hardware PML4 (4-level paging root). On other
    /// architectures it is the 2-level page directory, which the hardware uses directly.
    ///
    pub fn cr3_value(&self) -> Result<usize, Error> {
        #[cfg(target_arch = "x86_64")]
        {
            Ok(self.hw_pml4 as usize)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            Ok(self.pgdir.physical_address()?.into_raw_value())
        }
    }

    /// Mirrors a user-page mapping into the per-process hardware page table (x86_64). No-op on
    /// other architectures and for the kernel address space (`hw_pml4 == 0`).
    #[inline]
    fn hw_map_user(&mut self, vaddr: usize, paddr: usize, writable: bool) {
        #[cfg(target_arch = "x86_64")]
        if self.hw_pml4 != 0 {
            unsafe {
                proof_with! {
                    Tracked(&self.hwpt_manager),
                    Tracked(&mut self.hw_pages)
                };
                crate::hal::arch::x86::mem::mmu::hwpt::map_user(
                    self.hw_pml4,
                    vaddr,
                    paddr,
                    writable,
                );
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = (vaddr, paddr, writable);
        }
    }

    /// Mirrors a user-page unmapping into the per-process hardware page table (x86_64). No-op on
    /// other architectures and for the kernel address space.
    #[inline]
    fn hw_unmap_user(&mut self, vaddr: usize) {
        #[cfg(target_arch = "x86_64")]
        if self.hw_pml4 != 0 {
            unsafe {
                proof_with! {
                    Tracked(&mut self.hw_pages)
                };
                crate::hal::arch::x86::mem::mmu::hwpt::unmap_user(self.hw_pml4, vaddr);
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = vaddr;
        }
    }

    /// Mirrors a user-page permission change (copy-on-write) into the per-process hardware page
    /// table (x86_64). No-op on other architectures and for the kernel address space.
    #[inline]
    fn hw_protect_user(&mut self, vaddr: usize, writable: bool) {
        #[cfg(target_arch = "x86_64")]
        if self.hw_pml4 != 0 {
            unsafe {
                proof_with! {
                    Tracked(&mut self.hw_pages)
                };
                crate::hal::arch::x86::mem::mmu::hwpt::protect_user(self.hw_pml4, vaddr, writable);
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = (vaddr, writable);
        }
    }

    /// Mirrors a kernel-space MMIO mapping (e.g. a RAMFS window exposed to user space by `kctrl`)
    /// into the shared boot-PML4 low-memory page directory on x86_64. Because every per-process
    /// PML4 shares that page directory, the mapping becomes visible in all address spaces, matching
    /// the shared kernel page tables used on 32-bit targets. No-op on other architectures.
    #[inline]
    fn hw_map_kernel_mmio(&mut self, vaddr: usize, paddr: usize, writable: bool) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            proof_with! {
                Tracked(&self.hwpt_manager)
            };
            crate::hal::arch::x86::mem::mmu::hwpt::map_kernel_mmio(vaddr, paddr, writable);
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = (vaddr, paddr, writable);
        }
    }

    ///
    /// # Description
    ///
    /// Maps a kernel page to the target virtual address space.
    ///
    /// # Parameters
    /// - `kpage`: Kernel page to be mapped.
    /// - `vaddr`: Virtual address of the target page.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Upon failure, an error code is returned instead.
    ///
    pub fn map_kpage(
        &mut self,
        kpage: KernelPage,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        let pt_vaddr: PageTableAddress = PageTableAddress::new(PageTableAligned::from_raw_value(
            ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
        )?);

        // Get the corresponding page directory entry.
        let pde: PageDirectoryEntry = match self.pgdir.read_pde(pt_vaddr) {
            Some(pde) => pde,
            None => {
                let reason: &str = "failed to read page directory entry";
                error!("{reason}");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page table does not exist.
        if !pde.is_present() {
            let page_table: PageTable<PageTableStorage> = Self::allocate_kernel_page_table()?;

            // FIXME: do not be so open about permissions.
            proof_with! {
                Ghost(&page_table)
            };
            self.pgdir.map(
                pt_vaddr,
                page_table.physical_address()?,
                false,
                AccessPermission::RDWR,
            )?;

            //===================================================================
            // NOTE: if we fail beyond this point we should unmap the page table.
            //===================================================================

            // Add page table to the list of kernel page tables.
            self.kernel_page_tables
                .push_back(Rc::new(RefCell::new((pt_vaddr, page_table))));
        };

        // Get corresponding page table.
        for entry in self.kernel_page_tables.iter_mut() {
            if entry.borrow().0.into_raw_value() == pt_vaddr.into_raw_value() {
                // Map the page to the target virtual address space.
                // FIXME: do not be so open about permissions and caching.
                entry.borrow_mut().1.map(
                    PageAddress::new(vaddr),
                    kpage.frame_address(),
                    true,
                    true,
                    false,
                    AccessPermission::RDWR,
                )?;

                // Add the kernel page to the list of kernel pages.
                self.kernel_pages.push_back(Rc::new(RefCell::new(kpage)));

                // Reload page directory to force a TLB flush.
                self.load()?;

                return Ok(());
            }
        }

        let reason: &str = "page table not found";
        error!("{reason}");
        Err(Error::new(ErrorCode::NoSuchEntry, reason))
    }

    ///
    /// # Description
    ///
    /// Allocate a page table for mapping kernel memory.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(page_table)` is returned. Upon failure, an error is returned.
    ///
    fn allocate_kernel_page_table() -> Result<PageTable<PageTableStorage>, Error> {
        proof_decl! {
            let tracked raw_permissions:
                Map<nat, PointsTo<PteWord>>;
        }
        proof_with! {
            => Tracked(raw_permissions)
        };
        let mut kframe: KernelFrame = KernelFrame::allocate_page_table()?;
        kframe.clear()?;
        let kpage: KernelPage = KernelPage::new(kframe);
        let pgtable_storage: PageTableStorage = PageTableStorage::KernelPage(kpage);
        proof_with! {
            Tracked(raw_permissions)
        };
        let page_table: PageTable<PageTableStorage> = PageTable::new(pgtable_storage);
        Ok(page_table)
    }

    ///
    /// # Description
    ///
    /// Allocate a page table for mapping user memory.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(page_table)` is returned. Upon failure, an error is returned.
    ///
    fn allocate_user_page_table() -> Result<PageTable<PageTableStorage>, Error> {
        // SAFETY: the kernel is single-threaded and runs with interrupts disabled; no
        // concurrent or re-entrant access to the physical memory manager is possible.
        proof_decl! {
            let tracked raw_permissions:
                Map<nat, PointsTo<PteWord>>;
        }
        proof_with! {
            => Tracked(raw_permissions)
        };
        let mut kframe: KernelFrame = KernelFrame::allocate_page_table()?;
        kframe.clear()?;
        let kpage: KernelPage = KernelPage::new(kframe);
        let pgtable_storage: PageTableStorage = PageTableStorage::KernelPage(kpage);
        proof_with! {
            Tracked(raw_permissions)
        };
        let page_table: PageTable<PageTableStorage> = PageTable::new(pgtable_storage);
        Ok(page_table)
    }

    /// Maps a page to the target virtual address space.
    pub fn map(
        &mut self,
        uframe: UserFrame,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        // Check if the provided address lies outside the user space.
        if !Self::is_user_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in user space";
            error!("{reason} (uframe={uframe:?}, vaddr={vaddr:?}, access={access:?})",);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Get corresponding page table.
        let page_table: &mut PageTable<PageTableStorage> = {
            let vaddr: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
                ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
            )?;
            let pgtable_vaddr: PageTableAddress = PageTableAddress::new(vaddr);
            // Get the corresponding page directory entry.
            let pde: PageDirectoryEntry = match self.pgdir.read_pde(pgtable_vaddr) {
                Some(pde) => pde,
                None => {
                    let reason: &str = "failed to read page directory entry";
                    error!("{reason}");
                    return Err(Error::new(ErrorCode::TryAgain, reason));
                },
            };

            // Get corresponding page table.
            // Check if corresponding page table does not exist.
            if !pde.is_present() {
                let page_table: PageTable<PageTableStorage> = Self::allocate_user_page_table()?;

                let page_table_address: FrameAddress = page_table.physical_address()?;
                // FIXME: do not be so open about permissions.
                proof_with! {
                    Ghost(&page_table)
                };
                self.pgdir
                    .map(pgtable_vaddr, page_table_address, false, AccessPermission::RDWR)?;

                //===================================================================
                // NOTE: if we fail beyond this point we should unmap the page table.
                //===================================================================

                self.user_page_tables
                    .push_front((pgtable_vaddr, page_table));
            };

            self.lookup_user_page_table(pgtable_vaddr)?
        };

        // Map the page to the target virtual address space.
        page_table.map(PageAddress::new(vaddr), uframe.address(), false, false, true, access)?;

        // Mirror the mapping into the per-process hardware page table (x86_64).
        self.hw_map_user(
            vaddr.into_raw_value(),
            uframe.address().into_raw_value(),
            access.is_writable(),
        );

        //=============================================================
        // NOTE: if we fail beyond this point we should unmap the page.
        //=============================================================

        // Frame is now owned by the page table; prevent Drop from freeing it.
        uframe.leak();

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Checks whether a user page is currently mapped at the given virtual address.
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Virtual address of the page to check.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the page is mapped, `Ok(false)` if it is not, or `Err(_)` on
    /// unexpected failures.
    ///
    pub fn is_user_page_mapped(&self, vaddr: PageAligned<VirtualAddress>) -> Result<bool, Error> {
        // Check if the provided address lies outside user space.
        if !Self::is_user_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in user space";
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }
        Ok(self.try_find_user_frame(vaddr)?.is_some())
    }

    /// Asserts whether an address lies in the user space.
    pub fn is_user_addr(virt_addr: VirtualAddress) -> bool {
        virt_addr >= config::memory_layout::USER_BASE && virt_addr < config::memory_layout::USER_END
    }

    ///
    /// # Description
    ///
    /// Asserts whether a memory region lies entirely in user space.
    ///
    /// # Parameters
    ///
    /// - `start`: Starting virtual address of the region.
    /// - `size`: Size of the region in bytes.
    ///
    /// # Returns
    ///
    /// Returns `true` if the entire region lies in user space, `false` otherwise.
    ///
    pub fn is_user_region(start: VirtualAddress, size: usize) -> bool {
        // Reject zero-length regions.
        if size == 0 {
            return false;
        }

        // Check if the start and end addresses of the region lie in user space.
        match start.checked_add(size - 1) {
            Some(end) => Self::is_user_addr(start) && Self::is_user_addr(end),
            None => false,
        }
    }

    ///
    /// # Description
    ///
    /// Asserts whether a memory region lies entirely in user space and every page that backs it is
    /// mapped and writable, either directly or through a copy-on-write mapping that a kernel write
    /// would resolve to a private, writable copy.
    ///
    /// This is the check a kernel-side writer performs before writing to a user region whose
    /// address is under user control (for example, the signal-frame builder, which places a frame
    /// at the interrupted thread's user stack pointer). It lets an unmapped or read-only page fail
    /// gracefully up front instead of faulting the physical-alias write path that
    /// [`Self::copy_to_user_unaligned`] takes.
    ///
    /// # Parameters
    ///
    /// - `start`: Starting virtual address of the region.
    /// - `size`: Size of the region in bytes.
    ///
    /// # Returns
    ///
    /// `true` if the entire region lies in user space and is mapped and writable, `false`
    /// otherwise.
    ///
    pub fn is_user_region_writable(&self, start: VirtualAddress, size: usize) -> bool {
        // The region must first lie entirely in user space; this also rejects empty regions.
        if !Self::is_user_region(start, size) {
            return false;
        }

        let first_page: usize = ::sys::mm::align_down(start.into_raw_value(), PAGE_ALIGNMENT);
        // `is_user_region` already rejected zero-length and overflowing regions, so the subtraction
        // and the inclusive end are well defined.
        let last_page: usize = match start.into_raw_value().checked_add(size - 1) {
            Some(end_inclusive) => ::sys::mm::align_down(end_inclusive, PAGE_ALIGNMENT),
            None => return false,
        };

        let mut page: usize = first_page;
        loop {
            let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(page) {
                Ok(vaddr) => vaddr,
                Err(_) => return false,
            };
            match self.try_find_user_pte(vaddr) {
                // Mapped and writable, either directly or via a copy-on-write mapping that the
                // write path will resolve to a private, writable copy.
                Ok(Some(pte))
                    if pte.is_present() && (pte.flags().is_writable() || pte.is_cow()) => {},
                // Unmapped, read-only, or a lookup failure: not safe to write.
                _ => return false,
            }

            if page == last_page {
                break;
            }
            page = match page.checked_add(mem::PAGE_SIZE) {
                Some(next) => next,
                None => return false,
            };
        }

        true
    }

    /// Asserts whether an address lies in the kernel space.
    fn is_kernel_addr(virt_addr: VirtualAddress) -> bool {
        !Self::is_user_addr(virt_addr)
    }

    ///
    /// # Description
    ///
    /// Asserts whether a memory region lies entirely in kernel space.
    ///
    /// # Parameters
    ///
    /// - `start`: Starting virtual address of the region.
    /// - `size`: Size of the region in bytes.
    ///
    /// # Returns
    ///
    /// Returns `true` if the entire region lies in kernel space, `false` otherwise.
    ///
    fn is_kernel_region(start: VirtualAddress, size: usize) -> bool {
        // Reject zero-length regions.
        if size == 0 {
            return false;
        }

        // Check if the start and end addresses of the region lie in kernel space.
        match start.checked_add(size - 1) {
            Some(end) => Self::is_kernel_addr(start) && Self::is_kernel_addr(end),
            None => false,
        }
    }

    ///
    /// # Description
    ///
    /// Asserts whether a memory region lies within physical memory.
    ///
    /// # Parameters
    ///
    /// - `start`: Starting physical address of the region.
    /// - `size`: Size of the region in bytes.
    ///
    /// # Returns
    ///
    /// Returns `true` if the entire region lies within physical memory, `false` otherwise.
    ///
    pub fn is_physical_region(start: usize, size: usize) -> bool {
        crate::hal::platform::is_valid_physical_region(start, size)
    }

    ///
    /// # Description
    ///
    /// Looks up a user page table by its virtual base address. The first lookup in a given region
    /// is O(n) in the number of user page tables, but moves the found entry to the front of the
    /// list so that subsequent lookups for the same 4 MB region complete in O(1). This exploits
    /// spatial locality: consecutive pages within the same region share the same page table.
    ///
    /// # Preconditions
    ///
    /// The caller must ensure that the page table identified by `pt_vaddr` has already been mapped
    /// in the page directory (i.e., the corresponding PDE is present).
    ///
    /// # Parameters
    ///
    /// - `pt_vaddr`: Virtual base address of the page table to look up.
    ///
    /// # Returns
    ///
    /// Upon success, a mutable reference to the page table is returned. Upon failure, an error
    /// code is returned instead.
    ///
    fn lookup_user_page_table(
        &mut self,
        pt_vaddr: PageTableAddress,
    ) -> Result<&mut PageTable<PageTableStorage>, Error> {
        // Fast path: check if the entry is already at the front (O(1)).
        if let Some((addr, _pt)) = self.user_page_tables.front() {
            if addr.into_raw_value() == pt_vaddr.into_raw_value() {
                return Ok(&mut self.user_page_tables.front_mut().expect("front exists").1);
            }
        }

        // Slow path: single-traversal search using a cursor, then move to front.
        let removed_entry = {
            let mut cursor = self.user_page_tables.cursor_front_mut();
            loop {
                match cursor.current() {
                    Some((addr, _pt)) => {
                        if addr.into_raw_value() == pt_vaddr.into_raw_value() {
                            break cursor.remove_current();
                        }
                    },
                    None => break None,
                }
                cursor.move_next();
            }
        };

        if let Some(entry) = removed_entry {
            self.user_page_tables.push_front(entry);
            return Ok(&mut self
                .user_page_tables
                .front_mut()
                .expect("entry was just pushed to front")
                .1);
        }

        let reason: &str = "page table not found";
        error!("{reason}");
        Err(Error::new(ErrorCode::NoSuchEntry, reason))
    }

    fn lookup_kernel_page_table(
        &mut self,
        pde: &PageDirectoryEntry,
    ) -> Result<Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>, Error> {
        // Check if corresponding page table does not exist.
        if !pde.is_present() {
            let reason: &str = "page table not present";
            error!("{reason:?} (pde={pde:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        // Get corresponding page table.
        let pgtab_addr: FrameAddress = FrameAddress::from_frame_number(pde.frame_number())?;

        // Find corresponding page table.
        let mut page_table: Option<Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>> =
            None;
        for pt in self.kernel_page_tables.iter_mut() {
            if pt.borrow().1.physical_address()? == pgtab_addr {
                page_table = Some(pt.clone());
                break;
            }
        }

        match page_table {
            Some(entry) => Ok(entry),
            None => {
                let reason: &str = "page table not found";
                error!("{reason}");
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Finds a user frame in the target virtual memory space.
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Virtual address of the target page.
    ///
    /// # Returns
    ///
    /// Upon success, a reference to the target user page is returned. Upon failure, an error code is
    /// returned instead.
    ///
    fn find_user_frame(&self, vaddr: PageAligned<VirtualAddress>) -> Result<FrameAddress, Error> {
        let page_addr: PageAddress = PageAddress::new(vaddr);
        let pgtab_addr: PageTableAddress = PageTableAddress::new(PageTableAligned::from_raw_value(
            ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
        )?);

        // Look for the corresponding page table.
        for (lookup_pgtable_addr, page_table) in self.user_page_tables.iter() {
            // Found.
            if lookup_pgtable_addr == &pgtab_addr {
                // Look for the corresponding page.
                return page_table.lookup(page_addr);
            }
        }

        let reason: &str = "page not found";
        error!("{reason} (vaddr={vaddr:?})");
        Err(Error::new(ErrorCode::NoSuchEntry, reason))
    }

    ///
    /// # Description
    ///
    /// Attempts to find a user frame in the target virtual memory space.
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Virtual address of the target page.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(addr))` if the page is present.
    /// - `Ok(None)` if the page table or page is not present.
    /// - `Err(_)` on unexpected failures.
    ///
    fn try_find_user_frame(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<FrameAddress>, Error> {
        let page_addr: PageAddress = PageAddress::new(vaddr);
        let pgtab_addr: PageTableAddress = PageTableAddress::new(PageTableAligned::from_raw_value(
            ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
        )?);

        for (lookup_pgtable_addr, page_table) in self.user_page_tables.iter() {
            if lookup_pgtable_addr == &pgtab_addr {
                if page_table.is_page_present(page_addr)? {
                    return Ok(Some(page_table.lookup(page_addr)?));
                } else {
                    return Ok(None);
                }
            }
        }

        Ok(None)
    }

    ///
    /// # Description
    ///
    /// Attempts to find the page-table entry that backs the user page at `vaddr`.
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Virtual address of the target page.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(pte))` if the page is present, where `pte` is a decoded copy of the
    ///   page-table entry that backs the mapping.
    /// - `Ok(None)` if the page table or page is not present.
    /// - `Err(_)` on unexpected failures.
    ///
    pub(crate) fn try_find_user_pte(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<PageTableEntry>, Error> {
        let page_addr: PageAddress = PageAddress::new(vaddr);
        let pgtab_addr: PageTableAddress = PageTableAddress::new(PageTableAligned::from_raw_value(
            ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
        )?);

        for (lookup_pgtable_addr, page_table) in self.user_page_tables.iter() {
            if lookup_pgtable_addr == &pgtab_addr {
                return Ok(page_table.read_pte_at(page_addr));
            }
        }

        Ok(None)
    }

    ///
    /// # Description
    ///
    /// Invokes `f` once for each present user-space page in the target virtual memory
    /// space, in the order they appear in the internal user page-table list.
    ///
    /// # Parameters
    ///
    /// - `f`: Callback invoked with `(vaddr, pte)` for every present user mapping. The
    ///   virtual address is page-aligned and lies in user space; `pte` is a decoded copy
    ///   of the page-table entry that backs the mapping. Returning an error from `f`
    ///   short-circuits the iteration and propagates the error to the caller.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, the first error returned by `f`
    /// is propagated.
    ///
    pub fn for_each_user_mapping<F>(&self, mut f: F) -> Result<(), Error>
    where
        F: FnMut(PageAligned<VirtualAddress>, PageTableEntry) -> Result<(), Error>,
    {
        self.try_for_each_user_mapping(|vaddr, pte| {
            f(vaddr, pte)?;
            Ok(ControlFlow::Continue(()))
        })
    }

    ///
    /// # Description
    ///
    /// Like [`Self::for_each_user_mapping`], but the callback may stop the walk early by
    /// returning [`ControlFlow::Break`]. Bounded consumers that snapshot a fixed-size batch of
    /// mappings per pass use this to stop as soon as their buffer is full, instead of paying for
    /// a full traversal of every remaining mapping on each pass.
    ///
    /// # Parameters
    ///
    /// - `f`: Callback invoked with `(vaddr, pte)` for every present user mapping, in the order
    ///   they appear in the internal user page-table list. Returning `Ok(ControlFlow::Break(()))`
    ///   stops the iteration; returning an error short-circuits and propagates it to the caller.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned, whether the walk ran to completion or was stopped
    /// early. Upon failure, the first error returned by `f` is propagated.
    ///
    pub(crate) fn try_for_each_user_mapping<F>(&self, mut f: F) -> Result<(), Error>
    where
        F: FnMut(PageAligned<VirtualAddress>, PageTableEntry) -> Result<ControlFlow<()>, Error>,
    {
        for (pgtab_addr, page_table) in self.user_page_tables.iter() {
            let base: usize = pgtab_addr.into_raw_value();
            for (pte_idx, pte) in page_table.iter_present_ptes() {
                let raw_vaddr: usize = base
                    .checked_add(
                        pte_idx.checked_mul(mem::PAGE_SIZE).ok_or_else(|| {
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

    ///
    /// # Description
    ///
    /// Unmaps every present user-space page from this address space, freeing the underlying
    /// physical frames and reclaiming any now-empty user page tables.
    ///
    /// This is used by `execv()` to reclaim the frames backing the previous image before its
    /// address space is dropped. It must not be called on the address space that is currently
    /// active on the CPU; the caller defers reclamation until after the context switch into the
    /// new image has loaded a different page directory.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    /// # Notes
    ///
    /// Errors from this function indicate an internal bug and may leave the address space in a
    /// potentially inconsistent state. Callers typically log the error and rely on the debug
    /// assertion in `Vmem::drop` (debug/test builds) to catch leaked user mappings.
    pub fn clear_user_space(&mut self) -> Result<(), Error> {
        // Number of mappings unmapped per pass. This bounds an on-stack scratch buffer so the
        // routine performs no heap allocation: the kernel heap is a small slab allocator that
        // cannot satisfy the large buffer a full (potentially multi-megabyte) address space would
        // otherwise require. `unmap` needs `&mut self` whereas `try_for_each_user_mapping` borrows
        // `&self`, so each pass first snapshots up to `CHUNK` addresses, then unmaps them.
        const CHUNK: usize = 32;

        loop {
            let mut buf: [MaybeUninit<PageAligned<VirtualAddress>>; CHUNK] =
                [const { MaybeUninit::uninit() }; CHUNK];
            let mut count: usize = 0;
            // Break as soon as the batch is full. Without the early break each pass would
            // re-traverse every remaining mapping, making a full teardown quadratic in the number
            // of mapped pages. `unmap` removes emptied page tables from the front, so restarting
            // the walk each pass still advances and the whole teardown stays linear.
            self.try_for_each_user_mapping(|vaddr, _pte| {
                buf[count].write(vaddr);
                count += 1;
                if count == CHUNK {
                    Ok(ControlFlow::Break(()))
                } else {
                    Ok(ControlFlow::Continue(()))
                }
            })?;

            if count == 0 {
                return Ok(());
            }

            // Unmap each collected page. The `UserFrame` returned by `unmap` is dropped
            // immediately, which frees the underlying physical frame; empty page tables are
            // reclaimed by `unmap` itself.
            for slot in buf.iter().take(count) {
                // SAFETY: indices `< count` were initialized by the scan above.
                let vaddr: PageAligned<VirtualAddress> = unsafe { slot.assume_init_read() };
                let _uframe: Option<UserFrame> = self.unmap(vaddr)?;
                // User frame is dropped here, which frees the underlying physical frame.
            }

            // Fewer than a full chunk means the scan saw every remaining mapping this pass, so all
            // user pages have now been unmapped.
            if count < CHUNK {
                return Ok(());
            }
        }
    }

    ///
    /// # Description
    ///
    /// Marks the user page at `vaddr` as copy-on-write: clears the writable bit
    /// and sets the AVL copy-on-write bit on the underlying page-table entry.
    ///
    /// The page must be currently mapped and present. This is intended to be used
    /// when sharing a user page between two address spaces (e.g. during fork).
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Virtual address of the user page to mark.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    pub fn mark_user_page_cow(&mut self, vaddr: PageAligned<VirtualAddress>) -> Result<(), Error> {
        if !Self::is_user_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in user space";
            error!("{reason}");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        let pgtab_aligned: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
            ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
        )?;
        let pgtable_vaddr: PageTableAddress = PageTableAddress::new(pgtab_aligned);
        let page_table: &mut PageTable<PageTableStorage> =
            self.lookup_user_page_table(pgtable_vaddr)?;
        page_table.mark_cow(PageAddress::new(vaddr))?;

        // Mirror the write-protect into the per-process hardware page table (x86_64).
        self.hw_protect_user(vaddr.into_raw_value(), false);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Inverse of [`Self::mark_user_page_cow`]: clears the copy-on-write mark on the user
    /// page at `vaddr`, restoring its writable bit and clearing the AVL copy-on-write bit.
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Virtual address of the user page to be unmarked.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    pub fn unmark_user_page_cow(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        if !Self::is_user_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in user space";
            error!("{reason}");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        let pgtab_aligned: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
            ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
        )?;
        let pgtable_vaddr: PageTableAddress = PageTableAddress::new(pgtab_aligned);
        let page_table: &mut PageTable<PageTableStorage> =
            self.lookup_user_page_table(pgtable_vaddr)?;
        page_table.unmark_cow(PageAddress::new(vaddr))?;

        // Mirror the writable restore into the per-process hardware page table (x86_64).
        self.hw_protect_user(vaddr.into_raw_value(), true);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Resolves a copy-on-write fault on the user page at `vaddr` by repointing
    /// its page-table entry at `new_frame`, clearing the AVL copy-on-write bit,
    /// and restoring the writable bit.
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Virtual address of the user page being resolved.
    /// - `new_frame`: Physical frame to install in the PTE.
    ///
    /// # Returns
    ///
    /// Upon success, the previous frame address (the shared frame the PTE pointed
    /// at) is returned. The caller is responsible for releasing that reference.
    /// Upon failure, an error is returned instead.
    ///
    fn replace_user_page_cow_frame(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        new_frame: FrameAddress,
    ) -> Result<FrameAddress, Error> {
        if !Self::is_user_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in user space";
            error!("{reason}");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        let pgtab_aligned: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
            ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
        )?;
        let pgtable_vaddr: PageTableAddress = PageTableAddress::new(pgtab_aligned);
        let page_table: &mut PageTable<PageTableStorage> =
            self.lookup_user_page_table(pgtable_vaddr)?;
        let old_frame: FrameAddress =
            page_table.replace_cow_frame(PageAddress::new(vaddr), new_frame)?;

        // Mirror the new (private, writable) frame into the per-process hardware page table
        // (x86_64). The page becomes writable again now that it is no longer shared.
        self.hw_map_user(vaddr.into_raw_value(), new_frame.into_raw_value(), true);
        Ok(old_frame)
    }

    ///
    /// # Description
    ///
    /// Resolves a copy-on-write mapping at `vaddr`, if any. Allocates a private user frame,
    /// copies the shared frame's contents into it, repoints the PTE at the new frame, and
    /// drops the reference on the previously-shared frame.
    ///
    /// This is the building block used by both the page-fault handler (lazy resolution on a
    /// user-mode write) and the kernel-side write paths (eager resolution before the kernel
    /// writes to a user page via its physical alias, which would otherwise silently mutate
    /// the shared frame and bypass the copy-on-write contract).
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Page-aligned user virtual address to resolve.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if a copy-on-write mapping was found at `vaddr` and resolved.
    /// - `Ok(false)` if `vaddr` is not mapped or the PTE is not marked copy-on-write.
    /// - `Err(_)` if the resolution failed (e.g. out of frames).
    ///
    pub fn resolve_cow_at(&mut self, vaddr: PageAligned<VirtualAddress>) -> Result<bool, Error> {
        if !Self::is_user_addr(vaddr.into_inner()) {
            return Ok(false);
        }

        let pte: PageTableEntry = match self.try_find_user_pte(vaddr)? {
            Some(pte) => pte,
            None => return Ok(false),
        };
        if !pte.is_cow() {
            return Ok(false);
        }

        // Fast path: if this address space holds the last reference to the shared frame,
        // we can simply clear the copy-on-write mark in place — no allocation, no copy,
        // no free. This happens when every other sharer has already resolved its own CoW
        // mapping for this frame. Safe because the kernel is single-threaded and runs
        // with interrupts disabled, so the refcount cannot change under us between the
        // query and the unmark.
        let src_frame: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;
        // Wrap in `ManuallyDrop` so `refcount()` (which borrows `&self`) does not free
        // the frame when the temporary goes out of scope.
        let probe: ManuallyDrop<UserFrame> = ManuallyDrop::new(UserFrame::new(src_frame));
        if probe.refcount()? == 1 {
            self.unmark_user_page_cow(vaddr)?;
            return Ok(true);
        }

        // Allocate a fresh user frame via the single-frame helper; this keeps the kernel
        // watermark check on this path without paying the cost of an intermediate `Vec`.
        // SAFETY: the kernel is single-threaded and runs with interrupts disabled; no
        // concurrent or re-entrant access to the physical memory manager is possible.
        let new_frame: UserFrame = unsafe { PhysMemoryManager::get_mut() }.alloc_user_frame()?;

        let src_paddr: usize = pte.frame_address();
        let dst_paddr: usize = new_frame.address().into_raw_value();
        super::memcpy(dst_paddr as *mut u8, src_paddr as *const u8, mem::PAGE_SIZE)?;

        // Repoint the PTE at the new frame; the previous frame address is returned so we
        // can drop the shared reference.
        let new_frame_addr: FrameAddress = new_frame.address();
        let old_frame: FrameAddress = self.replace_user_page_cow_frame(vaddr, new_frame_addr)?;

        // The new frame is now owned by the page table; suppress its Drop.
        let _ = new_frame.leak();

        // Drop the shared reference: this decrements the per-frame refcount, freeing the
        // frame only when the last sharer releases it.
        drop(UserFrame::new(old_frame));

        Ok(true)
    }

    ///
    /// # Description
    ///
    /// Eagerly resolves all copy-on-write mappings overlapping the byte range `[addr, addr + size)`
    /// in user space. Pages outside user space or not marked copy-on-write are left untouched.
    ///
    /// This must be called by kernel-side write paths (e.g. `copy_to_user`) before they write
    /// to user memory via its physical alias, so that the write does not silently mutate a
    /// frame that is still shared with another address space.
    ///
    /// # Parameters
    ///
    /// - `addr`: Start of the byte range (need not be page-aligned).
    /// - `size`: Length of the byte range, in bytes. A zero-length range is a no-op.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    pub fn resolve_cow_for_region(
        &mut self,
        addr: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        if size == 0 {
            return Ok(());
        }

        let start: usize = ::sys::mm::align_down(addr.into_raw_value(), PAGE_ALIGNMENT);
        let end_inclusive: usize =
            addr.into_raw_value().checked_add(size - 1).ok_or_else(|| {
                let reason: &str = "resolve_cow_for_region: overflow";
                error!("{reason} (addr={addr:?}, size={size:?})");
                Error::new(ErrorCode::BadAddress, reason)
            })?;
        let last_page: usize = ::sys::mm::align_down(end_inclusive, PAGE_ALIGNMENT);

        let mut page: usize = start;
        loop {
            let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(page)?;
            // Only resolve pages that lie in user space; the routine is safe to call on
            // ranges that straddle user/kernel boundaries because non-user pages return
            // `Ok(false)`. Callers that need to enforce that the whole range is in user
            // space do so themselves (e.g. `copy_to_user_unaligned`).
            self.resolve_cow_at(vaddr)?;

            if page == last_page {
                break;
            }
            page = page.checked_add(mem::PAGE_SIZE).ok_or_else(|| {
                let reason: &str = "resolve_cow_for_region: page overflow";
                error!("{reason} (page={page:?})");
                Error::new(ErrorCode::BadAddress, reason)
            })?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Translates a user-space virtual address to a guest physical address by walking the page
    /// tables. The returned physical address includes the intra-page offset from the original
    /// virtual address.
    ///
    /// # Parameters
    ///
    /// - `vaddr`: User-space virtual address to translate.
    ///
    /// # Returns
    ///
    /// Upon success, the guest physical address corresponding to `vaddr` is returned. Upon
    /// failure, an error is returned instead.
    ///
    #[cfg(feature = "stdio")]
    pub fn user_vaddr_to_paddr(&self, vaddr: VirtualAddress) -> Result<usize, Error> {
        let page_aligned: PageAligned<VirtualAddress> =
            PageAligned::from_address(vaddr.align_down(PAGE_ALIGNMENT))?;
        let offset: usize = vaddr.into_raw_value() - page_aligned.into_raw_value();
        let frame: FrameAddress = self.find_user_frame(page_aligned)?;
        Ok(frame.into_raw_value() + offset)
    }

    ///
    /// # Description
    ///
    /// Copies data from user space to kernel space. The source and destination addresses do not
    /// have to be aligned, but the source address range must lie in user space, and the destination
    /// address range must lie in kernel space.
    ///
    /// # Parameters
    ///
    /// - `dst`: Destination address in kernel space.
    /// - `src`: Source address in user space.
    /// - `size`: Number of bytes to copy.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this function returns empty. Upon failure, this function returns
    /// an error that indicates the reason for the failure.
    ///
    /// # Errors
    ///
    /// This function fails with the following error codes:
    /// - [`ErrorCode::InvalidArgument`]: The size of the copy is zero.
    /// - [`ErrorCode::BadAddress`]: The source memory region does not lie in user space.
    /// - [`ErrorCode::BadAddress`]: The destination memory region does not lie in kernel space.
    ///
    pub fn copy_from_user_unaligned(
        &self,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        // Check if size is invalid.
        if size == 0 {
            let reason: &str = "zero-length copy";
            error!(
                "copy_from_user_unaligned(): {reason} (dst={dst:?}, src={src:?}, size={size:?})"
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if the source memory region lies entirely in user space.
        // NOTE: This check is sufficient because, by design, the kernel and user spaces do not
        // share any memory page.
        if !Self::is_user_region(src, size) {
            let reason: &str = "source memory region does not lie entirely in user space";
            error!(
                "copy_from_user_unaligned(): {reason} (dst={dst:?}, src={src:?}, size={size:?})"
            );
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Check if the destination region lies entirely in kernel space.
        // NOTE: This check is sufficient because, by design, the kernel and user spaces do not
        // share any memory page.
        if !Self::is_kernel_region(dst, size) {
            let reason: &str = "destination region does not lie entirely in kernel space";
            error!(
                "copy_from_user_unaligned(): {reason} (dst={dst:?}, src={src:?}, size={size:?})"
            );
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        let copy_from_user_unaligned_impl = |dry_run: bool,
                                             mut src: VirtualAddress,
                                             mut dst: VirtualAddress,
                                             mut size: usize|
         -> Result<(), Error> {
            while size > 0 {
                let vaddr: PageAligned<VirtualAddress> =
                    PageAligned::from_address(src.align_down(PAGE_ALIGNMENT))?;
                let offset: usize = src.into_raw_value() - vaddr.into_raw_value();
                let copy_size: usize = usize::min(size, mem::PAGE_SIZE - offset);

                let src_frame: FrameAddress = self.find_user_frame(vaddr)?;

                if !dry_run {
                    // Copy memory from user space to kernel space.
                    let dst_gpa: usize = crate::hal::platform::virt_to_phys(dst.into_raw_value());
                    super::memcpy(
                        dst_gpa as *mut u8,
                        (src_frame.into_raw_value() + offset) as *const u8,
                        copy_size,
                    )?;
                }

                size -= copy_size;
                src = VirtualAddress::new(src.into_raw_value() + copy_size);
                dst = VirtualAddress::new(dst.into_raw_value() + copy_size);
            }

            Ok(())
        };

        if cfg!(feature = "nightly-performance-optimizations") {
            copy_from_user_unaligned_impl(false, src, dst, size)?;
        } else {
            // Two-pass: first validate with a dry run, then copy.
            copy_from_user_unaligned_impl(true, src, dst, size)?;
            copy_from_user_unaligned_impl(false, src, dst, size)?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Copies data from kernel space to user space. The source and destination addresses do not
    /// have to be aligned, but the destination address range must lie in user space, and the source
    /// address range must lie in kernel space.
    ///
    /// # Parameters
    ///
    /// - `dst`: Destination address in user space.
    /// - `src`: Source address in kernel space.
    /// - `size`: Number of bytes to copy.
    /// - `dry_run`: If `true`, the function does not actually copy any data.
    ///
    /// # Return Value
    ///
    /// Upon successful completion, this function returns empty. Upon failure, this function returns
    /// an error that indicates the reason for the failure.
    ///
    /// # Errors
    ///
    /// This function fails with the following error codes:
    /// - [`ErrorCode::InvalidArgument`]: The size of the copy is zero.
    /// - [`ErrorCode::BadAddress`]: The source memory region does not lie in kernel space.
    /// - [`ErrorCode::BadAddress`]: The destination memory region does not lie in user space.
    /// - [`ErrorCode::BadAddress`]: The source memory region does not lie within physical memory.
    /// - [`ErrorCode::BadAddress`]: The destination memory region does not lie within physical memory.
    ///
    /// # Safety Notes
    ///
    ///  When not running in dry-run mode, this function performs a physical memory copy. Any
    ///  errors that occur while copying data will cause this function to panic.
    ///
    pub fn copy_to_user_unaligned_unchecked(
        &mut self,
        mut dst: VirtualAddress,
        mut src: VirtualAddress,
        mut size: usize,
        dry_run: bool,
    ) -> Result<(), Error> {
        // Check if size is invalid.
        if size == 0 {
            let reason: &str = "zero-length copy";
            error!(
                "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, src={src:?}, \
                 size={size:?})"
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if the source memory region lies entirely in kernel space.
        // NOTE: This check is sufficient because, by design, the kernel and user spaces do not
        // share any memory page.
        if !Self::is_kernel_region(src, size) {
            let reason: &str = "source memory region does not lie entirely in kernel space";
            error!(
                "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, src={src:?}, \
                 size={size:?})",
            );
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Check if the destination memory region lies entirely in user space.
        // NOTE: This check is sufficient because, by design, the kernel and user spaces do not
        // share any memory page.
        if !Self::is_user_region(dst, size) {
            let reason: &str = "destination memory region does not lie entirely in user space";
            error!(
                "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, src={src:?}, \
                 size={size:?})",
            );
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Eagerly resolve any copy-on-write mappings in the destination range. Kernel writes
        // below target the destination frames via their physical alias, which bypasses the
        // page-table read-only/CoW bits, so resolution must happen up front rather than via
        // the page-fault path. The dry-run pass is skipped because it does not write.
        if !dry_run {
            self.resolve_cow_for_region(dst, size)?;
        }

        while size > 0 {
            let vaddr: PageAligned<VirtualAddress> =
                match PageAligned::from_address(dst.align_down(PAGE_ALIGNMENT)) {
                    Ok(vaddr) => vaddr,
                    Err(e) => {
                        if !dry_run {
                            let reason: &str = "failed to align destination address";
                            panic!(
                                "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, \
                                 src={src:?}, size={size:?})"
                            );
                        }
                        return Err(e);
                    },
                };

            let offset: usize = dst.into_raw_value() - vaddr.into_raw_value();
            let copy_size: usize = usize::min(mem::PAGE_SIZE - offset, size);

            let src_phys_addr_raw: usize = src.into_raw_value();

            // Check if [src_phys_addr_raw, src_phys_addr_raw + copy_size) does not lie within physical memory.
            if !Self::is_physical_region(src_phys_addr_raw, copy_size) {
                let reason: &str = "source memory region does not lie within physical memory";
                if !dry_run {
                    panic!(
                        "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, src={src:?}, \
                         size={size:?})"
                    );
                } else {
                    error!(
                        "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, src={src:?}, \
                         size={size:?})"
                    );
                }
                return Err(Error::new(ErrorCode::BadAddress, reason));
            }

            // Only perform the following operations if not in dry-run mode.
            if !dry_run {
                let dst_frame: FrameAddress = match self.find_user_frame(vaddr) {
                    Ok(frame) => frame,
                    Err(error) => {
                        let reason: &str = "failed to find user frame";
                        panic!(
                            "copy_to_user_unaligned_unchecked(): {reason} (error={error:?}, \
                             dst={dst:?}, src={src:?}, size={size:?})"
                        );
                    },
                };

                let dst_phys_addr_raw: usize = dst_frame.into_raw_value() + offset;
                // Check if [dst_phys_addr_raw, dst_phys_addr_raw + copy_size) does not lie within physical memory.
                if !Self::is_physical_region(dst_phys_addr_raw, copy_size) {
                    let reason: &str =
                        "destination memory region does not lie within physical memory";
                    panic!(
                        "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, src={src:?}, \
                         size={size:?})"
                    );
                }

                // Copy memory from kernel space to user space.
                let dst: *mut u8 = (dst_frame.into_raw_value() + offset) as *mut u8;
                let src_gpa: usize = crate::hal::platform::virt_to_phys(src.into_raw_value());
                let src: *const u8 = src_gpa as *const u8;
                let copy_result: Result<(), Error> = super::memcpy(dst, src, copy_size);
                if let Err(error) = copy_result {
                    let reason: &str = "failed to perform physical memory copy";
                    panic!(
                        "copy_to_user_unaligned_unchecked(): {reason} (error={error:?}, \
                         dst={dst:?}, src={src:?}, size={size:?})"
                    );
                }
            }

            size -= copy_size;
            dst = VirtualAddress::new(dst.into_raw_value() + copy_size);
            src = VirtualAddress::new(src.into_raw_value() + copy_size);
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Copies data from kernel space to user space. The source and destination addresses do not
    /// have to be aligned, but the destination address range must lie in user space, and the source
    /// address range must lie in kernel space.
    ///
    /// Unlike [`Self::copy_to_user_unaligned_unchecked`], this function performs a dry run first to
    /// check for errors before performing the actual copy. If any error occurs during the dry run,
    /// it returns an error without performing the copy. If the dry run is successful, it proceeds
    /// to perform the actual copy operation.
    ///
    /// # Parameters
    ///
    /// - `dst`: Destination address in user space.
    /// - `src`: Source address in kernel space.
    /// - `size`: Number of bytes to copy.
    ///
    /// # Return Value
    ///
    /// Upon successful completion, this function returns empty. Upon failure, this function returns
    /// an error that indicates the reason for the failure.
    ///
    ///
    pub fn copy_to_user_unaligned(
        &mut self,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        if cfg!(feature = "nightly-performance-optimizations") {
            self.copy_to_user_unaligned_unchecked(dst, src, size, false)
        } else {
            // Two-pass: first validate with a dry run, then copy.
            self.copy_to_user_unaligned_unchecked(dst, src, size, true)?;
            self.copy_to_user_unaligned_unchecked(dst, src, size, false)
        }
    }

    ///
    /// # Description
    ///
    /// Copies data directly between the user spaces of two processes. The source address is
    /// resolved using `src_vmem` and the destination address is resolved using `dst_vmem`. Both
    /// addresses must lie in user space. The copy is performed page-by-page using physical frame
    /// addresses, bypassing kernel space entirely.
    ///
    /// # Parameters
    ///
    /// - `src_vmem`: Source process's virtual memory space.
    /// - `src`: Source address in `src_vmem`'s user space.
    /// - `dst_vmem`: Destination process's virtual memory space.
    /// - `dst`: Destination address in `dst_vmem`'s user space.
    /// - `size`: Number of bytes to copy.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. On failure, an error is returned instead.
    ///
    /// # Errors
    ///
    /// - [`ErrorCode::InvalidArgument`]: The size of the copy is zero.
    /// - [`ErrorCode::BadAddress`]: The source memory region does not lie in user space.
    /// - [`ErrorCode::BadAddress`]: The destination memory region does not lie in user space.
    /// - [`ErrorCode::NoSuchEntry`]: A page in the source or destination region is not mapped.
    ///
    pub fn copy_user_to_user(
        src_vmem: &Vmem,
        src: VirtualAddress,
        dst_vmem: &Vmem,
        dst: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        // Check if size is invalid.
        if size == 0 {
            let reason: &str = "zero-length copy";
            error!("copy_user_to_user(): {reason} (src={src:?}, dst={dst:?}, size={size:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if the source memory region lies entirely in user space.
        if !Self::is_user_region(src, size) {
            let reason: &str = "source memory region does not lie entirely in user space";
            error!("copy_user_to_user(): {reason} (src={src:?}, dst={dst:?}, size={size:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Check if the destination memory region lies entirely in user space.
        if !Self::is_user_region(dst, size) {
            let reason: &str = "destination memory region does not lie entirely in user space";
            error!("copy_user_to_user(): {reason} (src={src:?}, dst={dst:?}, size={size:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        let copy_user_to_user_impl = |dry_run: bool,
                                      mut cur_src: VirtualAddress,
                                      mut cur_dst: VirtualAddress,
                                      mut remaining: usize|
         -> Result<(), Error> {
            while remaining > 0 {
                // Resolve source page and offset within it.
                let src_page: PageAligned<VirtualAddress> =
                    PageAligned::from_address(cur_src.align_down(PAGE_ALIGNMENT))?;
                let src_offset: usize = cur_src.into_raw_value() - src_page.into_raw_value();
                let src_avail: usize = mem::PAGE_SIZE - src_offset;

                // Resolve destination page and offset within it.
                let dst_page: PageAligned<VirtualAddress> =
                    PageAligned::from_address(cur_dst.align_down(PAGE_ALIGNMENT))?;
                let dst_offset: usize = cur_dst.into_raw_value() - dst_page.into_raw_value();
                let dst_avail: usize = mem::PAGE_SIZE - dst_offset;

                // Copy size is bounded by the nearest page boundary on either side.
                let copy_size: usize = remaining.min(src_avail).min(dst_avail);

                let src_frame: FrameAddress = src_vmem.find_user_frame(src_page)?;
                let dst_frame: FrameAddress = dst_vmem.find_user_frame(dst_page)?;

                if !dry_run {
                    // The wrapper switches to the kernel address space and ensures identity
                    // mappings for the full source and destination ranges before copying.
                    let src_phys_addr: usize = src_frame.into_raw_value() + src_offset;
                    let dst_phys_addr: usize = dst_frame.into_raw_value() + dst_offset;
                    super::memcpy(dst_phys_addr as *mut u8, src_phys_addr as *const u8, copy_size)?;
                }

                remaining -= copy_size;
                cur_src = VirtualAddress::new(cur_src.into_raw_value() + copy_size);
                cur_dst = VirtualAddress::new(cur_dst.into_raw_value() + copy_size);
            }

            Ok(())
        };

        if cfg!(feature = "nightly-performance-optimizations") {
            copy_user_to_user_impl(false, src, dst, size)?;
        } else {
            // Two-pass: the dry run walks all page tables to verify that every source and
            // destination page is mapped before any bytes are copied. This prevents partial
            // transfers that would leave the destination in an inconsistent state if a page
            // fault were encountered mid-copy.
            copy_user_to_user_impl(true, src, dst, size)?;
            copy_user_to_user_impl(false, src, dst, size)?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Fills a page with a given value in the target virtual address space.
    ///
    /// # Parameters
    ///
    /// - `dst`: Virtual address of the target page.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Upon failure, an error code is returned instead.
    ///
    pub fn memset(&mut self, dst: PageAligned<VirtualAddress>, value: u32) -> Result<(), Error> {
        // Get corresponding user page.
        let uframe: FrameAddress = self.find_user_frame(dst)?;
        let dst: PageAligned<PhysicalAddress> = uframe.into_physical_address();
        let base: *mut u8 = dst.into_raw_value() as *mut u8;

        super::memset(base, value as u8, mem::PAGE_SIZE)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps a page from the target virtual address space.
    ///
    /// If the page is not present (e.g., was never demand-paged), `Ok(None)` is returned without
    /// logging any errors. This makes the method suitable for cleaning up lazily-allocated regions
    /// such as user stacks.
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Virtual address of the target page.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(frame))` if the page was present and has been unmapped.
    /// - `Ok(None)` if the page was not present.
    /// - `Err(_)` on unexpected failures.
    ///
    pub fn unmap(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<UserFrame>, Error> {
        // Check if the provided address lies outside the user space.
        if !Self::is_user_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in user space";
            error!("{reason}");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Find the corresponding frame address, returning None if the page is not present.
        let frame_address: FrameAddress = match self.try_find_user_frame(vaddr)? {
            Some(addr) => addr,
            None => return Ok(None),
        };

        let (pgtable_vaddr, unmap_pgtable): (PageTableAddress, bool) = {
            // Get corresponding page table.
            let (pgtable_vaddr, page_table): (PageTableAddress, &mut PageTable<PageTableStorage>) = {
                let vaddr: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
                    ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
                )?;
                let pgtable_vaddr: PageTableAddress = PageTableAddress::new(vaddr);
                // Get the corresponding page directory entry.
                let pde: PageDirectoryEntry = match self.pgdir.read_pde(pgtable_vaddr) {
                    Some(pde) => pde,
                    None => {
                        let reason: &str = "failed to read page directory entry";
                        error!("{reason}");
                        return Err(Error::new(ErrorCode::TryAgain, reason));
                    },
                };

                // Get corresponding page table.
                // Check if corresponding page table does not exist.
                if !pde.is_present() {
                    let reason: &str = "page table not present";
                    error!("{reason}");
                    return Err(Error::new(ErrorCode::NoSuchEntry, reason));
                };

                (pgtable_vaddr, self.lookup_user_page_table(pgtable_vaddr)?)
            };

            let page_address: PageAddress = PageAddress::new(vaddr);

            // Check if frame address matches what we expect.
            if page_table.lookup(page_address)? != frame_address {
                // The following statement should not be reachable because after mapping user frame we
                // must have added it to the list of user pages.
                unreachable!("frame address must match what we expect");
            }

            // Unmap the page from the target virtual address space.
            page_table.unmap(page_address)?;

            (pgtable_vaddr, page_table.nmapped() == 0)
        };

        // Mirror the unmapping into the per-process hardware page table (x86_64).
        self.hw_unmap_user(vaddr.into_raw_value());

        //====================================================================================
        // NOTE: if we fail beyond this point and we want to recover we should remap the page.
        //====================================================================================

        if unmap_pgtable {
            // Remove page table from the list of user page tables.
            let at = self
                .user_page_tables
                .iter()
                .position(|(addr, _)| addr == &pgtable_vaddr)
                .expect("page table must be in the list of user page tables");

            let (_pgtable_addr, _page_table) = self.user_page_tables.remove(at);

            self.pgdir.unmap(pgtable_vaddr)?;
        }

        Ok(Some(UserFrame::new(frame_address)))
    }

    /// Changes access permissions on a page.
    pub fn uctrl(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        // Check if the provided address lies outside the user space.
        if !Self::is_user_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in user space";
            error!("{reason}");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Get corresponding page table.
        let page_table: &mut PageTable<PageTableStorage> = {
            let vaddr: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
                ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
            )?;
            let pgtable_vaddr: PageTableAddress = PageTableAddress::new(vaddr);
            // Get the corresponding page directory entry.
            let pde: PageDirectoryEntry = match self.pgdir.read_pde(pgtable_vaddr) {
                Some(pde) => pde,
                None => {
                    let reason: &str = "failed to read page directory entry";
                    error!("{reason}");
                    return Err(Error::new(ErrorCode::TryAgain, reason));
                },
            };

            // Check if corresponding page table does not exist.
            if !pde.is_present() {
                let reason: &str = "page table not present";
                error!("{reason}");
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            };

            self.lookup_user_page_table(pgtable_vaddr)?
        };

        let page_address: PageAddress = PageAddress::new(vaddr);

        // Change access permissions on the page.
        page_table.ctrl(false, page_address, access)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Changes access permissions on a kernel page. When `dry_run` is `true`, validates that the
    /// operation would succeed without modifying any page table entries.
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Virtual address of the target kernel page.
    /// - `access`: New access permissions for the page.
    /// - `dry_run`: If `true`, only validates the operation without applying changes.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this function returns empty. Upon failure, this function returns
    /// an error that indicates the reason for the failure.
    ///
    /// # Errors
    ///
    /// This function fails with the following error codes:
    /// - [`ErrorCode::BadAddress`]: The provided address does not lie in kernel space.
    /// - [`ErrorCode::TryAgain`]: Failed to read the page directory entry.
    /// - [`ErrorCode::NoSuchEntry`]: The corresponding page table is not present.
    /// - [`ErrorCode::NoSuchEntry`]: The page table entry was not found (dry run only).
    ///
    pub fn kctrl(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
        dry_run: bool,
    ) -> Result<(), Error> {
        trace!("{vaddr:?}");

        // Check if the provided address lies outside the kernel space.
        if !Self::is_kernel_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in kernel space";
            error!("{reason}");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Get corresponding page table.
        let page_table = {
            let vaddr: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
                ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
            )?;
            // Get the corresponding page directory entry.
            let pde: PageDirectoryEntry = match self.pgdir.read_pde(PageTableAddress::new(vaddr)) {
                Some(pde) => pde,
                None => {
                    let reason: &str = "failed to read page directory entry";
                    error!("{reason}");
                    return Err(Error::new(ErrorCode::TryAgain, reason));
                },
            };

            // Check if corresponding page table does not exist.
            if !pde.is_present() {
                let reason: &str = "page table not present";
                error!("{reason}");
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            };

            self.lookup_kernel_page_table(&pde)?
        };

        let page_address: PageAddress = PageAddress::new(vaddr);

        if dry_run {
            // For dry-run validation, check if the PTE is present or can be created.
            let pt_ref = page_table.borrow();
            match pt_ref.1.is_page_present(page_address) {
                Ok(true) => {},
                // PTE absent — will be created in the non-dry-run pass.
                Ok(false) => {},
                Err(e) => return Err(e),
            }
        } else {
            let mut pt_mut = page_table.borrow_mut();
            // If the PTE is not present, create an identity-mapped entry first.
            match pt_mut.1.is_page_present(page_address) {
                Ok(false) => {
                    let frame_addr: FrameAddress =
                        FrameAddress::new(PageAligned::from_raw_value(vaddr.into_raw_value())?);
                    pt_mut.1.map(
                        page_address,
                        frame_addr,
                        false, // user-accessible (MMIO regions are mapped for user processes)
                        false, // not write-through
                        false, // cache disabled (MMIO-safe: prevents stale/speculative reads)
                        access,
                    )?;
                },
                Ok(true) => {
                    pt_mut.1.ctrl(false, page_address, access)?;
                },
                Err(e) => return Err(e),
            }
            // Release the borrow on the kernel page table before touching the hardware tables.
            drop(pt_mut);

            // Mirror the (user-accessible) identity MMIO mapping into the shared boot-PML4 kernel
            // page directory so user processes can reach it on x86_64 (no-op on other arches).
            self.hw_map_kernel_mmio(
                vaddr.into_raw_value(),
                vaddr.into_raw_value(),
                access.is_writable(),
            );
        }

        Ok(())
    }
}

impl Drop for Vmem {
    fn drop(&mut self) {
        // Safety net: by the time a `Vmem` is dropped, every user frame it mapped must already
        // have been reclaimed (via `clear_user_space()` or explicit unmapping). Dropping the user
        // page tables below frees only their backing storage, NOT the user frames their entries
        // reference, so any user page still mapped here is a leaked frame. Catch teardown paths
        // that forget to reclaim user frames in debug and test builds.
        debug_assert!(
            self.user_page_tables
                .iter()
                .all(|(_, page_table)| page_table.nmapped() == 0),
            "Vmem dropped with user pages still mapped: user frames would leak"
        );

        while let Some((_pgtable_vaddr, user_page_table)) = self.user_page_tables.pop_front() {
            drop(user_page_table);
        }

        // Unmap shared kernel pages.
        while let Some(entry) = self.kernel_pages.pop_front() {
            drop(entry);
        }

        // Unmap shared kernel page tables.
        while let Some(entry) = self.kernel_page_tables.pop_front() {
            drop(entry)
        }

        // Reclaim the per-process hardware page table (x86_64). This returns every process-private
        // page-table page (PDPT, user PDs/PTs, and the LAPIC tables) to the hardware page-table
        // free list; the shared kernel PD0 is never freed.
        #[cfg(target_arch = "x86_64")]
        if self.hw_pml4 != 0 {
            unsafe {
                proof_with! {
                    Tracked(&self.hwpt_manager),
                    Tracked(&mut self.hw_pages)
                };
                crate::hal::arch::x86::mem::mmu::hwpt::destroy_user_pml4(self.hw_pml4);
            }
            self.hw_pml4 = 0;
        }
    }
}
