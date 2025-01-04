// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
            page_table::{
                PageTable,
                PageTableStorage,
            },
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
            upage::AttachedUserPage,
        },
    },
};
use ::alloc::{
    collections::{
        btree_map::BTreeMap,
        LinkedList,
    },
    rc::Rc,
};
use ::core::cell::RefCell;
use ::sys::{
    arch::mem::{
        self,
        paging::PageDirectoryEntry,
    },
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

extern "C" {
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
    kernel_page_tables: LinkedList<Rc<RefCell<(PageTableAddress, PageTable)>>>,
    /// List of kernel pages mapped in the virtual address space.
    /// NOTE: this currently excludes kernel pages that are identity mapped.
    kernel_pages: LinkedList<Rc<RefCell<KernelPage>>>,
    /// List of private kernel pages.
    private_kernel_pages: LinkedList<KernelPage>,
    /// List of user page tables.
    user_page_tables: LinkedList<(PageTableAddress, PageTable)>,
    //// User pages in the virtual address space.
    user_pages: BTreeMap<PageTableAddress, LinkedList<AttachedUserPage>>,
}

impl Vmem {
    /// Initializes a new virtual memory space.
    pub fn new(
        mut kernel_pages: LinkedList<KernelPage>,
        mut kernel_page_tables: LinkedList<(PageTableAddress, PageTable)>,
    ) -> Result<Self, Error> {
        trace!("new()");

        // Create a clean page directory.
        let mut pgdir: PageDirectory = PageDirectory::new(PageDirectoryStorage::new());
        pgdir.clean();

        // Map and store root page tables.
        let mut kpage_tables: LinkedList<Rc<RefCell<(PageTableAddress, PageTable)>>> =
            LinkedList::new();
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
            user_pages: BTreeMap::new(),
        })
    }

    /// Clones the target virtual memory space.
    pub fn clone(from: &Vmem) -> Result<Vmem, Error> {
        // Create a clean page directory.
        let mut pgdir: PageDirectory = PageDirectory::new(PageDirectoryStorage::new());
        pgdir.clean();

        // Map and store root page tables.
        let mut kernel_page_tables: LinkedList<Rc<RefCell<(PageTableAddress, PageTable)>>> =
            LinkedList::new();
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
            user_pages: BTreeMap::new(),
        })
    }

    /// Adds a kernel page to the private list of the kernel pages in the target virtual memory space.
    pub fn add_private_kernel_page(&mut self, kpage: KernelPage) {
        self.private_kernel_pages.push_back(kpage);
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
    pub fn map_kpage(
        &mut self,
        kpage: KernelPage,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        let pt_vaddr: PageTableAddress = PageTableAddress::new(PageTableAligned::from_raw_value(
            ::sys::mm::align_down(vaddr.into_raw_value(), mmu::PGTAB_ALIGNMENT),
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
            let pgtable_storage: PageTableStorage = PageTableStorage::new();
            let page_table: PageTable = PageTable::new(pgtable_storage);

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
    pub fn map(
        &mut self,
        uframe: UserFrame,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        // Check if the provided address lies outside the user space.
        if !Self::is_user_addr(vaddr.into_inner()) {
            return Err(Error::new(ErrorCode::BadAddress, "address is not in user space"));
        }

        // Get corresponding page table.
        let (pgtable_addr, page_table): (PageTableAddress, &mut PageTable) = {
            let vaddr: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
                ::sys::mm::align_down(vaddr.into_raw_value(), mmu::PGTAB_ALIGNMENT),
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
                let pgtable_storage: PageTableStorage = PageTableStorage::new();
                let page_table: PageTable = PageTable::new(pgtable_storage);

                let page_table_address: FrameAddress = page_table.physical_address()?;
                // FIXME: do not be so open about permissions.
                self.pgdir
                    .map(pgtable_vaddr, page_table_address, false, AccessPermission::RDWR)?;

                //===================================================================
                // NOTE: if we fail beyond this point we should unmap the page table.
                //===================================================================

                self.user_page_tables.push_back((pgtable_vaddr, page_table));
                self.user_pages.insert(pgtable_vaddr, LinkedList::new());

                // Get the corresponding page directory entry.
                pde = match self.pgdir.read_pde(PageTableAddress::new(vaddr)) {
                    Some(pde) => pde,
                    None => unreachable!("failed to read page directory entry"),
                };
            };

            (pgtable_vaddr, self.lookup_page_table(&pde)?)
        };

        // Map the page to the target virtual address space.
        page_table.map(PageAddress::new(vaddr), uframe.address(), false, false, true, access)?;

        //=============================================================
        // NOTE: if we fail beyond this point we should unmap the page.
        //=============================================================

        self.user_pages
            .get_mut(&pgtable_addr)
            .expect("page table should be mapped")
            .push_back(AttachedUserPage::new(PageAddress::new(vaddr), uframe));

        Ok(())
    }

    /// Asserts whether an address lies in the user space.
    fn is_user_addr(virt_addr: VirtualAddress) -> bool {
        virt_addr >= config::memory_layout::USER_BASE && virt_addr < config::memory_layout::USER_END
    }

    /// Asserts wether an address lies in the kernel space.
    fn is_kernel_addr(virt_addr: VirtualAddress) -> bool {
        !Self::is_user_addr(virt_addr)
    }

    /// Looks up a page table in the list of page tables.
    fn lookup_page_table(&mut self, pde: &PageDirectoryEntry) -> Result<&mut PageTable, Error> {
        // Check if corresponding page table does not exist.
        if !pde.is_present() {
            return Err(Error::new(ErrorCode::NoSuchEntry, "page table not present"));
        }

        // Get corresponding page table.
        let pgtab_addr: FrameAddress = FrameAddress::from_frame_number(pde.frame())?;

        // Find corresponding page table.
        let mut page_table: Option<&mut PageTable> = None;
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
    ) -> Result<Rc<RefCell<(PageTableAddress, PageTable)>>, Error> {
        // Check if corresponding page table does not exist.
        if !pde.is_present() {
            return Err(Error::new(ErrorCode::NoSuchEntry, "page table not present"));
        }

        // Get corresponding page table.
        let pgtab_addr: FrameAddress = FrameAddress::from_frame_number(pde.frame())?;

        // Find corresponding page table.
        let mut page_table: Option<Rc<RefCell<(PageTableAddress, PageTable)>>> = None;
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
    /// Finds a user page in the target virtual memory space.
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
    pub fn find_page(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<&AttachedUserPage, Error> {
        let pgtable_addr: PageTableAddress =
            PageTableAddress::new(PageTableAligned::from_raw_value(::sys::mm::align_down(
                vaddr.into_raw_value(),
                mmu::PGTAB_ALIGNMENT,
            ))?);

        match self.user_pages.get(&pgtable_addr) {
            Some(pages) => {
                for page in pages.iter() {
                    if page.vaddr().into_virtual_address() == vaddr {
                        return Ok(page);
                    }
                }

                let reason: &str = "page not found";
                error!("find_page(): {} (vaddr={:?})", reason, vaddr);
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
            None => {
                let reason: &str = "page table not found";
                error!("find_page(): {}", reason);
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        }
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
                    PageAligned::from_address(src.align_down(mmu::PAGE_ALIGNMENT)?)?;
                let offset: usize = src.into_raw_value() - vaddr.into_raw_value();
                let copy_size: usize = usize::min(size, mem::PAGE_SIZE - offset);

                let src_page: &AttachedUserPage = self.find_page(vaddr)?;
                let src_frame: FrameAddress = src_page.frame_address();

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
                    PageAligned::from_address(dst.align_down(mmu::PAGE_ALIGNMENT)?)?;

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

                let dst_page: &AttachedUserPage = self.find_page(vaddr)?;
                let dst_frame: FrameAddress = dst_page.frame_address();

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
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Upon failure, an error code is returned instead.
    ///
    pub fn memcpy(
        &mut self,
        dst: PageAligned<VirtualAddress>,
        src: PageAligned<PhysicalAddress>,
    ) -> Result<(), Error> {
        // Get corresponding user page.
        let page = self.find_page(dst)?;
        let uframe: FrameAddress = page.frame_address();
        let dst: PageAligned<PhysicalAddress> = uframe.into_physical_address();
        let dst: *mut u8 = dst.into_raw_value() as *mut u8;
        let src: *const u8 = (src.into_raw_value()) as *const u8;

        // Safety: `dst` and `src` point to valid memory locations and `mem::PAGE_SIZE` bytes are
        // readable from `src` and writable to `dst`.
        unsafe {
            __phys_memcpy(dst, src, mem::PAGE_SIZE);
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
        let page: &AttachedUserPage = self.find_page(dst)?;
        let uframe: FrameAddress = page.frame_address();
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
        let frame_addres: FrameAddress = self.find_page(vaddr)?.frame_address();

        let pgtable_vaddr = {
            // Get corresponding page table.
            let (pgtable_vaddr, page_table): (PageTableAddress, &mut PageTable) = {
                let vaddr: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
                    ::sys::mm::align_down(vaddr.into_raw_value(), mmu::PGTAB_ALIGNMENT),
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
            if page_table.lookup(page_address)? != frame_addres {
                // The following statement should not be reachable because after mapping user frame we
                // must have added it to the list of user pages.
                unreachable!("frame address must match what we expect");
            }

            // Unmap the page from the target virtual address space.
            page_table.unmap(page_address)?;

            pgtable_vaddr
        };

        //====================================================================================
        // NOTE: if we fail beyond this point and we want to recover we should remap the page.
        //====================================================================================

        let uframe: UserFrame = match self.user_pages.get_mut(&pgtable_vaddr) {
            Some(pages) => {
                // The following statement should not be reachable because we have checked earlier
                // in this function that the user page was in the list of user pages.
                let at = pages
                    .iter()
                    .position(|page| page.vaddr().into_virtual_address() == vaddr)
                    .expect("page must be in the list of user pages");
                let (_page_addr, uframe): (PageAddress, UserFrame) =
                    pages.remove(at).into_owned_parts();

                if pages.is_empty() {
                    // Remove page table from the list of user page tables.
                    let at = self
                        .user_page_tables
                        .iter()
                        .position(|(addr, _)| addr == &pgtable_vaddr)
                        .expect("page table must be in the list of user page tables");

                    let (_pgtable_addr, _page_table) = self.user_page_tables.remove(at);

                    self.user_pages.remove(&pgtable_vaddr);
                    self.pgdir.unmap(pgtable_vaddr)?;
                }

                uframe
            },
            None => {
                let reason: &str = "page table not found";
                error!("unmap(): {}", reason);
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            },
        };

        Ok(uframe)
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
        let page_table: &mut PageTable = {
            let vaddr: PageTableAligned<VirtualAddress> = PageTableAligned::from_raw_value(
                ::sys::mm::align_down(vaddr.into_raw_value(), mmu::PGTAB_ALIGNMENT),
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
                ::sys::mm::align_down(vaddr.into_raw_value(), mmu::PGTAB_ALIGNMENT),
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
}

impl Drop for Vmem {
    fn drop(&mut self) {
        while let Some((pgtable_vaddr, user_page_table)) = self.user_page_tables.pop_front() {
            while let Some(mut user_pages) = self.user_pages.remove(&pgtable_vaddr) {
                while let Some(user_page) = user_pages.pop_front() {
                    drop(user_page);
                }
            }
            drop(user_page_table);
        }

        assert_eq!(self.user_pages.len(), 0);

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
