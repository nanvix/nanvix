// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("manager.spec.rs");
#[cfg(verus_keep_ghost)]
include!("manager.proof.rs");

use crate::{
    hal::{
        arch::x86::mem::mmu::page_table::PageTable,
        mem::{
            AccessPermission,
            Address,
            FrameAddress,
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
    vec::Vec,
};
use ::arch::mem::{
    self,
    paging::PageTableEntry,
    PAGE_ALIGNMENT,
};
use ::core::{
    hint::unlikely,
    mem::{
        ManuallyDrop,
        MaybeUninit,
    },
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

/// Number of user mappings processed per chunk by [`VirtMemoryManager::link_user_pages`]
/// and [`VirtMemoryManager::rollback_linked_pages`]. Sized so the per-chunk buffer fits
/// comfortably on the kernel stack while keeping the snapshot/rollback walks of the
/// parent's user mappings to a small number of passes for typical user processes.
const LINK_CHUNK: usize = 32;

/// Snapshot entry for one parent user mapping consumed by
/// [`VirtMemoryManager::link_user_pages`].
type LinkUserMapping = (PageAligned<VirtualAddress>, FrameAddress, bool, bool);

/// Uninitialized slot in a link snapshot chunk.
type LinkUserMappingSlot = MaybeUninit<LinkUserMapping>;

/// Fixed-size chunk buffer used while linking user pages.
type LinkUserMappingBuf = [LinkUserMappingSlot; LINK_CHUNK];

///
/// # Description
///
/// Creates an uninitialized fixed-size array.
///
/// This helper wraps `[const { MaybeUninit::uninit() }; N]`, which Verus does
/// not support in verified function bodies. The construct is isolated here as
/// a narrow external body boundary while callers keep the surrounding control
/// flow visible to Verus.
///
/// # Returns
///
/// A fixed-size array of uninitialized values.
///
#[inline]
fn make_uninitialized_array<T: Sized, const N: usize>() -> [MaybeUninit<T>; N] { ... }

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
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub struct VirtMemoryManager;

impl VirtMemoryManager {
    ///
    /// # Description
    ///
    /// Initializes the virtual memory manager.
    ///
    /// # Parameters
    /// - `kernel_pages`: Kernel pages.
    /// - `kernel_page_tables`: Kernel page tables.
    ///
    pub fn init(
        kernel_pages: LinkedList<KernelPage>,
        kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) -> Result<Vmem, Error> { ... }

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
    pub unsafe fn get<'a>() -> &'a VirtMemoryManager { ... }

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
    pub unsafe fn get_mut<'a>() -> &'a mut VirtMemoryManager { ... }

    ///
    /// # Description
    ///
    /// Instantiates a memory manager.
    ///
    /// # Parameters
    /// - `kernel_pages`: Kernel pages.
    /// - `kernel_page_tables`: Kernel page tables.
    ///
    fn new(
        kernel_pages: LinkedList<KernelPage>,
        kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) -> Result<(Vmem, Self), Error> { ... }

    ///
    /// # Description
    ///
    /// Creates a new virtual address space, based on root.
    ///
    /// # Parameters
    /// - `vmem`: Virtual address space to clone.
    ///
    /// # Return Values
    /// - `Ok(new_vmem)` if the new virtual address space was successfully created.
    /// - `Err(_)` if the new virtual address space could not be created.
    ///
    #[verus_spec(ret =>
        requires
            self.inv(),
            vmem.inv(),
        ensures
            match ret {
                Ok(new) => {
                    &&& new.inv()
                    &&& new@.kernel == vmem@.kernel
                    &&& new@.user == Map::<nat, UserPageView>::empty()
                    &&& new@.pgdir != vmem@.pgdir
                },
                Err(_) => true,
            },
    )]
    // external_body: depends on the not-yet-verified `phys`, `kpage`, and
    // `PageDirectory` modules (no Verus contracts yet). The contract above is the
    // trusted boundary until those modules are verified.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    pub fn new_vmem(&self, vmem: &Vmem) -> Result<Vmem, Error> { ... }

    ///
    /// # Description
    ///
    /// Shares every user-space page mapped in `parent` with `child` using copy-on-write
    /// semantics.
    ///
    /// For each present user mapping in `parent`, this function adds a new reference to
    /// the underlying physical frame and installs the same mapping in `child` at the same
    /// virtual address. A mapping is treated as *logically writable* when it is writable in
    /// hardware or already copy-on-write (the latter arises when the page was shared
    /// writable by an earlier fork, leaving it read-only with the AVL CoW bit set). For a
    /// logically-writable mapping the child is mapped RDWR and then marked copy-on-write,
    /// and the parent is marked copy-on-write too unless it already was, so that the first
    /// write from any sharer triggers a page fault. The in-kernel page-fault handler
    /// resolves such faults by allocating a private frame and pointing the faulting PTE at
    /// it.
    ///
    /// Genuinely read-only mappings (e.g. the text segment) are shared without the
    /// copy-on-write bit set: a write to them must continue to fault as a regular
    /// protection fault.
    ///
    /// # Parameters
    ///
    /// - `parent`: Virtual memory space whose user mappings should be shared.
    /// - `child`: Destination virtual memory space. Must not already contain any user
    ///   mappings overlapping those of `parent`.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned and both
    /// `parent` and `child` are restored to the state they had on entry: any pages
    /// already linked into `child` are unmapped (releasing the shared refcount) and any
    /// copy-on-write marks installed on `parent` are cleared.
    ///
    #[verus_spec(ret =>
        requires
            self.inv(),
            parent.inv(),
            child.inv(),
            link_user_pages_pre(parent@, child@),
        ensures
            match ret {
                Ok(_) => {
                    &&& links_child_cow(old(parent)@, final(parent)@, old(child)@, final(child)@)
                    &&& final(parent).inv()
                    &&& final(child).inv()
                },
                Err(_) => {
                    &&& final(parent).inv()
                    &&& final(child).inv()
                },
            },
    )]
    // Verus front-end limitation: the body calls `parent.for_each_user_mapping(|..| { .. })`
    // with a closure that captures `count`, `buf`, and `child` by mutable reference. Verus
    // does not support closures capturing a mutable reference ("only &mut capture is blocked";
    // see verus-syntax/verus-constraints). The callback-based iteration API cannot be expressed
    // without such a capture, so this function is kept `external_body` and its `#[verus_spec]`
    // contract above is the trusted boundary. See `verus-unsupported.md`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    pub fn link_user_pages(&mut self, parent: &mut Vmem, child: &mut Vmem) -> Result<(), Error> { ... }

    /// Links a single user page from `parent` into `child` with copy-on-write semantics.
    ///
    /// On failure the caller is expected to invoke [`Self::rollback_linked_pages`] to
    /// undo any prior fully-linked iterations; this helper itself leaves no partial
    /// per-iteration state behind.
    fn link_one_user_page(
        parent: &mut Vmem,
        child: &mut Vmem,
        vaddr: PageAligned<VirtualAddress>,
        frame: FrameAddress,
        writable: bool,
        parent_cow: bool,
    ) -> Result<(), Error> { ... }

    /// Rolls back pages already linked by [`Self::link_user_pages`] on the error path.
    ///
    /// Iterates `child`'s user mappings in fixed-size chunks (avoiding any allocation
    /// proportional to the mapping set) and unmaps each page that this call could have
    /// linked, releasing the shared refcount via the returned `UserFrame`'s `Drop`. A
    /// child page is unmapped only when `parent` still has a present user mapping at the
    /// same virtual address: every page linked by [`Self::link_user_pages`] is a copy of a
    /// parent mapping, so a child page with no counterpart in `parent` is skipped.
    ///
    /// This `parent`-presence filter is a coarse heuristic, not exact provenance tracking:
    /// it cannot tell a page this call installed from a pre-existing `child` mapping that
    /// merely happens to overlap a `parent` mapping at the same address — such a page would
    /// still be unmapped. That ambiguity is harmless given [`Self::link_user_pages`]'s
    /// contract that `child` must not already contain user mappings overlapping `parent`'s,
    /// so under correct use the only `child` pages overlapping `parent` are exactly those
    /// this call linked. The filter simply avoids tearing down any non-overlapping
    /// pre-existing mappings.
    ///
    /// The parent's copy-on-write marks are intentionally left untouched. A parent page
    /// that this call marked copy-on-write could in principle be restored to writable, but
    /// distinguishing such a page from one that was already copy-on-write before this call
    /// (a re-fork of a frame still shared with an earlier child) cannot be done reliably:
    /// PTE state alone is ambiguous, and the shared frame's reference count is not either
    /// (e.g. a frame may have had a refcount of 2 from a different child while this child
    /// never linked it). Wrongly unmarking a page that another sharer still relies on would
    /// break copy-on-write for that sharer, whereas leaving a page copy-on-write that could
    /// have been writable merely costs one extra copy-on-write fault on the next write.
    /// Since this rollback path is taken only on a rare allocation/mapping failure, that
    /// extra fault is negligible, so the safe choice is to never unmark. Steps log and
    /// continue on failure to make a best-effort restoration.
    fn rollback_linked_pages(parent: &mut Vmem, child: &mut Vmem) { ... }

    ///
    /// # Description
    ///
    /// Attempts to handle a page fault as a copy-on-write fault.
    ///
    /// A page fault is treated as a copy-on-write fault when it is a user-mode write to
    /// a present page whose PTE has the AVL copy-on-write bit set. In that case this
    /// function allocates a private frame for the faulting address, copies the contents
    /// of the shared frame into it, and points the faulting PTE at the new frame with
    /// the writable bit restored and the copy-on-write bit cleared. The reference held
    /// on the old shared frame is then released.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory space of the faulting process.
    /// - `fault_addr`: Faulting virtual address (need not be page-aligned).
    /// - `error_code`: Typed x86 page-fault error code.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if the fault was resolved as a copy-on-write fault.
    /// - `Ok(false)` if the fault is not a copy-on-write fault and should be forwarded
    ///   to the registered handler.
    /// - `Err(_)` if the fault was a copy-on-write fault but resolving it failed.
    ///
    #[verus_spec(ret =>
        requires
            self.inv(),
            vmem.inv(),
        ensures
            match ret {
                Ok(true) => {
                    &&& spec_is_cow_write_fault(error_code)
                    &&& old(vmem)@.user_mapped(page_base(fault_addr as nat))
                    &&& old(vmem)@.user[page_base(fault_addr as nat)].cow
                    &&& exists|f: nat|
                            #![trigger old(vmem)@.spec_resolve_cow(page_base(fault_addr as nat), f)]
                            {
                                &&& is_page_aligned(f)
                                &&& spec_is_physical_region(f, page_size())
                                &&& final(vmem)@
                                        == old(vmem)@.spec_resolve_cow(page_base(fault_addr as nat), f)
                            }
                    &&& final(vmem).inv()
                },
                Ok(false) => {
                    &&& final(vmem)@ == old(vmem)@
                    &&& (!spec_is_cow_write_fault(error_code)
                            || !spec_is_user_addr(page_base(fault_addr as nat))
                            || !old(vmem)@.user_mapped(page_base(fault_addr as nat))
                            || !old(vmem)@.user[page_base(fault_addr as nat)].cow)
                },
                Err(_) => final(vmem)@ == old(vmem)@,
            },
    )]
    // external_body: depends on the not-yet-verified `arch::cpu::excp::ErrorCode`
    // accessors, `sys::mm::align_down`, and `hal` `PageAligned::from_raw_value`
    // (no Verus contracts yet). The contract above is the trusted boundary.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    pub fn try_resolve_cow_fault(
        &mut self,
        vmem: &mut Vmem,
        fault_addr: usize,
        error_code: ::arch::cpu::excp::ErrorCode,
    ) -> Result<bool, Error> { ... }

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
    #[verus_spec(ret =>
        requires
            self.inv(),
            vmem.inv(),
        ensures
            match ret {
                Ok(true) => {
                    &&& old(vmem)@.user_mapped(vaddr.addr_nat())
                    &&& final(vmem)@ == old(vmem)@.spec_unmap(vaddr.addr_nat())
                    &&& final(vmem).inv()
                },
                Ok(false) => {
                    &&& !old(vmem)@.user_mapped(vaddr.addr_nat())
                    &&& final(vmem)@ == old(vmem)@
                },
                Err(_) => final(vmem)@ == old(vmem)@,
            },
    )]
    pub fn try_unmap_upage(
        &mut self,
        vmem: &mut Vmem,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<bool, Error> { ... }

    ///
    /// # Description
    ///
    /// Allocates and maps user pages into a virtual address space.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory space where pages are mapped.
    /// - `vaddr`: Starting virtual address for the mapping.
    /// - `access`: Access permissions for the mapped pages.
    /// - `clear`: Clear pages after mapping?
    /// - `nframes`: Number of pages to allocate.
    /// - `uframes`: Mutable reference to a pre-allocated vector for temporary frame storage.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, all successfully mapped pages are rolled
    /// back and an error is returned instead.
    ///
    #[verus_spec(ret =>
        requires
            self.inv(),
            vmem.inv(),
            old(uframes)@.len() == 0,
        ensures
            match ret {
                Ok(_) => {
                    &&& maps_user_run_with(
                            old(vmem)@,
                            final(vmem)@,
                            vaddr.addr_nat(),
                            nframes as nat,
                            access.perms_view(),
                        )
                    &&& final(vmem).inv()
                    &&& final(uframes)@.len() == 0
                },
                Err(_) => {
                    &&& final(vmem)@ == old(vmem)@
                    &&& final(vmem).inv()
                    &&& final(uframes)@.len() == 0
                },
            },
    )]
    // external_body: uses `Vec::drain(..)`/`Vec::capacity()`, std iterator types
    // that vstd does not model. See `verus-unsupported.md`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    pub fn alloc_upages(
        &mut self,
        vmem: &mut Vmem,
        mut vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
        clear: bool,
        nframes: usize,
        uframes: &mut Vec<UserFrame>,
    ) -> Result<(), Error> { ... }

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
    #[verus_spec(ret =>
        requires
            self.inv(),
            vmem.inv(),
            vmem@.user_mapped(vaddr.addr_nat()),
        ensures
            match ret {
                Ok(_) => {
                    &&& final(vmem)@ == old(vmem)@.spec_uctrl(vaddr.addr_nat(), access.perms_view())
                    &&& final(vmem).inv()
                },
                Err(_) => final(vmem)@ == old(vmem)@,
            },
    )]
    pub fn ctrl_upage(
        &mut self,
        vmem: &mut Vmem,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> { ... }

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
    #[verus_spec(ret =>
        requires
            self.inv(),
        ensures
            match ret {
                Ok(kpage) => {
                    &&& is_page_aligned(kpage.addr_nat())
                    &&& spec_is_physical_region(kpage.addr_nat(), page_size())
                },
                Err(_) => true,
            },
    )]
    // external_body: depends on the not-yet-verified `phys` and `kpage` modules
    // (no Verus contracts yet). The contract above is the trusted boundary.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    pub fn alloc_kpage(&mut self, clear: bool) -> Result<KernelPage, Error> { ... }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of kernel frames into caller-provided storage.
    ///
    /// # Parameters
    ///
    /// - `clear`: Clear frames?
    /// - `count`: Number of frames to allocate.
    /// - `kframes`: Pre-allocated vector where allocated frames are placed. It
    ///   must have sufficient capacity for `count` entries.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `kframes` is filled with `count`
    /// frames. Upon failure, an error is returned instead.
    ///
    #[verus_spec(ret =>
        requires
            self.inv(),
            old(kframes)@.len() == 0,
        ensures
            match ret {
                Ok(_) => final(kframes)@.len() == count as nat,
                Err(_) => final(kframes)@.len() == 0,
            },
    )]
    // external_body: uses `iter_mut().try_for_each(..)`, std iterator combinators
    // that vstd does not model. See `verus-unsupported.md`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    pub fn alloc_kpages(
        &mut self,
        clear: bool,
        count: usize,
        kframes: &mut Vec<KernelFrame>,
    ) -> Result<(), Error> { ... }

    /// Load an ELF image into a virtual address space.
    #[verus_spec(ret =>
        requires
            self.inv(),
            vmem.inv(),
        ensures
            match ret {
                Ok((entry, args_vaddr)) => {
                    &&& final(vmem).inv()
                    &&& old(vmem)@.user.dom().subset_of(final(vmem)@.user.dom())
                    &&& final(vmem)@.kernel == old(vmem)@.kernel
                    &&& final(vmem)@.pgdir == old(vmem)@.pgdir
                    &&& spec_is_user_addr(entry.addr_nat())
                    &&& spec_is_user_addr(args_vaddr.addr_nat())
                    &&& is_page_aligned(args_vaddr.addr_nat())
                },
                Err(_) => final(vmem).inv(),
            },
    )]
    // external_body: delegates to the not-yet-verified `elf::elf32_load` (no Verus
    // contract yet). The contract above is the trusted boundary.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    pub fn load_elf(
        &mut self,
        vmem: &mut Vmem,
        elf: &Elf32Fhdr,
    ) -> Result<(VirtualAddress, PageAligned<VirtualAddress>), Error> { ... }
}
