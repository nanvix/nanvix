// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
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
        },
    },
};
use ::alloc::{
    collections::LinkedList,
    vec::Vec,
};
use ::sys::error::Error;

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
    physman: PhysMemoryManager,
}

impl VirtMemoryManager {
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
    pub fn new(
        kernel_pages: LinkedList<KernelPage>,
        kernel_page_tables: LinkedList<(PageTableAddress, PageTable)>,
        physman: PhysMemoryManager,
    ) -> Result<(Vmem, Self), Error> {
        let root: Vmem = Vmem::new(kernel_pages, kernel_page_tables)?;

        // Load root root address space.
        root.load()?;

        Ok((root, Self { physman }))
    }

    /// Creates a new virtual address space, based on root.
    pub fn new_vmem(&self, vmem: &Vmem) -> Result<Vmem, Error> {
        let new_vmem: Vmem = Vmem::clone(vmem)?;

        trace!(
            "new_vmem(): new_vmem={:?}, old_vmem={:?}",
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
    ) -> Result<(), Error> {
        let uframe: UserFrame = self.physman.alloc_user_frame()?;

        vmem.map(uframe, vaddr, access)?;

        Ok(())
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
        vmem.unmap(vaddr)?;
        Ok(())
    }

    pub fn alloc_upages(
        &mut self,
        vmem: &mut Vmem,
        mut vaddr: PageAligned<VirtualAddress>,
        nframes: usize,
        access: AccessPermission,
    ) -> Result<(), Error> {
        trace!("alloc_upages(): vaddr={:?}, nframes={}", vaddr, nframes);

        let uframes: Vec<UserFrame> = self.physman.alloc_many_user_frames(nframes)?;

        // FIXME: check if range is not busy.

        for uframe in uframes {
            vaddr = PageAligned::from_raw_value(vaddr.into_raw_value() + mem::PAGE_SIZE)?;
            vmem.map(uframe, vaddr, access)?;
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
        let kframe: KernelFrame = self.physman.alloc_kernel_frame(clear)?;
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
        let mut kpages: Vec<KernelFrame> = self.physman.alloc_many_kernel_frames(clear, count)?;

        let mut pages: Vec<KernelPage> = Vec::new();
        while let Some(kframes) = kpages.pop() {
            pages.push(KernelPage::new(kframes));
        }

        Ok(pages)
    }

    /// Load an ELF image into a virtual address space.
    pub fn load_elf(&mut self, vmem: &mut Vmem, elf: &Elf32Fhdr) -> Result<VirtualAddress, Error> {
        let entry: VirtualAddress = elf::elf32_load(self, vmem, elf)?;

        Ok(entry)
    }
}
