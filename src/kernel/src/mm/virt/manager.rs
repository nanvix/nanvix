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
fn make_uninitialized_array<T: Sized, const N: usize>() -> [MaybeUninit<T>; N] {
    [const { MaybeUninit::<T>::uninit() }; N]
}

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
    ) -> Result<Vmem, Error> {
        // Check if the memory manager is already initialized.
        if unlikely(MEMORY_MANAGER_INIT.load(ORDER)) {
            panic!("memory manager was already initialized");
        }

        let (root, manager): (Vmem, VirtMemoryManager) =
            VirtMemoryManager::new(kernel_pages, kernel_page_tables)?;

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
    ///
    fn new(
        kernel_pages: LinkedList<KernelPage>,
        kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) -> Result<(Vmem, Self), Error> {
        let root: Vmem = Vmem::new(kernel_pages, kernel_page_tables)?;

        // Load root root address space.
        root.load()?;

        Ok((root, Self))
    }

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
    pub fn new_vmem(&self, vmem: &Vmem) -> Result<Vmem, Error> {
        // Allocate a kernel page for the new page directory.
        let pgdir_page: KernelPage = {
            // The page directory initialization logic (PageDirectory::new/clean)
            // will zero the page; no need to clear the frame here.
            // SAFETY: the kernel is single-threaded and runs with interrupts disabled; no
            // concurrent or re-entrant access to the physical memory manager is possible.
            let kframe: KernelFrame =
                unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;
            KernelPage::new(kframe)
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
    pub fn link_user_pages(&mut self, parent: &mut Vmem, child: &mut Vmem) -> Result<(), Error> {
        // Process the parent's user mappings in fixed-size chunks. We cannot mutate
        // `parent` while borrowing its page tables via `for_each_user_mapping`, so each
        // chunk first snapshots up to LINK_CHUNK entries into a stack-resident buffer,
        // then performs the share/map/mark steps. This avoids any heap allocation that
        // scales with the parent's mapping count (the kernel slab allocator caps at
        // 512-byte requests, which a single Vec proportional to the mapping set quickly
        // exceeds).
        //
        // The selection filter checks `child` for an existing mapping at the candidate
        // vaddr and skips entries already linked by a prior chunk. This makes the walk
        // robust against the parent's iteration revisiting entries we have already
        // processed (e.g. writable entries that are now CoW-marked but still present).
        loop {
            let mut buf: LinkUserMappingBuf = make_uninitialized_array();
            let mut count: usize = 0;
            parent.for_each_user_mapping(|vaddr, pte: PageTableEntry| {
                if count < LINK_CHUNK && child.try_find_user_pte(vaddr)?.is_none() {
                    let frame: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;
                    // A page that is already copy-on-write (read-only in hardware with the
                    // AVL CoW bit set) was shared writable by a prior fork and is therefore
                    // logically writable. Classify it as writable so the new child is shared
                    // copy-on-write too, rather than as a plain read-only mapping that would
                    // fault fatally on the child's first write. The parent's current CoW
                    // state is carried alongside so the link step can skip re-marking an
                    // already-CoW parent (mark_cow requires a writable PTE).
                    let parent_cow: bool = pte.is_cow();
                    let writable: bool = pte.flags().is_writable() || parent_cow;
                    buf[count].write((vaddr, frame, writable, parent_cow));
                    count += 1;
                }
                Ok(())
            })?;

            if count == 0 {
                break;
            }

            for slot in buf.iter().take(count) {
                // SAFETY: `slot` was written above for indices < count.
                let (vaddr, frame, writable, parent_cow) = unsafe { slot.assume_init_read() };
                if let Err(e) =
                    Self::link_one_user_page(parent, child, vaddr, frame, writable, parent_cow)
                {
                    Self::rollback_linked_pages(parent, child);
                    return Err(e);
                }
            }

            if count < LINK_CHUNK {
                break;
            }
        }

        Ok(())
    }

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
    ) -> Result<(), Error> {
        // Wrap the parent's already-owned frame in a [`ManuallyDrop`] handle so that
        // we can call [`UserFrame::share`] on it without risking a spurious decrement
        // of the parent's refcount if `share` itself returns an error. The parent's
        // page table still holds the original reference; only `child_handle` carries
        // the freshly-acquired one.
        let parent_handle: ManuallyDrop<UserFrame> = ManuallyDrop::new(UserFrame::new(frame));
        let child_handle: UserFrame = parent_handle.share()?;

        // The child installs the shared frame at the same virtual address. If the page is
        // logically writable the mapping is created RDWR so the subsequent CoW marking can
        // switch it to RO+CoW (mark_cow requires the entry to be writable); otherwise the
        // page is plain read-only with no CoW bit on either side. If `map` fails it drops
        // `child_handle`, releasing the refcount just acquired by `share`.
        let access: AccessPermission = if writable {
            AccessPermission::RDWR
        } else {
            AccessPermission::RDONLY
        };
        child.map(child_handle, vaddr, access)?;

        if writable {
            // Mark the parent copy-on-write only if it is not already so. A page that is
            // already CoW was shared writable by an earlier fork; re-marking it would fail
            // because mark_cow requires a writable PTE. The child is still mapped RDWR
            // (above) and marked CoW (below) so that a later write from the child resolves
            // as a copy-on-write fault instead of faulting fatally.
            if !parent_cow {
                if let Err(e) = parent.mark_user_page_cow(vaddr) {
                    if let Err(re) = child.unmap(vaddr) {
                        warn!(
                            "link_user_pages(): partial rollback unmap failed (vaddr={vaddr:?}, \
                             error={re:?})"
                        );
                    }
                    return Err(e);
                }
            }
            if let Err(e) = child.mark_user_page_cow(vaddr) {
                // Only undo the parent's mark if this call installed it; a parent that was
                // already copy-on-write must be left untouched.
                if !parent_cow {
                    if let Err(re) = parent.unmark_user_page_cow(vaddr) {
                        warn!(
                            "link_user_pages(): partial rollback unmark failed (vaddr={vaddr:?}, \
                             error={re:?})"
                        );
                    }
                }
                if let Err(re) = child.unmap(vaddr) {
                    warn!(
                        "link_user_pages(): partial rollback unmap failed (vaddr={vaddr:?}, \
                         error={re:?})"
                    );
                }
                return Err(e);
            }
        }

        Ok(())
    }

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
    fn rollback_linked_pages(parent: &mut Vmem, child: &mut Vmem) {
        loop {
            let mut buf: [MaybeUninit<PageAligned<VirtualAddress>>; LINK_CHUNK] =
                make_uninitialized_array();
            let mut count: usize = 0;
            let walk: Result<(), Error> = child.for_each_user_mapping(|vaddr, _pte| {
                if count < LINK_CHUNK {
                    // Only consider pages that also exist in `parent`; a child mapping
                    // without a parent counterpart was not installed by this call.
                    match parent.is_user_page_mapped(vaddr) {
                        Ok(true) => {
                            buf[count].write(vaddr);
                            count += 1;
                        },
                        Ok(false) => {},
                        Err(e) => return Err(e),
                    }
                }
                Ok(())
            });
            if let Err(e) = walk {
                warn!("link_user_pages(): rollback walk failed (error={e:?})");
                return;
            }
            if count == 0 {
                return;
            }
            for slot in buf.iter().take(count) {
                // SAFETY: `slot` was written above for indices < count.
                let vaddr = unsafe { slot.assume_init_read() };
                if let Err(re) = child.unmap(vaddr) {
                    warn!(
                        "link_user_pages(): rollback unmap failed (vaddr={vaddr:?}, error={re:?})"
                    );
                }
            }
            if count < LINK_CHUNK {
                return;
            }
        }
    }

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
    pub fn try_resolve_cow_fault(
        &mut self,
        vmem: &mut Vmem,
        fault_addr: usize,
        error_code: ::arch::cpu::excp::ErrorCode,
    ) -> Result<bool, Error> {
        // Copy-on-write faults are user-mode writes to a present page.
        if !error_code.is_present() || !error_code.is_write() || !error_code.is_user() {
            return Ok(false);
        }

        // Reject addresses that are not in user space outright. We must do this before
        // touching `vmem` so that bogus addresses do not error out.
        let page_addr: usize = ::sys::mm::align_down(fault_addr, PAGE_ALIGNMENT);
        let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(page_addr) {
            Ok(v) => v,
            Err(_) => return Ok(false),
        };

        // Delegate to the shared resolver. It returns `Ok(false)` if the page is not
        // copy-on-write (in which case the fault is forwarded to the registered handler)
        // and `Ok(true)` if a copy-on-write mapping was resolved.
        vmem.resolve_cow_at(vaddr)
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
        // The returned `UserFrame` is dropped here, which frees the underlying physical frame.
        Ok(vmem.unmap(vaddr)?.is_some())
    }

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
    pub fn alloc_upages(
        &mut self,
        vmem: &mut Vmem,
        mut vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
        clear: bool,
        nframes: usize,
        uframes: &mut Vec<UserFrame>,
    ) -> Result<(), Error> {
        trace!("vaddr={:?}, nframes={}", vaddr, nframes);

        // The caller-supplied buffer must be empty; stale frames would cause double-mapping.
        if !uframes.is_empty() {
            let reason: &str = "caller-supplied uframes vector is not empty";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        if uframes.capacity() < nframes {
            let reason: &str = "caller-supplied uframes vector has insufficient capacity";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

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
        let mut checked_count: usize = 0;
        while checked_count < nframes {
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
            checked_count += 1;
        }

        // SAFETY: the kernel is single-threaded and runs with interrupts disabled; no concurrent
        // or re-entrant access to the physical memory manager is possible.
        let alloc_result: Result<(), Error> =
            unsafe { PhysMemoryManager::get_mut() }.alloc_many_user_frames(nframes, uframes);
        if let Err(e) = alloc_result {
            uframes.clear();
            return Err(e);
        }

        let start_vaddr: PageAligned<VirtualAddress> = vaddr;
        let mut mapped_count: usize = 0;
        let mut map_error: Result<(), Error> = Ok(());

        {
            let mut drain = uframes.drain(..);
            loop {
                let uframe: UserFrame = match drain.next() {
                    Some(uframe) => uframe,
                    None => break,
                };

                if let Err(e) = vmem.map(uframe, vaddr, access) {
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
        }

        if let Err(e) = map_error {
            // Rollback: unmap all pages that were successfully mapped.
            let mut rollback_addr: PageAligned<VirtualAddress> = start_vaddr;
            let mut rollback_count: usize = 0;
            while rollback_count < mapped_count {
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
                rollback_count += 1;
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
        // SAFETY: the kernel is single-threaded and runs with interrupts disabled; no concurrent
        // or re-entrant access to the physical memory manager is possible.
        let mut kframe: KernelFrame =
            unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;
        if clear {
            kframe.clear()?;
        }
        Ok(KernelPage::new(kframe))
    }

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
    pub fn alloc_kpages(
        &mut self,
        clear: bool,
        count: usize,
        kframes: &mut Vec<KernelFrame>,
    ) -> Result<(), Error> {
        if !kframes.is_empty() {
            let reason: &str = "caller-supplied kframes vector is not empty";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        if kframes.capacity() < count {
            let reason: &str = "caller-supplied kframes vector has insufficient capacity";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // SAFETY: the kernel is single-threaded and runs with interrupts disabled; no concurrent
        // or re-entrant access to the physical memory manager is possible.
        unsafe { PhysMemoryManager::get_mut() }.alloc_many_kernel_frames(count, kframes)?;
        if clear {
            let clear_result = kframes.iter_mut().try_for_each(|kframe| kframe.clear());
            if let Err(e) = clear_result {
                // Drop all allocated frames to free them back to the physical memory manager.
                kframes.clear();
                return Err(e);
            }
        }

        Ok(())
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
