// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "x86_64"))]
use crate::hal::arch::x86::mem::mmu::page_table::PageTable;
#[cfg(feature = "x86_64")]
use crate::hal::arch::x86_64::mem::mmu::page_table::PageTable;
use crate::{
    hal::mem::{
        AccessPermission,
        Address,
        PageAligned,
        PageTableAddress,
        VirtualAddress,
    },
    mm::{
        elf,
        phys::{
            KernelFrame,
            PhysMemoryManager,
            UserFrame,
        },
        virt::{
            kpage::KernelPage,
            vmem::Vmem,
            PageTableStorage,
        },
    },
};
use ::alloc::{
    collections::LinkedList,
    rc::Rc,
    vec::Vec,
};
use ::arch::mem;
use ::core::{
    cell::RefCell,
    hint::unlikely,
    mem::MaybeUninit,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

// Use relaxed ordering for all atomic operations to mitigate synchronization overhead. It is safe
// to use this ordering semantics because Nanvix is a single-core system, and the kernel runs with
// interrupts disabled.
const ORDER: Ordering = Ordering::Relaxed;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Memory manager storage.
static mut MEMORY_MANAGER: MaybeUninit<VirtMemoryManager> = MaybeUninit::uninit();

/// Whether the memory manager has been initialized.
static MEMORY_MANAGER_INIT: AtomicBool = AtomicBool::new(false);

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Memory manager.
///
pub struct VirtMemoryManager {
    /// Physical memory manager.
    physman: Rc<RefCell<PhysMemoryManager>>,
}

impl VirtMemoryManager {
    ///
    /// # Description
    ///
    /// Initializes the virtual memory manager.
    ///
    /// # Parameters
    /// - `kernel_pages`: Kernel pages.
    /// - `kernel_page_tables`: Kernel page tables.
    /// - `physman`: Physical memory manager.
    ///
    pub fn init(
        kernel_pages: LinkedList<KernelPage>,
        kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
        physman: PhysMemoryManager,
    ) -> Result<Vmem, Error> {
        // Check if the memory manager is already initialized.
        if unlikely(MEMORY_MANAGER_INIT.load(ORDER)) {
            panic!("memory manager was already initialized");
        }

        let (root, manager): (Vmem, VirtMemoryManager) =
            VirtMemoryManager::new(kernel_pages, kernel_page_tables, physman)?;

        // SAFETY: This happens during kernel initialization and no other threads are running.
        unsafe { MEMORY_MANAGER.write(manager) };
        MEMORY_MANAGER_INIT.store(true, ORDER);

        Ok(root)
    }

    ///
    /// # Description
    ///
    /// Gets a reference to the memory manager.
    ///
    /// # Safety
    ///
    /// This function panics if the memory manager is not initialized.
    ///
    /// This function is unsafe because it operates on a global variable.
    ///
    /// This function is safe to use if and only if all the following conditions are met:
    ///
    /// - Access to the memory manager is synchronized.
    ///
    #[allow(dead_code)] // TODO: remove this lint allowance when the function is used.
    pub unsafe fn get<'a>() -> &'a VirtMemoryManager {
        if unlikely(!MEMORY_MANAGER_INIT.load(ORDER)) {
            panic!("memory manager is not initialized");
        }

        // SAFETY: The memory manager has been initialized, so the value is valid.
        MEMORY_MANAGER.assume_init_ref()
    }

    ///
    /// # Description
    ///
    /// Gets a mutable reference to the memory manager.
    ///
    /// # Safety
    ///
    /// This function panics if the memory manager is not initialized.
    ///
    /// This function is unsafe because it operates on a global variable.
    ///
    /// This function is safe to use if and only if all the following conditions are met:
    ///
    /// - Access to the memory manager is synchronized.
    ///
    pub unsafe fn get_mut<'a>() -> &'a mut VirtMemoryManager {
        if unlikely(!MEMORY_MANAGER_INIT.load(ORDER)) {
            panic!("memory manager is not initialized");
        }

        // SAFETY: The memory manager has been initialized, so the value is valid.
        MEMORY_MANAGER.assume_init_mut()
    }

    ///
    /// # Description
    ///
    /// Instantiates a memory manager.
    ///
    /// # Parameters
    /// - `kernel_pages`: Kernel pages.
    /// - `kernel_page_tables`: Kernel page tables.
    /// - `physman`: Physical memory manager.
    ///
    fn new(
        kernel_pages: LinkedList<KernelPage>,
        kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
        physman: PhysMemoryManager,
    ) -> Result<(Vmem, Self), Error> {
        let root: Vmem = Vmem::new(kernel_pages, kernel_page_tables)?;

        // Load root address space.
        // On x86_64, the VMM's identity mapping is sufficient for boot;
        // full 4-level page table management (PML4/PDPT/PD/PT) is not yet implemented.
        #[cfg(not(feature = "x86_64"))]
        root.load()?;

        Ok((
            root,
            Self {
                physman: Rc::new(RefCell::new(physman)),
            },
        ))
    }

    /// Creates a new virtual address space, based on root.
    pub fn new_vmem(&self, vmem: &Vmem) -> Result<Vmem, Error> {
        let new_vmem: Vmem = Vmem::clone(vmem)?;

        trace!(
            "new_vmem={:?}, old_vmem={:?}",
            new_vmem.pgdir().physical_address(),
            vmem.pgdir().physical_address()
        );

        Ok(new_vmem)
    }

    pub fn alloc_upage(
        &mut self,
        vmem: &mut Vmem,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
        clear: bool,
    ) -> Result<(), Error> {
        // On x86_64, identity mapping via 2MB huge pages means virtual = physical.
        // No need to allocate a separate frame or map in the page directory.
        #[cfg(target_arch = "x86_64")]
        {
            let _ = access; // suppress unused warning
            if clear {
                vmem.memset(vaddr, 0)?;
            }
            return Ok(());
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            let uframe: UserFrame = match self.physman.try_borrow_mut() {
                Ok(mut physman) => physman.alloc_user_frame()?,
                Err(_) => {
                    let reason: &str = "failed to borrow physical memory manager";
                    error!("{reason}");
                    return Err(Error::new(ErrorCode::ResourceBusy, reason));
                },
            };

            let physman: Rc<RefCell<PhysMemoryManager>> = self.physman.clone();
            let page_table_allocator = move || {
                let kframe: KernelFrame = match physman.try_borrow_mut() {
                    Ok(mut physman) => physman.alloc_kernel_frame(true)?,
                    Err(_) => {
                        let reason: &str = "failed to borrow physical memory manager";
                        error!("{reason}");
                        return Err(Error::new(ErrorCode::ResourceBusy, reason));
                    },
                };
                let kpage: KernelPage = KernelPage::new(kframe);
                let pgtable_storage: PageTableStorage = PageTableStorage::KernelPage(kpage);
                let page_table: PageTable<PageTableStorage> = PageTable::new(pgtable_storage);
                Ok(page_table)
            };

            vmem.map(uframe, vaddr, access, &page_table_allocator)?;

            // Check if the page should be cleared.
            if clear {
                // Safety: `vaddr` points to a valid memory location.
                vmem.memset(vaddr, 0)?;
            }

            Ok(())
        }
    }

    ///
    /// # Description
    ///
    /// Unmaps a user page from the target virtual memory space.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory space where the page is mapped.
    /// - `vaddr`: Virtual address of the page to be unmapped.
    ///
    /// # Return Values
    ///
    /// Upon success, empty is returned. Upon failure, an error is returned instead.
    ///
    pub fn unmap_upage(
        &mut self,
        vmem: &mut Vmem,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        let uframe: UserFrame = vmem.unmap(vaddr)?;
        self.physman.borrow_mut().free_user_frame(uframe)
    }

    pub fn alloc_upages(
        &mut self,
        vmem: &mut Vmem,
        mut vaddr: PageAligned<VirtualAddress>,
        nframes: usize,
        access: AccessPermission,
    ) -> Result<(), Error> {
        trace!("vaddr={:?}, nframes={}", vaddr, nframes);

        // On x86_64, identity mapping means virtual = physical. No frame alloc/mapping needed.
        #[cfg(target_arch = "x86_64")]
        {
            let _ = (vmem, access);
            for _ in 0..nframes {
                // Memory is already accessible via identity-mapped 2MB huge pages.
                // Just advance the virtual address.
                vaddr = PageAligned::from_raw_value(vaddr.into_raw_value() + mem::PAGE_SIZE)?;
            }
            return Ok(());
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            let physman: Rc<RefCell<PhysMemoryManager>> = self.physman.clone();

            let page_table_allocator = move || {
                let kframe: KernelFrame = match physman.try_borrow_mut() {
                    Ok(mut physman) => physman.alloc_kernel_frame(true)?,
                    Err(_) => {
                        let reason: &str = "failed to borrow physical memory manager";
                        error!("{reason}");
                        return Err(Error::new(ErrorCode::ResourceBusy, reason));
                    },
                };
                let kpage: KernelPage = KernelPage::new(kframe);
                let pgtable_storage: PageTableStorage = PageTableStorage::KernelPage(kpage);
                let page_table: PageTable<PageTableStorage> = PageTable::new(pgtable_storage);
                Ok(page_table)
            };

            let uframes: Vec<UserFrame> = match self.physman.try_borrow_mut() {
                Ok(mut physman) => physman.alloc_many_user_frames(nframes)?,
                Err(_) => {
                    let reason: &str = "failed to borrow physical memory manager";
                    error!("{reason}");
                    return Err(Error::new(ErrorCode::ResourceBusy, reason));
                },
            };

            // FIXME: check if range is not busy.

            for uframe in uframes {
                vmem.map(uframe, vaddr, access, &page_table_allocator)?;
                vaddr = PageAligned::from_raw_value(vaddr.into_raw_value() + mem::PAGE_SIZE)?;
            }

            Ok(())
        }
    }

    ///
    /// # Description
    ///
    /// Changes the access permissions of a user page.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory space where the page is mapped.
    /// - `vaddr`: Virtual address of the page to be controlled.
    /// - `access`: Access permissions.
    ///
    /// # Return Values
    ///
    /// Upon success, empty is returned. Upon failure, an error is returned instead.
    ///
    pub fn ctrl_upage(
        &mut self,
        vmem: &mut Vmem,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        vmem.uctrl(vaddr, access)
    }

    ///
    /// # Description
    ///
    /// Allocates a kernel page.
    ///
    /// # Parameters
    ///
    /// - `clear`: Clear page?
    ///
    /// # Return Values
    ///
    /// Upon success, a kernel page is returned. Upon failure, an error is returned instead.
    ///
    pub fn alloc_kpage(&mut self, clear: bool) -> Result<KernelPage, Error> {
        let kframe: KernelFrame = match self.physman.try_borrow_mut() {
            Ok(mut physman) => physman.alloc_kernel_frame(clear)?,
            Err(_) => {
                let reason: &str = "failed to borrow physical memory manager";
                error!("{reason}");
                return Err(Error::new(ErrorCode::ResourceBusy, reason));
            },
        };
        Ok(KernelPage::new(kframe))
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of kernel pages.
    ///
    /// # Parameters
    ///
    /// - `clear`: Clear pages?
    /// - `count`: Number of pages to allocate.
    ///
    /// # Return Values
    ///
    /// Upon success, a vector of kernel pages is returned. Upon failure, an error is returned
    /// instead.
    ///
    pub fn alloc_kpages(&mut self, clear: bool, count: usize) -> Result<Vec<KernelPage>, Error> {
        let mut kpages: Vec<KernelFrame> = match self.physman.try_borrow_mut() {
            Ok(mut physman) => physman.alloc_many_kernel_frames(clear, count)?,
            Err(_) => {
                let reason: &str = "failed to borrow physical memory manager";
                error!("{reason}");
                return Err(Error::new(ErrorCode::ResourceBusy, reason));
            },
        };

        let mut pages: Vec<KernelPage> = Vec::new();
        while let Some(kframes) = kpages.pop() {
            pages.push(KernelPage::new(kframes));
        }

        Ok(pages)
    }

    /// Load an ELF image into a virtual address space.
    pub fn load_elf(
        &mut self,
        vmem: &mut Vmem,
        addr: usize,
    ) -> Result<(VirtualAddress, PageAligned<VirtualAddress>), Error> {
        elf::load_elf(self, vmem, addr)
    }
}
