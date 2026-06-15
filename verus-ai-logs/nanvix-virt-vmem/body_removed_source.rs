// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// TODO: remove this.
#![allow(clippy::type_complexity)]

//==================================================================================================
// Imports
//==================================================================================================
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("vmem.spec.rs");
#[cfg(verus_keep_ghost)]
include!("vmem.proof.rs");

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
            page_table_allocator::PAGE_TABLE_ALLOCATOR,
            PageDirectoryStorage,
            PageTableStorage,
            VirtMemoryManager,
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
    mem::ManuallyDrop,
};
use ::type_safe::{
    usize_to_const_ptr,
    usize_to_mut_ptr,
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
}

impl Vmem {
    /// Initializes a new virtual memory space.
    pub fn new(
        mut kernel_pages: LinkedList<KernelPage>,
        mut kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) -> Result<Self, Error> { ... }

    /// Clones the target virtual memory space.
    pub fn clone(from: &Vmem, pgdir_page: KernelPage) -> Result<Vmem, Error> { ... }

    pub fn load(&self) -> Result<(), Error> { ... }

    /// Returns a reference to the underlying page directory.
    pub fn pgdir(&self) -> &PageDirectory<PageDirectoryStorage> { ... }

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
    ) -> Result<(), Error> { ... }

    ///
    /// # Description
    ///
    /// Allocate a page table for mapping kernel memory.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(page_table)` is returned. Upon failure, an error is returned.
    ///
    fn allocate_kernel_page_table() -> Result<PageTable<PageTableStorage>, Error> { ... }

    ///
    /// # Description
    ///
    /// Allocate a page table for mapping user memory.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(page_table)` is returned. Upon failure, an error is returned.
    ///
    fn allocate_user_page_table() -> Result<PageTable<PageTableStorage>, Error> { ... }

    /// Maps a page to the target virtual address space.
    pub fn map(
        &mut self,
        uframe: UserFrame,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> { ... }

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
    pub fn is_user_page_mapped(&self, vaddr: PageAligned<VirtualAddress>) -> Result<bool, Error> { ... }

    /// Asserts whether an address lies in the user space.
    pub fn is_user_addr(virt_addr: VirtualAddress) -> bool { ... }

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
    pub fn is_user_region(start: VirtualAddress, size: usize) -> bool { ... }

    /// Asserts whether an address lies in the kernel space.
    fn is_kernel_addr(virt_addr: VirtualAddress) -> bool { ... }

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
    fn is_kernel_region(start: VirtualAddress, size: usize) -> bool { ... }

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
    pub fn is_physical_region(start: usize, size: usize) -> bool { ... }

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
    ) -> Result<&mut PageTable<PageTableStorage>, Error> { ... }

    fn lookup_kernel_page_table(
        &mut self,
        pde: &PageDirectoryEntry,
    ) -> Result<Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>, Error> { ... }

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
    fn find_user_frame(&self, vaddr: PageAligned<VirtualAddress>) -> Result<FrameAddress, Error> { ... }

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
    ) -> Result<Option<FrameAddress>, Error> { ... }

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
    ) -> Result<Option<PageTableEntry>, Error> { ... }

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
    { ... }

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
    pub fn mark_user_page_cow(&mut self, vaddr: PageAligned<VirtualAddress>) -> Result<(), Error> { ... }

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
    ) -> Result<(), Error> { ... }

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
    ) -> Result<FrameAddress, Error> { ... }

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
    pub fn resolve_cow_at(&mut self, vaddr: PageAligned<VirtualAddress>) -> Result<bool, Error> { ... }

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
    ) -> Result<(), Error> { ... }

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
    pub fn user_vaddr_to_paddr(&self, vaddr: VirtualAddress) -> Result<usize, Error> { ... }

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
    ) -> Result<(), Error> { ... }

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
    ) -> Result<(), Error> { ... }

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
    ) -> Result<(), Error> { ... }

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
    ) -> Result<(), Error> { ... }

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
    pub fn memset(&mut self, dst: PageAligned<VirtualAddress>, value: u32) -> Result<(), Error> { ... }

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
    ) -> Result<Option<UserFrame>, Error> { ... }

    /// Changes access permissions on a page.
    pub fn uctrl(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> { ... }

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
    ) -> Result<(), Error> { ... }
}

impl Drop for Vmem {
    fn drop(&mut self) { ... }
}
