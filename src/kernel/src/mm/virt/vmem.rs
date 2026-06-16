// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::x86::mem::mmu::PageMap,
        mem::{
            AccessPermission,
            Address,
            FrameAddress,
            PageAligned,
            VirtualAddress,
        },
    },
    mm::{
        phys::{
            PhysMemoryManager,
            UserFrame,
        },
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
    paging::PageTableEntry,
    PAGE_ALIGNMENT,
};
use ::config::kernel::MEMORY_SIZE;
use ::core::{
    cell::RefCell,
    fmt,
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

//==================================================================================================
// Constants
//==================================================================================================

// TODO: `USER_BASE` should be aligned to a page boundary.

// TODO: `USER_BASE` should be aligned to a page table boundary.

//==================================================================================================
// Virtual Memory Space
//==================================================================================================

///
/// # Description
///
/// A type that represents a virtual memory space.
///
/// This is the architecture-independent virtual memory abstraction. All page table walk logic
/// is delegated to [`PageMap`], which is provided by the architecture layer — following
/// the Linux kernel's approach where the generic MM layer is agnostic to page table levels
/// and entry formats.
///
pub struct Vmem {
    /// Architecture-specific page table hierarchy (e.g., 2-level on x86, 4-level on x86_64).
    page_map: PageMap,
    /// List of kernel pages mapped in the virtual address space.
    /// NOTE: this currently excludes kernel pages that are identity mapped.
    kernel_pages: LinkedList<Rc<RefCell<KernelPage>>>,
    /// List of private kernel pages.
    private_kernel_pages: LinkedList<KernelPage>,
}

impl fmt::Debug for Vmem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cr3: usize = self.page_map.cr3_value().unwrap_or(0);
        f.debug_struct("Vmem")
            .field("cr3", &format_args!("{:#x}", cr3))
            .finish()
    }
}

impl Vmem {
    /// Initializes a new virtual memory space (root address space, boot path).
    pub fn new(
        mut kernel_pages: LinkedList<KernelPage>,
        kernel_page_tables: LinkedList<(
            crate::hal::mem::PageTableAligned<crate::hal::mem::VirtualAddress>,
            PageTableStorage,
        )>,
    ) -> Result<Self, Error> {
        trace!("kernel_pages.len()={}", kernel_pages.len());

        let page_map: PageMap = PageMap::new_boot(kernel_page_tables)?;

        // Store root pages.
        let mut kpages: LinkedList<Rc<RefCell<KernelPage>>> = LinkedList::new();
        while let Some(entry) = kernel_pages.pop_front() {
            kpages.push_back(Rc::new(RefCell::new(entry)));
        }

        Ok(Self {
            page_map,
            kernel_pages: kpages,
            private_kernel_pages: LinkedList::new(),
        })
    }

    /// Clones the target virtual memory space for a new process.
    pub fn clone(from: &Vmem, pages: LinkedList<KernelPage>) -> Result<Vmem, Error> {
        let page_map: PageMap = PageMap::new_clone(&from.page_map, pages)?;

        // Share root pages.
        let mut kernel_pages: LinkedList<Rc<RefCell<KernelPage>>> = LinkedList::new();
        for entry in from.kernel_pages.iter() {
            kernel_pages.push_back(entry.clone());
        }

        Ok(Self {
            page_map,
            kernel_pages,
            private_kernel_pages: LinkedList::new(),
        })
    }

    pub fn load(&self) -> Result<(), Error> {
        // SAFETY: PageMap manages a valid page table hierarchy.
        unsafe { self.page_map.load() }
    }

    /// Returns the value to be loaded into CR3 for this address space.
    pub fn cr3_value(&self) -> Result<usize, Error> {
        self.page_map.cr3_value()
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
    /// Allocates backing storage for a new page table from the kernel page pool.
    fn pt_allocator() -> Result<PageTableStorage, Error> {
        // SAFETY: the memory manager is initialized and access is synchronized (single-threaded,
        // interrupts disabled). The manager is a stateless coordinator, so this does not alias
        // any `Vmem` being mapped.
        let kpage: KernelPage = unsafe { super::VirtMemoryManager::get_mut() }.alloc_kpage(true)?;
        Ok(PageTableStorage::KernelPage(kpage))
    }

    pub fn map_kpage(
        &mut self,
        kpage: KernelPage,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        self.page_map
            .map_kernel_page(kpage.frame_address(), vaddr, Self::pt_allocator)?;

        // Track the kernel page for reference counting.
        self.kernel_pages.push_back(Rc::new(RefCell::new(kpage)));

        // Reload page map to force a TLB flush.
        self.load()?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Maps an MMIO page into the kernel address space.
    ///
    /// Unlike [`map_kpage`], this maps a caller-specified physical frame (typically a
    /// device MMIO address) rather than a frame from the kernel page pool.
    ///
    /// # Parameters
    ///
    /// - `frame`: Physical frame address of the MMIO region.
    /// - `vaddr`: Virtual address to map to.
    /// - `page_table_allocator`: Allocator for page table storage if a new page table is needed.
    ///
    pub fn map_mmio_page(
        &mut self,
        frame: FrameAddress,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        self.page_map
            .map_kernel_page(frame, vaddr, Self::pt_allocator)?;

        // NOTE: no load() here — the invlpg inside map_page() already invalidates
        // the TLB entry for vaddr. A full CR3 reload is unnecessary and on x86
        // re-triggers paging enable (CR0.PG), which can crash if intermediate
        // paging state is inconsistent.

        Ok(())
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

        self.page_map
            .map_user_page(vaddr, uframe.address(), access, Self::pt_allocator)?;

        // Transfer ownership of the frame's reference to the page table: the mapping now holds
        // the reference, so suppress the handle's Drop (which would otherwise free the frame).
        let _ = uframe.leak();

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
        let frame: FrameAddress = self.page_map.lookup_user_page(page_aligned)?;
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

        if !Self::is_user_region(src, size) {
            let reason: &str = "source memory region does not lie entirely in user space";
            error!(
                "copy_from_user_unaligned(): {reason} (dst={dst:?}, src={src:?}, size={size:?})"
            );
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

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

                let src_frame: FrameAddress = self.page_map.lookup_user_page(vaddr)?;

                if !dry_run {
                    unsafe {
                        self.page_map.copy_from_user(
                            dst.into_raw_value() as *mut u8,
                            src.into_raw_value(),
                            src_frame.into_raw_value() + offset,
                            copy_size,
                        );
                    }
                }

                size -= copy_size;
                src = VirtualAddress::new(src.into_raw_value() + copy_size);
                dst = VirtualAddress::new(dst.into_raw_value() + copy_size);
            }

            Ok(())
        };

        copy_from_user_unaligned_impl(true, src, dst, size)?;
        copy_from_user_unaligned_impl(false, src, dst, size)?;

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
    /// - [`ErrorCode::BadAddress`]: The destination memory region does not lie within physical
    ///   memory.
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
        if size == 0 {
            let reason: &str = "zero-length copy";
            error!(
                "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, src={src:?}, \
                 size={size:?})"
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        if !Self::is_kernel_region(src, size) {
            let reason: &str = "source memory region does not lie entirely in kernel space";
            error!(
                "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, src={src:?}, \
                 size={size:?})",
            );
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

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

            if !dry_run {
                let dst_frame: FrameAddress = match self.page_map.lookup_user_page(vaddr) {
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
                if !Self::is_physical_region(dst_phys_addr_raw, copy_size) {
                    let reason: &str =
                        "destination memory region does not lie within physical memory";
                    panic!(
                        "copy_to_user_unaligned_unchecked(): {reason} (dst={dst:?}, src={src:?}, \
                         size={size:?})"
                    );
                }

                unsafe {
                    self.page_map.copy_to_user(
                        dst.into_raw_value(),
                        dst_phys_addr_raw,
                        src.into_raw_value() as *const u8,
                        copy_size,
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
    /// Copies data from kernel space to user space.
    ///
    /// Unlike [`Self::copy_to_user_unaligned_unchecked`], this function performs a dry run first to
    /// check for errors before performing the actual copy.
    ///
    pub fn copy_to_user_unaligned(
        &self,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        self.copy_to_user_unaligned_unchecked(dst, src, size, true)?;
        self.copy_to_user_unaligned_unchecked(dst, src, size, false)
    }

    ///
    /// # Description
    ///
    /// Copies data directly between the user spaces of two processes.
    ///
    pub fn copy_user_to_user(
        src_vmem: &Vmem,
        src: VirtualAddress,
        dst_vmem: &Vmem,
        dst: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        if size == 0 {
            let reason: &str = "zero-length copy";
            error!("copy_user_to_user(): {reason} (src={src:?}, dst={dst:?}, size={size:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        if !Self::is_user_region(src, size) {
            let reason: &str = "source memory region does not lie entirely in user space";
            error!("copy_user_to_user(): {reason} (src={src:?}, dst={dst:?}, size={size:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        if !Self::is_user_region(dst, size) {
            let reason: &str = "destination memory region does not lie entirely in user space";
            error!("copy_user_to_user(): {reason} (src={src:?}, dst={dst:?}, size={size:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Dry-run pass: validate that all source and destination pages are present.
        {
            let mut cur_src = src;
            let mut cur_dst = dst;
            let mut remaining = size;
            while remaining > 0 {
                let src_page: PageAligned<VirtualAddress> =
                    PageAligned::from_address(cur_src.align_down(PAGE_ALIGNMENT))?;
                let src_offset: usize = cur_src.into_raw_value() - src_page.into_raw_value();
                let src_avail: usize = mem::PAGE_SIZE - src_offset;

                let dst_page: PageAligned<VirtualAddress> =
                    PageAligned::from_address(cur_dst.align_down(PAGE_ALIGNMENT))?;
                let dst_offset: usize = cur_dst.into_raw_value() - dst_page.into_raw_value();
                let dst_avail: usize = mem::PAGE_SIZE - dst_offset;

                let copy_size: usize = remaining.min(src_avail).min(dst_avail);

                let _src_frame: FrameAddress = src_vmem.page_map.lookup_user_page(src_page)?;
                let _dst_frame: FrameAddress = dst_vmem.page_map.lookup_user_page(dst_page)?;

                remaining -= copy_size;
                cur_src = VirtualAddress::new(cur_src.into_raw_value() + copy_size);
                cur_dst = VirtualAddress::new(cur_dst.into_raw_value() + copy_size);
            }
        }

        // Actual copy pass.
        Self::copy_user_to_user_impl(src_vmem, src, dst_vmem, dst, size)?;

        Ok(())
    }

    /// Fills a page with a given value in the target virtual address space.
    pub fn memset(&mut self, dst: PageAligned<VirtualAddress>, value: u32) -> Result<(), Error> {
        // Get corresponding user page.
        let uframe: FrameAddress = self.page_map.lookup_user_page(dst)?;
        let base: *mut u8 = uframe.into_raw_value() as *mut u8;

        // Safety:
        // - `base` is obtained from `lookup_user_page()`, which resolves a mapped user page,
        //   so it points to a valid, writable physical memory location.
        // - `base` is page-aligned, which satisfies the 4-byte alignment requirement.
        // - `mem::PAGE_SIZE` is a multiple of 4 bytes, satisfying the size requirement.
        super::identity_map::phys_memset32(base, value as u8, mem::PAGE_SIZE)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps a page from the target virtual address space.
    ///
    /// If the page is not present (e.g., was never demand-paged), `Ok(None)` is returned without
    /// logging any errors.
    ///
    pub fn unmap(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<UserFrame>, Error> {
        if !Self::is_user_addr(vaddr.into_inner()) {
            let reason: &str = "address is not in user space";
            error!("{reason}");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        match self.page_map.unmap_user_page(vaddr)? {
            Some(frame_address) => Ok(Some(UserFrame::new(frame_address))),
            None => Ok(None),
        }
    }

    /// Checks whether a user page is currently mapped.
    pub fn is_user_page_mapped(&self, vaddr: PageAligned<VirtualAddress>) -> Result<bool, Error> {
        Ok(self.page_map.try_lookup_user_page(vaddr)?.is_some())
    }

    /// Changes access permissions on a user page.
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

        self.page_map.ctrl_user_page(vaddr, access)
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

        self.page_map.ctrl_kernel_page(vaddr, access, dry_run)
    }

    /// Performs the actual data copy between two user address spaces.
    ///
    /// Both source and destination frame addresses are physical addresses obtained from page
    /// table lookups. The copy is performed in the kernel address space, which has identity-
    /// mapped physical memory. A temporary CR3 switch provides access to the identity mapping.
    fn copy_user_to_user_impl(
        src_vmem: &Vmem,
        src: VirtualAddress,
        dst_vmem: &Vmem,
        dst: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        let mut cur_src = src;
        let mut cur_dst = dst;
        let mut remaining = size;
        while remaining > 0 {
            let src_page: PageAligned<VirtualAddress> =
                PageAligned::from_address(cur_src.align_down(PAGE_ALIGNMENT))?;
            let src_offset: usize = cur_src.into_raw_value() - src_page.into_raw_value();
            let src_avail: usize = mem::PAGE_SIZE - src_offset;

            let dst_page: PageAligned<VirtualAddress> =
                PageAligned::from_address(cur_dst.align_down(PAGE_ALIGNMENT))?;
            let dst_offset: usize = cur_dst.into_raw_value() - dst_page.into_raw_value();
            let dst_avail: usize = mem::PAGE_SIZE - dst_offset;

            let copy_size: usize = remaining.min(src_avail).min(dst_avail);

            let src_frame: FrameAddress = src_vmem.page_map.lookup_user_page(src_page)?;
            let dst_frame: FrameAddress = dst_vmem.page_map.lookup_user_page(dst_page)?;

            // The wrapper switches to the kernel address space and ensures identity
            // mappings for the full source and destination ranges before copying.
            let src_phys_addr: usize = src_frame.into_raw_value() + src_offset;
            let dst_phys_addr: usize = dst_frame.into_raw_value() + dst_offset;
            let dst: *mut u8 = dst_phys_addr as *mut u8;
            let src: *const u8 = src_phys_addr as *const u8;
            let word_size: usize = ::core::mem::size_of::<u32>();
            let copy_result: Result<(), Error> = if copy_size.is_multiple_of(word_size)
                && dst_phys_addr.is_multiple_of(word_size)
                && src_phys_addr.is_multiple_of(word_size)
            {
                super::identity_map::phys_memcpy32(dst, src, copy_size)
            } else {
                super::identity_map::phys_memcpy(dst, src, copy_size)
            };
            copy_result?;

            remaining -= copy_size;
            cur_src = VirtualAddress::new(cur_src.into_raw_value() + copy_size);
            cur_dst = VirtualAddress::new(cur_dst.into_raw_value() + copy_size);
        }
        Ok(())
    }

    //==============================================================================================
    // Copy-on-Write
    //==============================================================================================

    /// Reads the user page-table entry for `vaddr` (None if not mapped/present).
    pub(crate) fn try_find_user_pte(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<PageTableEntry>, Error> {
        self.page_map.try_lookup_user_pte(vaddr)
    }

    /// Marks the user page at `vaddr` as copy-on-write (clears writable, sets the CoW bit).
    pub fn mark_user_page_cow(&mut self, vaddr: PageAligned<VirtualAddress>) -> Result<(), Error> {
        self.page_map.mark_user_page_cow(vaddr)
    }

    /// Clears the copy-on-write mark on the user page at `vaddr` and restores writability.
    pub fn unmark_user_page_cow(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        self.page_map.unmark_user_page_cow(vaddr)
    }

    /// Repoints the copy-on-write page at `vaddr` to `new_frame`; returns the old frame.
    fn replace_user_page_cow_frame(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        new_frame: FrameAddress,
    ) -> Result<FrameAddress, Error> {
        self.page_map.replace_user_page_cow_frame(vaddr, new_frame)
    }

    /// Resolves a copy-on-write mapping at `vaddr`, copying the shared frame if needed.
    ///
    /// Returns `Ok(true)` if a copy-on-write mapping was found and resolved, `Ok(false)` if the
    /// address is not a copy-on-write user mapping.
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

        // Fast path: if this address space holds the last reference to the shared frame, clear
        // the copy-on-write mark in place. Safe because the kernel is single-threaded and runs
        // with interrupts disabled, so the refcount cannot change between query and unmark.
        let src_frame: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;
        let probe: ManuallyDrop<UserFrame> = ManuallyDrop::new(UserFrame::new(src_frame));
        if probe.refcount()? == 1 {
            self.unmark_user_page_cow(vaddr)?;
            return Ok(true);
        }

        // SAFETY: the kernel is single-threaded and runs with interrupts disabled; no concurrent
        // or re-entrant access to the physical memory manager is possible.
        let new_frame: UserFrame = unsafe { PhysMemoryManager::get_mut() }.alloc_user_frame()?;

        let src_paddr: usize = pte.frame_address();
        let dst_paddr: usize = new_frame.address().into_raw_value();
        super::memcpy(dst_paddr as *mut u8, src_paddr as *const u8, mem::PAGE_SIZE)?;

        let new_frame_addr: FrameAddress = new_frame.address();
        let old_frame: FrameAddress = self.replace_user_page_cow_frame(vaddr, new_frame_addr)?;

        // The new frame is now owned by the page table; suppress its Drop.
        let _ = new_frame.leak();

        // Drop the shared reference: frees the frame only when the last sharer releases it.
        drop(UserFrame::new(old_frame));

        Ok(true)
    }

    /// Eagerly resolves all copy-on-write mappings overlapping `[addr, addr + size)`.
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

    //==============================================================================================
    // User-Mapping Iteration
    //==============================================================================================

    /// Invokes `f(vaddr, pte)` for every present user mapping.
    pub fn for_each_user_mapping<F>(&self, mut f: F) -> Result<(), Error>
    where
        F: FnMut(PageAligned<VirtualAddress>, PageTableEntry) -> Result<(), Error>,
    {
        self.page_map.try_for_each_user_mapping(|vaddr, pte| {
            f(vaddr, pte)?;
            Ok(ControlFlow::Continue(()))
        })
    }

    /// Like [`Self::for_each_user_mapping`], but the callback may stop early via
    /// [`ControlFlow::Break`].
    pub(crate) fn try_for_each_user_mapping<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnMut(PageAligned<VirtualAddress>, PageTableEntry) -> Result<ControlFlow<()>, Error>,
    {
        self.page_map.try_for_each_user_mapping(f)
    }

    /// Unmaps every present user-space page, freeing the underlying frames.
    pub fn clear_user_space(&mut self) -> Result<(), Error> {
        // Number of mappings unmapped per pass; bounds an on-stack scratch buffer so the routine
        // performs no heap allocation. `unmap` needs `&mut self` whereas the iteration borrows
        // `&self`, so each pass snapshots up to `CHUNK` addresses, then unmaps them.
        const CHUNK: usize = 32;

        loop {
            let mut buf: [MaybeUninit<PageAligned<VirtualAddress>>; CHUNK] =
                [const { MaybeUninit::uninit() }; CHUNK];
            let mut count: usize = 0;
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

            for slot in buf.iter().take(count) {
                // SAFETY: indices `< count` were initialized by the scan above.
                let vaddr: PageAligned<VirtualAddress> = unsafe { slot.assume_init_read() };
                let _uframe: Option<UserFrame> = self.unmap(vaddr)?;
            }

            if count < CHUNK {
                return Ok(());
            }
        }
    }
}

impl Drop for Vmem {
    fn drop(&mut self) {
        // Unmap all kernel private kernel pages.
        while let Some(kpage) = self.private_kernel_pages.pop_front() {
            drop(kpage)
        }

        // Unmap shared kernel pages.
        while let Some(entry) = self.kernel_pages.pop_front() {
            drop(entry);
        }

        // PageMap drop handles page table cleanup.
    }
}
