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
        phys::UserFrame,
        virt::{
            kpage::KernelPage,
            page_table_allocator::PAGE_TABLE_ALLOCATOR,
            PageDirectoryStorage,
            PageTableStorage,
        },
    },
};
use ::alloc::{
    collections::LinkedList,
    rc::Rc,
};
use ::arch::{
    cpu::cr3::{
        Cr3Register,
        PageDirectoryBaseAddress as Cr3PageDirectoryBaseAddress,
        PageLevelCacheDisableFlag,
        PageLevelWriteThroughFlag,
    },
    mem::{
        self,
        paging::{
            PageDirectoryEntry,
            PteWord,
        },
        PAGE_ALIGNMENT,
        PAGE_TABLE_LENGTH,
        PGTAB_ALIGNMENT,
    },
};
use ::config::kernel::MEMORY_SIZE;
use ::core::cell::RefCell;
use ::sys::{
    config,
    error::{
        Error,
        ErrorCode,
    },
};

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
    /// List of private kernel pages.
    private_kernel_pages: LinkedList<KernelPage>,
    /// List of user page tables.
    user_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
}

impl Vmem {
    /// Allocates a fresh 4KB scratch page and returns it wrapped as a
    /// [`crate::hal::mem::FrameAddress`] so callers (frame allocator,
    /// kpool) don't each have to repeat the GPA-to-FrameAddress dance.
    #[cfg(feature = "hyperlight")]
    #[allow(dead_code)]
    pub fn alloc_scratch_frame() -> Result<crate::hal::mem::FrameAddress, Error> {
        use crate::hal::mem::{
            FrameAddress,
            PageAligned,
            PhysicalAddress,
            VirtualAddress,
        };
        let gpa: u32 = Self::alloc_scratch_page();
        if gpa == 0 {
            return Err(Error::new(ErrorCode::OutOfMemory, "scratch bump allocator exhausted"));
        }
        let phys: PhysicalAddress = unsafe {
            PhysicalAddress::from_mmio_address(VirtualAddress::from_raw_value(gpa as usize))?
        };
        let aligned: PageAligned<PhysicalAddress> = PageAligned::from_address(phys)?;
        Ok(FrameAddress::new(aligned))
    }

    /// Allocates a fresh 4KB page from the scratch bump allocator.
    /// Returns the GPA of the new page, or 0 if out of scratch memory.
    ///
    /// The cursor lives in the host-published slot inside scratch itself
    /// (at `ALLOCATOR_GVA = 0xFFFFFFF0`). Keeping it in scratch (rather
    /// than kernel BSS) avoids a bootstrap recursion on HL with
    /// PTE_COW-from-boot: a BSS-hosted cursor would CoW on first write,
    /// and the CoW handler itself needs to call `alloc_scratch_page`.
    #[cfg(feature = "hyperlight")]
    pub fn alloc_scratch_page() -> u32 {
        const ALLOCATOR_GVA: u32 = 0xFFFF_FFF0;
        unsafe {
            let alloc_ptr = ALLOCATOR_GVA as *mut u32;
            let current = core::ptr::read_volatile(alloc_ptr);
            if current >= Self::SCRATCH_TOP_GPA {
                return 0;
            }
            let new_val = current + 4096;
            core::ptr::write_volatile(alloc_ptr, new_val);
            current
        }
    }

    /// First byte of scratch memory that is NOT available for frame
    /// allocation — scratch GPAs `[scratch_base, SCRATCH_TOP_GPA)` are
    /// what the allocator hands out. The GDT sits at `SCRATCH_TOP_GPA`
    /// and the CoW bookkeeping slots (scratch_size, allocator cursor)
    /// sit above that.
    #[cfg(feature = "hyperlight")]
    const SCRATCH_TOP_GPA: u32 = 0xFFFD_F000;

    /// Returns `(first_frame_index, frame_count)` describing the scratch
    /// region as it exists in the *global frame-number* coordinate space
    /// used by the sparse bitmap (frame index = `gpa / PAGE_SIZE`).
    ///
    /// `scratch_size` is read from the host-published slot at
    /// `SCRATCH_SIZE_GVA = 0xFFFF_FFF8` — the same slot the CoW entry
    /// stub consults to decide whether CoW is active. The returned
    /// frame count is rounded up to a multiple of 8 to satisfy
    /// [`::bitmap::Bitmap::new`]'s alignment requirement; unused
    /// bits at the tail are never addressed by real allocations because
    /// [`Self::alloc_scratch_page`] stops at [`Self::SCRATCH_TOP_GPA`].
    #[cfg(feature = "hyperlight")]
    pub fn scratch_range_in_frames() -> (usize, usize) {
        const SCRATCH_SIZE_GVA: u32 = 0xFFFF_FFF8;
        let scratch_size: u64 = unsafe { core::ptr::read_volatile(SCRATCH_SIZE_GVA as *const u64) };
        let scratch_base_gpa: u32 = (u32::MAX as u64 - scratch_size + 1) as u32;
        let frame_count_unaligned: usize =
            ((Self::SCRATCH_TOP_GPA - scratch_base_gpa) / ::arch::mem::PAGE_SIZE as u32) as usize;
        // Bitmap::new requires a multiple of 8 bits. Round up.
        let frame_count: usize = frame_count_unaligned.next_multiple_of(8);
        let first_frame_index: usize = scratch_base_gpa as usize / ::arch::mem::PAGE_SIZE;
        (first_frame_index, frame_count)
    }

    /// Initializes a new virtual memory space.
    #[cfg_attr(feature = "hyperlight", allow(dead_code))]
    pub fn new(
        mut kernel_pages: LinkedList<KernelPage>,
        mut kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) -> Result<Self, Error> {
        trace!("kernel_pages.len()={}", kernel_pages.len());

        // Create a clean page directory.
        let mut pgdir: PageDirectory<PageDirectoryStorage> =
            // SAFETY: this constructor is only used during early single-threaded init;
            // BSS is zero-initialized, so assume_init_mut() is sound for integer arrays.
            PageDirectory::new(PageDirectoryStorage::Bss(unsafe {
                PAGE_TABLE_ALLOCATOR
                    .alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()
                    .map_err(|e| {
                        error!("Vmem::new(): page directory allocation failed: {}", e);
                        Error::new(ErrorCode::OutOfMemory, "BSS page directory allocation failed")
                    })?
                    .assume_init_mut()
            }));

        // Map and store root page tables.
        let mut kpage_tables: LinkedList<
            Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>,
        > = LinkedList::new();
        while let Some((vaddr, page_table)) = kernel_page_tables.pop_front() {
            let page_table_address: FrameAddress = page_table.physical_address()?;
            // FIXME: do not be so open about permissions.
            pgdir.map(vaddr, page_table_address, false, AccessPermission::RDWR)?;
            kpage_tables.push_back(Rc::new(RefCell::new((vaddr, page_table))));
        }

        // Register kernel PD for lazy identity mapping. On x86, the PD is also the CR3 root.
        let pd_paddr_raw: usize = pgdir.physical_address()?.into_raw_value();
        let pd_paddr: PageDirectoryAddress = PageDirectoryAddress::from_raw_value(pd_paddr_raw)?;
        let kernel_cr3: Cr3Register = Cr3Register {
            page_level_write_through: PageLevelWriteThroughFlag::Disabled,
            page_level_cache_disable: PageLevelCacheDisableFlag::Enabled,
            paging_structure_base_address: Cr3PageDirectoryBaseAddress::new(pd_paddr_raw as u32)
                .ok_or_else(|| {
                    Error::new(
                        ErrorCode::BadAddress,
                        "kernel page directory address is not 4 KB aligned",
                    )
                })?,
        };
        super::identity_map::init(pd_paddr, kernel_cr3);

        // Store root pages.
        let mut kpages: LinkedList<Rc<RefCell<KernelPage>>> = LinkedList::new();
        while let Some(entry) = kernel_pages.pop_front() {
            kpages.push_back(Rc::new(RefCell::new(entry)));
        }

        Ok(Self {
            pgdir,
            kernel_page_tables: kpage_tables,
            kernel_pages: kpages,
            private_kernel_pages: LinkedList::new(),
            user_page_tables: LinkedList::new(),
        })
    }

    /// Clones the target virtual memory space.
    /// Creates a Vmem that shares the parent's page directory.
    /// User-space pages mapped through this Vmem go into the shared PD.
    /// Only safe when there's a single user process (Hyperlight).
    #[cfg(feature = "hyperlight")]
    #[allow(dead_code)]
    pub fn share_pd(from: &Vmem) -> Result<Vmem, Error> {
        let pa = from.pgdir.physical_address()?;
        let shared_storage = PageDirectoryStorage::from_cr3(pa.into_raw_value() as u32);
        Ok(Self {
            pgdir: PageDirectory::from_existing(shared_storage),
            kernel_page_tables: from.kernel_page_tables.clone(),
            kernel_pages: from.kernel_pages.clone(),
            private_kernel_pages: LinkedList::new(),
            user_page_tables: LinkedList::new(),
        })
    }

    /// Switches the Vmem's page directory to the given PD GPA in scratch.
    /// After snapshot restore, each process gets its own rebuilt PD.
    /// Also updates kernel page table pointers to match the new PD's entries.
    #[cfg(feature = "hyperlight")]
    pub fn adopt_pd(&mut self, pd_gpa: u32) {
        self.pgdir = PageDirectory::from_existing(PageDirectoryStorage::from_cr3(pd_gpa));

        // Update kernel page tables — point each stored PT's storage
        // to the corresponding scratch PT page (from the adopted PD).
        let pd_ptr = pd_gpa as *const u32;
        for pt_entry in self.kernel_page_tables.iter_mut() {
            let mut borrowed = pt_entry.borrow_mut();
            let pt_vaddr = borrowed.0;
            let pdi = pt_vaddr.get_pde_index();
            let pde = unsafe { core::ptr::read_volatile(pd_ptr.add(pdi)) };
            if (pde & 1) != 0 {
                let pt_gpa = pde & 0xFFFFF000;
                let nmapped = borrowed.1.nmapped();
                borrowed.1 = PageTable::from_existing(
                    PageTableStorage::Scratch(pt_gpa as *mut [u32; 1024]),
                    nmapped,
                );
            }
        }
    }

    /// Resolves a virtual address to its physical address by walking the
    /// hardware page table (CR3 → PD → PT → frame). After CoW, the frame's
    /// PA may differ from the identity-mapped VA. Returns 0 if unmapped.
    #[cfg(feature = "hyperlight")]
    #[allow(dead_code)]
    pub fn resolve_pa(va: usize) -> u32 {
        // SAFETY: caller runs at privilege level 0.
        let cr3: ::arch::cpu::cr3::Cr3Register = unsafe { ::arch::cpu::cr3::Cr3Register::read() };
        let pd_pa: u32 = cr3.paging_structure_base_address.address();
        unsafe {
            let pd = pd_pa as *const u32;
            let pdi = (va >> 22) & 0x3FF;
            let pde = core::ptr::read_volatile(pd.add(pdi));
            if (pde & 1) == 0 {
                return 0;
            }
            let pt = (pde & 0xFFFFF000) as *const u32;
            let pti = (va >> 12) & 0x3FF;
            let pte = core::ptr::read_volatile(pt.add(pti));
            if (pte & 1) == 0 {
                return 0;
            }
            pte & 0xFFFFF000
        }
    }

    pub fn clone(from: &Vmem, pgdir_page: KernelPage) -> Result<Vmem, Error> {
        // Create a clean page directory backed by a kernel page from the pool.
        let mut pgdir: PageDirectory<PageDirectoryStorage> =
            PageDirectory::new(PageDirectoryStorage::KernelPage(pgdir_page));

        // Map and store root page tables.
        let mut kernel_page_tables: LinkedList<
            Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>,
        > = LinkedList::new();
        for entry in from.kernel_page_tables.iter() {
            let page_table_address: FrameAddress = entry.borrow().1.physical_address()?;
            // FIXME: do not be so open about permissions.
            pgdir.map(entry.borrow().0, page_table_address, false, AccessPermission::RDWR)?;
            kernel_page_tables.push_back(entry.clone());
        }

        // Store root pages.
        let mut kernel_pages: LinkedList<Rc<RefCell<KernelPage>>> = LinkedList::new();
        for entry in from.kernel_pages.iter() {
            kernel_pages.push_back(entry.clone());
        }

        Ok(Self {
            pgdir,
            kernel_page_tables,
            kernel_pages,
            private_kernel_pages: LinkedList::new(),
            user_page_tables: LinkedList::new(),
        })
    }

    /// Creates a virtual memory space from BSS-backed storage that was populated
    /// from Hyperlight's page tables. Switches CR3 to the new BSS-backed page directory
    /// so that the mappings survive snapshot restore.
    #[cfg(feature = "hyperlight")]
    pub fn from_existing(
        pd_storage: PageDirectoryStorage,
        mut kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) -> Result<Self, Error> {
        // Wrap the scratch-resident PD without zeroing — entries already populated
        // by the host. CR3 already points to this PD; no reload needed.
        let pgdir: PageDirectory<PageDirectoryStorage> = PageDirectory::from_existing(pd_storage);

        // Convert page tables into Rc<RefCell<...>> for shared ownership.
        let mut kpage_tables: LinkedList<
            Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>,
        > = LinkedList::new();
        while let Some(entry) = kernel_page_tables.pop_front() {
            kpage_tables.push_back(Rc::new(RefCell::new(entry)));
        }

        Ok(Self {
            pgdir,
            kernel_page_tables: kpage_tables,
            kernel_pages: LinkedList::new(),
            private_kernel_pages: LinkedList::new(),
            user_page_tables: LinkedList::new(),
        })
    }

    #[cfg_attr(feature = "hyperlight", allow(dead_code))]
    pub fn load(&self) -> Result<(), Error> {
        let pgdir_addr: FrameAddress = self.pgdir.physical_address()?;
        let pd_pa = pgdir_addr.into_raw_value();
        // On Hyperlight with PTE_COW, the PD page may have been CoW'd to
        // scratch. Resolve the VA through hardware PTs to find where the
        // actual populated PD data lives.
        #[cfg(feature = "hyperlight")]
        let pd_pa = {
            let resolved = Self::resolve_pa(pd_pa) as usize;
            if resolved != 0 {
                resolved
            } else {
                pd_pa
            }
        };
        unsafe { mmu::load_page_directory(pd_pa) };
        Ok(())
    }

    /// Returns a reference to the underlying page directory.
    pub fn pgdir(&self) -> &PageDirectory<PageDirectoryStorage> {
        &self.pgdir
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
    #[cfg_attr(feature = "hyperlight", allow(dead_code))]
    pub fn map_kpage<T: Fn() -> Result<PageTable<PageTableStorage>, Error>>(
        &mut self,
        kpage: KernelPage,
        vaddr: PageAligned<VirtualAddress>,
        page_table_allocator: T,
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
            let page_table: PageTable<PageTableStorage> = page_table_allocator()?;

            // FIXME: do not be so open about permissions.
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

    /// Maps a page to the target virtual address space.
    pub fn map<T: Fn() -> Result<PageTable<PageTableStorage>, Error>>(
        &mut self,
        uframe: UserFrame,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
        page_table_allocator: T,
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
                let page_table: PageTable<PageTableStorage> = page_table_allocator()?;

                let page_table_address: FrameAddress = page_table.physical_address()?;
                // FIXME: do not be so open about permissions.
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
        // On Hyperlight, the frame allocator already redirects to scratch GPAs.
        let frame_addr = uframe.address();
        page_table.map(PageAddress::new(vaddr), frame_addr, false, false, true, access)?;

        // Flush TLB for this page after remapping.
        unsafe {
            let va = vaddr.into_raw_value();
            core::arch::asm!("invlpg [{}]", in(reg) va, options(nostack));
        }

        //=============================================================
        // NOTE: if we fail beyond this point we should unmap the page.
        //=============================================================

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
        // Reject zero-length regions.
        if size == 0 {
            return false;
        }

        // Check if the start and end addresses of the region lie within physical memory.
        match start.checked_add(size - 1) {
            Some(end) => start < MEMORY_SIZE && end < MEMORY_SIZE,
            None => false,
        }
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
        _pgtable_vaddr: PageTableAddress,
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
            // On Hyperlight after restore, PT pages are rebuilt in scratch.
            // The stored PT's PA (kpool) differs from the PDE's frame (scratch).
            // Match by the VA range the PT covers instead.
            let matches = if crate::hal::platform::use_va_copies() {
                pt.borrow().0 == _pgtable_vaddr
            } else {
                pt.borrow().1.physical_address()? == pgtab_addr
            };

            if matches {
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

                if crate::hal::platform::use_va_copies() {
                    if !dry_run {
                        // Hyperlight: source is a user VA. Switch CR3 to the
                        // CoW-resolved user PD so the VA resolves correctly,
                        // then do a VA-level copy. Any PTE_COW page traps via
                        // the #PF handler and gets cloned to scratch.
                        #[cfg(feature = "hyperlight")]
                        let _cr3_guard = unsafe { cr3_switch_to_user_pd(&self.pgdir) };
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                src.into_raw_value() as *const u8,
                                dst.into_raw_value() as *mut u8,
                                copy_size,
                            );
                        }
                    }
                } else {
                    let src_frame: FrameAddress = self.find_user_frame(vaddr)?;
                    if !dry_run {
                        // Copy memory from user space to kernel space.
                        // SAFETY: The following conditions are guaranteed:
                        // - `dst.into_raw_value()` is a valid kernel-space address for `copy_size` bytes.
                        // - `src_frame.into_raw_value() + offset` is a valid user-space address for `copy_size` bytes.
                        // - Both regions are non-overlapping and accessible for the operation.
                        super::identity_map::memcpy(
                            dst.into_raw_value() as *mut u8,
                            (src_frame.into_raw_value() + offset) as *const u8,
                            copy_size,
                        )?;
                    }
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
        &self,
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
            // Skip on Hyperlight after CoW (identity mapping broken).
            let src_in_bounds = crate::hal::platform::use_va_copies()
                || Self::is_physical_region(src_phys_addr_raw, copy_size);
            if !src_in_bounds {
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
                let dst_in_bounds = crate::hal::platform::use_va_copies()
                    || Self::is_physical_region(dst_phys_addr_raw, copy_size);
                if !dst_in_bounds {
                    let reason: &str =
                        "destination memory region does not lie within physical memory";
                    panic!(
                        "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, src={src:?}, \
                         size={size:?})"
                    );
                }

                // Copy memory from kernel space to user space.
                if crate::hal::platform::use_va_copies() {
                    // Hyperlight: switch CR3 to the CoW-resolved user PD so user
                    // VAs resolve correctly, clear CR0.WP so the kernel can write
                    // into PTE-readonly user code pages, do the VA-level copy,
                    // restore both. PTE_COW pages trap via the #PF handler.
                    #[cfg(feature = "hyperlight")]
                    let _cr3_guard = unsafe { cr3_switch_to_user_pd(&self.pgdir) };
                    #[cfg(feature = "hyperlight")]
                    let _wp_guard = unsafe { cr0_disable_write_protect() };
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            src.into_raw_value() as *const u8,
                            dst.into_raw_value() as *mut u8,
                            copy_size,
                        );
                    }
                } else {
                    let dst: *mut u8 = (dst_frame.into_raw_value() + offset) as *mut u8;
                    let src: *const u8 = src.into_raw_value() as *const u8;
                    let copy_result: Result<(), Error> =
                        super::identity_map::memcpy(dst, src, copy_size);
                    if let Err(error) = copy_result {
                        let reason: &str = "failed to perform physical memory copy";
                        panic!(
                            "copy_to_user_unaligned_unchecked(): {reason} (error={error:?}, \
                             dst={dst:?}, src={src:?}, size={size:?})"
                        );
                    }
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
        &self,
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
                    super::identity_map::memcpy(
                        dst_phys_addr as *mut u8,
                        src_phys_addr as *const u8,
                        copy_size,
                    )?;
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
        if crate::hal::platform::use_va_copies() {
            // Hyperlight: VA-level memset via CR3 switch + CR0.WP toggle.
            #[cfg(feature = "hyperlight")]
            let _cr3_guard = unsafe { cr3_switch_to_user_pd(&self.pgdir) };
            #[cfg(feature = "hyperlight")]
            let _wp_guard = unsafe { cr0_disable_write_protect() };
            unsafe {
                core::ptr::write_bytes(
                    dst.into_raw_value() as *mut u8,
                    value as u8,
                    mem::PAGE_SIZE,
                );
            }
            return Ok(());
        }

        let uframe: FrameAddress = self.find_user_frame(dst)?;
        let phys_dst: PageAligned<PhysicalAddress> = uframe.into_physical_address();
        let base: *mut u8 = phys_dst.into_raw_value() as *mut u8;
        super::identity_map::memset(base, value as u8, mem::PAGE_SIZE)?;

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
    #[cfg_attr(feature = "hyperlight", allow(dead_code))]
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

            self.lookup_kernel_page_table(&pde, PageTableAddress::new(vaddr))?
        };

        let page_address: PageAddress = PageAddress::new(vaddr);

        if dry_run {
            page_table.borrow().1.lookup(page_address)?;
        } else {
            page_table
                .borrow_mut()
                .1
                .ctrl(false, page_address, access)?;
        }

        Ok(())
    }
}

//==================================================================================================
// Hyperlight RAII guards for VA-based user copies
//==================================================================================================

///
/// RAII guard that restores the original CR3 on drop. Used by Hyperlight copy
/// paths that temporarily switch to a user process's PD so that VA-level copies
/// resolve user VAs correctly.
///
#[cfg(feature = "hyperlight")]
pub(super) struct Cr3Guard {
    saved: ::arch::cpu::cr3::Cr3Register,
    switched: bool,
}

#[cfg(feature = "hyperlight")]
impl Drop for Cr3Guard {
    fn drop(&mut self) {
        if self.switched {
            // SAFETY: caller runs at privilege level 0; `saved` is a CR3 value read
            // earlier in the same call path.
            unsafe { self.saved.write() };
        }
    }
}

///
/// Switches CR3 to the CoW-resolved physical address of `pgdir`, returning an
/// RAII guard that restores the previous CR3 on drop. If `pgdir` cannot be
/// resolved (e.g., stale PA that no longer maps to anything), no switch is
/// performed.
///
/// # Safety
///
/// Caller must run at privilege level 0 and must ensure that the resolved PA
/// points at a populated page directory.
///
#[cfg(feature = "hyperlight")]
pub(super) unsafe fn cr3_switch_to_user_pd(
    pgdir: &PageDirectory<PageDirectoryStorage>,
) -> Cr3Guard {
    let old: ::arch::cpu::cr3::Cr3Register = unsafe { ::arch::cpu::cr3::Cr3Register::read() };
    let pd_va: u32 = pgdir
        .physical_address()
        .ok()
        .map(|f| f.into_raw_value() as u32)
        .unwrap_or(0);
    if pd_va == 0 {
        return Cr3Guard {
            saved: old,
            switched: false,
        };
    }
    let resolved_pa: u32 = Vmem::resolve_pa(pd_va as usize);
    if resolved_pa == 0 || resolved_pa == old.paging_structure_base_address.address() {
        return Cr3Guard {
            saved: old,
            switched: false,
        };
    }
    // SAFETY: resolved_pa was obtained from the currently active page tables and is page-aligned.
    let new_base: ::arch::cpu::cr3::PagingStructureBaseAddress =
        ::arch::cpu::cr3::PagingStructureBaseAddress::new(resolved_pa)
            .expect("resolve_pa returned a page-aligned address");
    let new_cr3: ::arch::cpu::cr3::Cr3Register = ::arch::cpu::cr3::Cr3Register {
        page_level_write_through: old.page_level_write_through,
        page_level_cache_disable: old.page_level_cache_disable,
        paging_structure_base_address: new_base,
    };
    // SAFETY: caller at CPL 0; new CR3 holds a valid, resolved PD PA.
    unsafe { new_cr3.write() };
    Cr3Guard {
        saved: old,
        switched: true,
    }
}

///
/// RAII guard that restores the original CR0.WP value on drop. Used by
/// Hyperlight copy paths that need to write into PTE-readonly user code pages
/// from ring 0 while CR0.WP is set.
///
#[cfg(feature = "hyperlight")]
pub(super) struct Cr0WpGuard {
    saved: ::arch::cpu::cr0::Cr0Register,
}

#[cfg(feature = "hyperlight")]
impl Drop for Cr0WpGuard {
    fn drop(&mut self) {
        // SAFETY: caller runs at privilege level 0.
        unsafe { self.saved.write() };
    }
}

///
/// Clears CR0.WP so ring-0 writes bypass PTE read-only enforcement, returning a
/// guard that restores the original CR0 on drop.
///
/// # Safety
///
/// Caller must run at privilege level 0.
///
#[cfg(feature = "hyperlight")]
pub(super) unsafe fn cr0_disable_write_protect() -> Cr0WpGuard {
    let saved: ::arch::cpu::cr0::Cr0Register = unsafe { ::arch::cpu::cr0::Cr0Register::read() };
    let mut disabled: ::arch::cpu::cr0::Cr0Register = saved;
    disabled.write_protect = ::arch::cpu::cr0::WriteProtectFlag::Disabled;
    // SAFETY: caller at CPL 0.
    unsafe { disabled.write() };
    Cr0WpGuard { saved }
}

impl Drop for Vmem {
    fn drop(&mut self) {
        while let Some((_pgtable_vaddr, user_page_table)) = self.user_page_tables.pop_front() {
            drop(user_page_table);
        }

        // Unmap all kernel private kernel pages.
        while let Some(kpage) = self.private_kernel_pages.pop_front() {
            drop(kpage)
        }

        // Unmap shared kernel pages.
        while let Some(entry) = self.kernel_pages.pop_front() {
            drop(entry);
        }

        // Unmap shared kernel page tables.
        while let Some(entry) = self.kernel_page_tables.pop_front() {
            drop(entry)
        }
    }
}
