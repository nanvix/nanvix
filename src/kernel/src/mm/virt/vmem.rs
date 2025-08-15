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
            page_directory::{
                PageDirectory,
                PageDirectoryStorage,
            },
            page_table::PageTable,
        },
        mem::{
            AccessPermission,
            Address,
            FrameAddress,
            PageAddress,
            PageAligned,
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
    paging::PageDirectoryEntry,
    PAGE_ALIGNMENT,
    PGTAB_ALIGNMENT,
};
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

unsafe extern "C" {
    ///
    /// # Description
    ///
    /// Performs a physical memory copy.
    ///
    /// # Parameters
    ///
    /// - `dst`: Destination address.
    /// - `src`: Source address.
    /// - `size`: Number of bytes to copy.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs physical memory copying and may lead to
    /// undefined behavior if the destination or source memory regions are invalid.
    ///
    fn __phys_memcpy(dst: *mut u8, src: *const u8, size: usize);

    ///
    /// # Description
    ///
    /// This function disables paging and fills the memory region with the provided value.
    ///
    /// # Parameters
    ///
    /// - `base`: Base address of the memory region.
    /// - `value`: Value to fill the memory region with.
    /// - `size`: Size of the memory region.
    ///
    /// # Safety
    ///
    /// This function may lead to undefined behavior if the provided base address is not
    /// correct.
    ///
    fn __phys_memset(base: *mut u8, value: u8, size: usize);
}

/// A type that represents a virtual memory space.
pub struct Vmem {
    /// Underlying page directory.
    pgdir: PageDirectory,
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
    /// Initializes a new virtual memory space.
    pub fn new(
        mut kernel_pages: LinkedList<KernelPage>,
        mut kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) -> Result<Self, Error> {
        trace!("new()");

        // Create a clean page directory.
        let mut pgdir: PageDirectory = PageDirectory::new(PageDirectoryStorage::new());
        pgdir.clean();

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
    pub fn clone(from: &Vmem) -> Result<Vmem, Error> {
        // Create a clean page directory.
        let mut pgdir: PageDirectory = PageDirectory::new(PageDirectoryStorage::new());
        pgdir.clean();

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

    pub fn load(&self) -> Result<(), Error> {
        let pgdir_addr: FrameAddress = self.pgdir.physical_address()?;
        unsafe { mmu::load_page_directory(pgdir_addr.into_raw_value()) };
        Ok(())
    }

    /// Returns a reference to the underlying page directory.
    pub fn pgdir(&self) -> &PageDirectory {
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
                error!("map_kpage(): {}", reason);
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
        error!("lookup_kernel_page_table(): {}", reason);
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
            error!(
                "map(): {} (uframe={:?}, vaddr={:?}, access={:?})",
                reason, uframe, vaddr, access
            );
            return Err(Error::new(ErrorCode::BadAddress, "address is not in user space"));
        }

        // Get corresponding page table.
        let page_table: &mut PageTable<PageTableStorage> = {
            let vaddr: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
                ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
            )?;
            let pgtable_vaddr: PageTableAddress = PageTableAddress::new(vaddr);
            // Get the corresponding page directory entry.
            let mut pde: PageDirectoryEntry = match self.pgdir.read_pde(pgtable_vaddr) {
                Some(pde) => pde,
                None => {
                    let reason: &str = "failed to read page directory entry";
                    error!("map(): {}", reason);
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

                self.user_page_tables.push_back((pgtable_vaddr, page_table));

                // Get the corresponding page directory entry.
                pde = match self.pgdir.read_pde(PageTableAddress::new(vaddr)) {
                    Some(pde) => pde,
                    None => unreachable!("failed to read page directory entry"),
                };
            };

            self.lookup_page_table(&pde)?
        };

        // Map the page to the target virtual address space.
        page_table.map(PageAddress::new(vaddr), uframe.address(), false, false, true, access)?;

        //=============================================================
        // NOTE: if we fail beyond this point we should unmap the page.
        //=============================================================

        Ok(())
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

    /// Asserts wether an address lies in the kernel space.
    fn is_kernel_addr(virt_addr: VirtualAddress) -> bool {
        !Self::is_user_addr(virt_addr)
    }

    /// Looks up a page table in the list of page tables.
    fn lookup_page_table(
        &mut self,
        pde: &PageDirectoryEntry,
    ) -> Result<&mut PageTable<PageTableStorage>, Error> {
        // Check if corresponding page table does not exist.
        if !pde.is_present() {
            let reason: &str = "page table not present";
            error!("lookup_page_table(): {reason:?} (pde={pde:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        // Get corresponding page table.
        let pgtab_addr: FrameAddress = FrameAddress::from_frame_number(pde.frame())?;

        // Find corresponding page table.
        let mut page_table: Option<&mut PageTable<PageTableStorage>> = None;
        for (_pgtable_vaddr, pt) in self.user_page_tables.iter_mut() {
            if pt.physical_address()? == pgtab_addr {
                page_table = Some(pt);
                break;
            }
        }

        match page_table {
            Some(pt) => Ok(pt),
            None => {
                let reason: &str = "page table not found";
                error!("lookup_page_table(): {}", reason);
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        }
    }

    fn lookup_kernel_page_table(
        &mut self,
        pde: &PageDirectoryEntry,
    ) -> Result<Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>, Error> {
        // Check if corresponding page table does not exist.
        if !pde.is_present() {
            let reason: &str = "page table not present";
            error!("lookup_kernel_page_table(): {reason:?} (pde={pde:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        // Get corresponding page table.
        let pgtab_addr: FrameAddress = FrameAddress::from_frame_number(pde.frame())?;

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
                error!("lookup_kernel_page_table(): {}", reason);
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
        error!("find_page(): {} (vaddr={:?})", reason, vaddr);
        Err(Error::new(ErrorCode::NoSuchEntry, reason))
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
    /// Upon success, empty is returned. Upon failure, an error code is returned instead.
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
            error!("copy_from_user_unaligned(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let copy_from_user_unaligned = |dry_run: bool,
                                        mut src: VirtualAddress,
                                        mut dst: VirtualAddress,
                                        mut size: usize|
         -> Result<(), Error> {
            while size > 0 {
                // Check if source address does not lie in user space.
                if !Self::is_user_addr(src) {
                    let reason: &str = "source address does not lie in user space";
                    error!("copy_from_user_unaligned(): {}", reason);
                    return Err(Error::new(ErrorCode::BadAddress, reason));
                }

                // Check if destination address does not lie in kernel space.
                if !Self::is_kernel_addr(dst) {
                    let reason: &str = "destination address does not lie in kernel space";
                    error!("copy_from_user_unaligned(): {}", reason);
                    return Err(Error::new(ErrorCode::BadAddress, reason));
                }

                let vaddr: PageAligned<VirtualAddress> =
                    PageAligned::from_address(src.align_down(PAGE_ALIGNMENT))?;
                let offset: usize = src.into_raw_value() - vaddr.into_raw_value();
                let copy_size: usize = usize::min(size, mem::PAGE_SIZE - offset);

                let src_frame: FrameAddress = self.find_user_frame(vaddr)?;

                // Check if end source address does not lie in user space.
                if !Self::is_user_addr(VirtualAddress::new(src.into_raw_value() + copy_size - 1)) {
                    let reason: &str = "end source address does not lie in user space";
                    error!(
                        "copy_from_user_unaligned(): {} (dst={:?}, src={:?}, size={:?})",
                        reason, dst, src, size
                    );
                    return Err(Error::new(ErrorCode::BadAddress, reason));
                }

                // Check if end destination address does not lie in kernel space.
                if !Self::is_kernel_addr(VirtualAddress::new(dst.into_raw_value() + copy_size - 1))
                {
                    let reason: &str = "end destination address does not lie in kernel space";
                    error!(
                        "copy_from_user_unaligned(): {} (dst={:?}, src={:?}, size={:?})",
                        reason, dst, src, size
                    );
                    return Err(Error::new(ErrorCode::BadAddress, reason));
                }

                // Check if we are not in dry-run mode.
                if !dry_run {
                    // Copy data.
                    unsafe {
                        __phys_memcpy(
                            dst.into_raw_value() as *mut u8,
                            (src_frame.into_raw_value() + offset) as *const u8,
                            copy_size,
                        )
                    };
                }

                size -= copy_size;
                src = VirtualAddress::new(src.into_raw_value() + copy_size);
                dst = VirtualAddress::new(dst.into_raw_value() + copy_size);
            }

            Ok(())
        };

        // Run in dry-run mode first to check for errors.
        copy_from_user_unaligned(true, src, dst, size)?;
        // Run in normal mode to effectively copy data.
        copy_from_user_unaligned(false, src, dst, size)?;

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
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Upon failure, an error code is returned instead.
    ///
    pub fn copy_to_user_unaligned(
        &self,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        // Check if size is invalid.
        if size == 0 {
            let reason: &str = "zero-length copy";
            error!("copy_to_user_unaligned(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let _copy_to_user_unaligned = |dry_run: bool,
                                       mut dst: VirtualAddress,
                                       mut src: VirtualAddress,
                                       mut size: usize|
         -> Result<(), Error> {
            while size > 0 {
                // Checks if start source address does not lie in kernel space.
                if !Self::is_kernel_addr(src) {
                    let reason: &str = "start source address does not lie in kernel space";
                    error!(
                        "copy_to_user_unaligned(): {} (dst={:?}, src={:?}, size={:?})",
                        reason, dst, src, size
                    );
                    return Err(Error::new(ErrorCode::BadAddress, reason));
                }

                // Check if start destination address does not lie in user space.
                if !Self::is_user_addr(dst) {
                    let reason: &str = "start destination address does not lie in user space";
                    error!(
                        "copy_to_user_unaligned(): {} (dst={:?}, src={:?}, size={:?})",
                        reason, dst, src, size
                    );
                    return Err(Error::new(ErrorCode::BadAddress, reason));
                }

                let vaddr: PageAligned<VirtualAddress> =
                    PageAligned::from_address(dst.align_down(PAGE_ALIGNMENT))?;

                let offset: usize = dst.into_raw_value() - vaddr.into_raw_value();
                let copy_size: usize = usize::min(mem::PAGE_SIZE - offset, size);

                // Check if end source address does not lie in kernel space.
                if !Self::is_kernel_addr(VirtualAddress::new(src.into_raw_value() + copy_size - 1))
                {
                    let reason: &str = "end source address does not lie in kernel space";
                    error!(
                        "copy_to_user_unaligned(): {} (dst={:?}, src={:?}, size={:?})",
                        reason, dst, src, size
                    );
                    return Err(Error::new(ErrorCode::BadAddress, reason));
                }

                // Check if end destination address does not lie in user space.
                if !Self::is_user_addr(VirtualAddress::new(dst.into_raw_value() + copy_size - 1)) {
                    let reason: &str = "end destination address does not lie in user space";
                    error!(
                        "copy_to_user_unaligned(): {} (dst={:?}, src={:?}, size={:?})",
                        reason, dst, src, size
                    );
                    return Err(Error::new(ErrorCode::BadAddress, reason));
                }

                let dst_frame: FrameAddress = self.find_user_frame(vaddr)?;

                // Check if we are not in dry-run mode.
                if !dry_run {
                    // Copy data.
                    unsafe {
                        __phys_memcpy(
                            (dst_frame.into_raw_value() + offset) as *mut u8,
                            src.into_raw_value() as *const u8,
                            copy_size,
                        )
                    };
                }

                size -= copy_size;
                dst = VirtualAddress::new(dst.into_raw_value() + copy_size);
                src = VirtualAddress::new(src.into_raw_value() + copy_size);
            }
            Ok(())
        };

        // Run in dry-run mode first to check for errors.
        _copy_to_user_unaligned(true, dst, src, size)?;
        // Run in normal mode to effectively copy data.
        _copy_to_user_unaligned(false, dst, src, size)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Copies data from one page to another in the target virtual address space.
    ///
    /// # Parameters
    ///
    /// - `dst`: Virtual address of the destination page.
    /// - `src`: Virtual address of the source page.
    /// - `size`: Number of bytes to copy.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Upon failure, an error code is returned instead.
    ///
    pub fn memcpy(
        &mut self,
        dst: PageAligned<VirtualAddress>,
        src: PageAligned<PhysicalAddress>,
        size: usize,
    ) -> Result<(), Error> {
        // Get corresponding user page.
        let uframe: FrameAddress = self.find_user_frame(dst)?;
        let dst: PageAligned<PhysicalAddress> = uframe.into_physical_address();
        let dst: usize = dst.into_raw_value();
        let src: usize = src.into_raw_value();

        // Check if memory regions overlap.
        if Self::overlap(dst, size, src, size) {
            let reason: &str = "memory regions overlap";
            error!("memcpy(): {} (dst={:?}, src={:?}, size={:?})", reason, dst, src, size);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Safety: `dst` and `src` point to valid memory locations and `mem::PAGE_SIZE` bytes are
        // readable from `src` and writable to `dst`.
        unsafe {
            __phys_memcpy(dst as *mut u8, src as *const u8, size);
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

        // Safety: `base` points to a valid memory location and `mem::PAGE_SIZE` bytes are
        // writable.
        unsafe {
            __phys_memset(base, value as u8, mem::PAGE_SIZE);
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps a page from the target virtual address space.
    ///
    /// # Parameters
    ///
    /// - `vaddr`: Virtual address of the target page.
    ///
    /// # Returns
    ///
    /// Upon success, the user frame that was unmapped is returned. Upon failure, an error code is
    /// returned instead.
    ///
    pub fn unmap(&mut self, vaddr: PageAligned<VirtualAddress>) -> Result<UserFrame, Error> {
        // Check if the provided address lies outside the user space.
        if !Self::is_user_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in user space";
            error!("unmap(): {}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Find the corresponding frame address.
        let frame_address: FrameAddress = self.find_user_frame(vaddr)?;

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
                        error!("map(): {}", reason);
                        return Err(Error::new(ErrorCode::TryAgain, reason));
                    },
                };

                // Get corresponding page table.
                // Check if corresponding page table does not exist.
                if !pde.is_present() {
                    let reason: &str = "page table not present";
                    error!("unmap(): {}", reason);
                    return Err(Error::new(ErrorCode::NoSuchEntry, reason));
                };

                (pgtable_vaddr, self.lookup_page_table(&pde)?)
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

        Ok(UserFrame::new(frame_address))
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
            error!("ctrl(): {}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Get corresponding page table.
        let page_table: &mut PageTable<PageTableStorage> = {
            let vaddr: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
                ::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),
            )?;
            // Get the corresponding page directory entry.
            let pde: PageDirectoryEntry = match self.pgdir.read_pde(PageTableAddress::new(vaddr)) {
                Some(pde) => pde,
                None => {
                    let reason: &str = "failed to read page directory entry";
                    error!("ctrl(): {}", reason);
                    return Err(Error::new(ErrorCode::TryAgain, reason));
                },
            };

            // Check if corresponding page table does not exist.
            if !pde.is_present() {
                let reason: &str = "page table not present";
                error!("ctrl(): {}", reason);
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            };

            self.lookup_page_table(&pde)?
        };

        let page_address: PageAddress = PageAddress::new(vaddr);

        // Change access permissions on the page.
        page_table.ctrl(false, page_address, access)?;

        Ok(())
    }

    // Changes access permissions on a kernel page.
    pub fn kctrl(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        trace!("kctrl(): {:?}", vaddr);

        // Check if the provided address lies outside the kernel space.
        if !Self::is_kernel_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in kernel space";
            error!("kctrl(): {}", reason);
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
                    error!("ctrl(): {}", reason);
                    return Err(Error::new(ErrorCode::TryAgain, reason));
                },
            };

            // Check if corresponding page table does not exist.
            if !pde.is_present() {
                let reason: &str = "page table not present";
                error!("ctrl(): {}", reason);
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            };

            self.lookup_kernel_page_table(&pde)?
        };

        let page_address: PageAddress = PageAddress::new(vaddr);

        // Change access permissions on the page.
        page_table
            .borrow_mut()
            .1
            .ctrl(false, page_address, access)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Tests if a memory region overlaps with another.
    ///
    /// # Parameters.
    ///
    /// - `base1`: Base address of the first memory region.
    /// - `size1`: Size of the first memory region.
    /// - `base2`: Base address of the second memory region.
    /// - `size2`: Size of the second memory region.
    ///
    /// # Returns
    ///
    /// If the memory regions overlap, `true` is returned. Otherwise, `false` is returned.
    ///
    fn overlap(base1: usize, size1: usize, base2: usize, size2: usize) -> bool {
        let end1: usize = base1 + size1;
        let end2: usize = base2 + size2;

        // Check if one region is to the left of the other.
        if base1 >= end2 || base2 >= end1 {
            return false;
        }

        true
    }
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
