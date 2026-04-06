// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::x86::mem::mmu::page_table::PageTable,
        mem::{
            AccessPermission,
            Address,
            PageAligned,
            PageTableAddress,
            VirtualAddress,
        },
    },
    mm::{
        elf::{
            self,
            Elf32Fhdr,
        },
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

        // Load root root address space.
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
        // Allocate a kernel page for the new page directory.
        let pgdir_page: KernelPage = match self.physman.try_borrow_mut() {
            Ok(mut physman) => {
                // The page directory initialization logic (PageDirectory::new/clean)
                // will zero the page; no need to clear the frame here.
                let kframe: KernelFrame = physman.alloc_kernel_frame(false)?;
                KernelPage::new(kframe)
            },
            Err(_) => {
                let reason: &str = "failed to borrow physical memory manager";
                error!("{reason}");
                return Err(Error::new(ErrorCode::ResourceBusy, reason));
            },
        };

        let new_vmem: Vmem = Vmem::clone(vmem, pgdir_page)?;

        trace!(
            "new_vmem={:?}, old_vmem={:?}",
            new_vmem.pgdir().physical_address(),
            vmem.pgdir().physical_address()
        );

        Ok(new_vmem)
    }

    ///
    /// # Description
    ///
    /// Attempts to unmap a user page from the target virtual memory space.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory space where the page is mapped.
    /// - `vaddr`: Virtual address of the page to be unmapped.
    ///
    /// # Return Values
    ///
    /// - `Ok(true)` if the page was present and has been unmapped.
    /// - `Ok(false)` if the page was not present.
    /// - `Err(_)` on unexpected failures.
    ///
    pub fn try_unmap_upage(
        &mut self,
        vmem: &mut Vmem,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<bool, Error> {
        if let Some(uframe) = vmem.unmap(vaddr)? {
            self.physman.borrow_mut().free_user_frame(uframe)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn alloc_upages(
        &mut self,
        vmem: &mut Vmem,
        mut vaddr: PageAligned<VirtualAddress>,
        nframes: usize,
        access: AccessPermission,
        clear: bool,
    ) -> Result<(), Error> {
        trace!("vaddr={:?}, nframes={}", vaddr, nframes);

        // Validate that nframes is positive and the full range lies in user space.
        let range_size: usize = nframes.checked_mul(mem::PAGE_SIZE).ok_or_else(|| {
            let reason: &str = "range size overflow";
            error!("{reason} (nframes={nframes})");
            Error::new(ErrorCode::InvalidArgument, reason)
        })?;
        if !Vmem::is_user_region(vaddr.into_inner(), range_size) {
            let reason: &str = "range is not entirely in user space";
            error!("{reason} (vaddr={vaddr:?}, nframes={nframes})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Check that none of the pages in the range are already mapped.
        let mut check_addr: PageAligned<VirtualAddress> = vaddr;
        for _ in 0..nframes {
            if vmem.is_user_page_mapped(check_addr)? {
                let reason: &str = "page already mapped in range";
                error!("{reason} (vaddr={check_addr:?})");
                return Err(Error::new(ErrorCode::ResourceBusy, reason));
            }
            check_addr = PageAligned::from_raw_value(
                check_addr
                    .into_raw_value()
                    .checked_add(mem::PAGE_SIZE)
                    .ok_or_else(|| {
                        let reason: &str = "address overflow in range check";
                        error!("{reason} (check_addr={check_addr:?})");
                        Error::new(ErrorCode::BadAddress, reason)
                    })?,
            )?;
        }

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

        let start_vaddr: PageAligned<VirtualAddress> = vaddr;
        let mut mapped_count: usize = 0;
        let mut map_error: Result<(), Error> = Ok(());

        for uframe in uframes {
            if let Err(e) = vmem.map(uframe, vaddr, access, &page_table_allocator) {
                map_error = Err(e);
                break;
            }
            mapped_count += 1;
            if clear {
                if let Err(e) = vmem.memset(vaddr, 0) {
                    map_error = Err(e);
                    break;
                }
            }
            match PageAligned::from_raw_value(vaddr.into_raw_value() + mem::PAGE_SIZE) {
                Ok(next) => vaddr = next,
                Err(e) => {
                    map_error = Err(e);
                    break;
                },
            }
        }

        if let Err(e) = map_error {
            // Rollback: unmap all pages that were successfully mapped.
            let mut rollback_addr: PageAligned<VirtualAddress> = start_vaddr;
            for _ in 0..mapped_count {
                if let Err(re) = self.try_unmap_upage(vmem, rollback_addr) {
                    warn!(
                        "alloc_upages(): rollback failed (vaddr={rollback_addr:?}, error={re:?})"
                    );
                }
                rollback_addr = match PageAligned::from_raw_value(
                    rollback_addr.into_raw_value() + mem::PAGE_SIZE,
                ) {
                    Ok(next) => next,
                    Err(_) => break,
                };
            }
            return Err(e);
        }

        Ok(())
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
        elf: &Elf32Fhdr,
    ) -> Result<(VirtualAddress, PageAligned<VirtualAddress>), Error> {
        elf::elf32_load(self, vmem, elf)
    }
}
