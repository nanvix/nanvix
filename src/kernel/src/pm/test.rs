// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        AccessPermission,
        PageAligned,
        VirtualAddress,
    },
    ipc::Mailbox,
    mm::{
        phys::{
            PhysMemoryManager,
            UserFrame,
        },
        VirtMemoryManager,
        Vmem,
    },
    pm::ProcessManager,
};
use ::arch::mem::paging::PageTableEntry;
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    mm::Address,
};

//==================================================================================================
// Tests
//==================================================================================================

///
/// # Description
///
/// Verifies that a freshly constructed mailbox reports as empty, and that posting a single
/// message causes it to report as non-empty.
///
fn test_mailbox_is_empty_tracks_buffered_messages() -> bool {
    let mut mailbox: Mailbox = Mailbox::default();
    if !mailbox.is_empty() {
        error!("freshly constructed mailbox reported as non-empty");
        return false;
    }

    let message: Message = Message::new(
        MessageSender::KERNEL,
        MessageReceiver::KERNEL,
        MessageType::Ipc,
        Option::<ErrorCode>::None,
        [0u8; Message::PAYLOAD_SIZE],
    );
    mailbox.send(message);
    if mailbox.is_empty() {
        error!("mailbox reported as empty after sending a message");
        return false;
    }
    true
}

///
/// # Description
///
/// Verifies that the kernel process (the process running at kernel-test time) does not own any
/// special resources, i.e. that [`crate::pm::process::state::ProcessState::has_special_resources`]
/// returns `false` for the kernel process. This is the precondition that allows the
/// `duplicate()` kernel call to succeed.
///
fn test_kernel_process_has_no_special_resources() -> bool {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };
    if pm.current_has_special_resources() {
        error!("kernel process unexpectedly owns special resources");
        return false;
    }
    true
}

///
/// # Description
///
/// End-to-end exercise of the copy-on-write resolution path.
///
/// Sets up a single user page that is shared between the page table of a freshly created
/// virtual memory space and a standalone [`UserFrame`] handle (the latter standing in for the
/// "other" address space that would normally co-own the frame after a fork-style duplication).
/// The page is then marked copy-on-write, and [`Vmem::resolve_cow_at`] is invoked to verify
/// that:
///
/// - the call reports the fault as resolved (`Ok(true)`),
/// - the page-table entry is rewritten to a *different* frame number,
/// - the rewritten entry is writable and no longer carries the copy-on-write bit, and
/// - dropping the lingering shared handle releases the last reference to the original frame
///   (otherwise the resolver would have failed to drop its reference).
///
fn test_cow_resolution_creates_private_frame() -> bool {
    // Pick a page-aligned user-space virtual address that is guaranteed not to overlap any
    // mapping the kernel process already owns. `USER_MMAP_BASE_RAW` is the base of the
    // user-mmap region and is reserved for ad-hoc user mappings.
    const TEST_VADDR_RAW: usize = ::config::memory_layout::USER_MMAP_BASE_RAW;

    let vaddr: PageAligned<VirtualAddress> =
        match PageAligned::<VirtualAddress>::from_raw_value(TEST_VADDR_RAW) {
            Ok(v) => v,
            Err(e) => {
                error!("PageAligned::from_raw_value failed (error={e:?})");
                return false;
            },
        };

    // SAFETY: pm/init() runs after the physical and virtual memory managers are initialized;
    // access is synchronized because the kernel is single-threaded with interrupts disabled.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };
    let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };

    // Build a fresh address space cloned off the kernel process's vmem so the test does not
    // disturb the running kernel mappings.
    let mut parent: Vmem = match mm.new_vmem(pm.current_vmem()) {
        Ok(v) => v,
        Err(e) => {
            error!("new_vmem failed (error={e:?})");
            return false;
        },
    };

    // Allocate a user frame and create a second handle aliasing the same physical frame.
    // The second handle stands in for the "other" address space that co-owns the frame.
    let uframe1: UserFrame = match unsafe { PhysMemoryManager::get_mut() }.alloc_user_frame() {
        Ok(f) => f,
        Err(e) => {
            error!("alloc_user_frame failed (error={e:?})");
            return false;
        },
    };
    let uframe2: UserFrame = match uframe1.share() {
        Ok(f) => f,
        Err(e) => {
            error!("UserFrame::share failed (error={e:?})");
            return false;
        },
    };

    // Map the first handle into `parent` at the test address. `Vmem::map` leaks the handle
    // into the page table, so it must not be referenced again from this function.
    if let Err(e) = parent.map(uframe1, vaddr, AccessPermission::RDWR) {
        error!("Vmem::map failed (error={e:?})");
        drop(uframe2);
        return false;
    }

    // Mark the page copy-on-write.
    if let Err(e) = parent.mark_user_page_cow(vaddr) {
        error!("mark_user_page_cow failed (error={e:?})");
        drop(uframe2);
        return false;
    }

    // After marking, the PTE must be present, read-only, carry the CoW bit, and still point at
    // the original (shared) frame.
    let before: PageTableEntry = match parent.try_find_user_pte(vaddr) {
        Ok(Some(p)) => p,
        Ok(None) => {
            error!("PTE missing after map + mark_user_page_cow");
            drop(uframe2);
            return false;
        },
        Err(e) => {
            error!("try_find_user_pte failed (error={e:?})");
            drop(uframe2);
            return false;
        },
    };
    if before.flags().is_writable() {
        error!("PTE is unexpectedly writable after mark_user_page_cow");
        drop(uframe2);
        return false;
    }
    if !before.is_cow() {
        error!("PTE is missing CoW bit after mark_user_page_cow");
        drop(uframe2);
        return false;
    }
    let original_frame_number = before.frame_number().into_raw_value();

    // Resolve the CoW mapping. This must allocate a new frame, copy the page contents, repoint
    // the PTE, and drop the resolver's reference on the previously shared frame.
    match parent.resolve_cow_at(vaddr) {
        Ok(true) => {},
        Ok(false) => {
            error!("resolve_cow_at returned false on a CoW mapping");
            drop(uframe2);
            return false;
        },
        Err(e) => {
            error!("resolve_cow_at failed (error={e:?})");
            drop(uframe2);
            return false;
        },
    };

    // The PTE must now be writable, no longer CoW, and point at a different frame.
    let after: PageTableEntry = match parent.try_find_user_pte(vaddr) {
        Ok(Some(p)) => p,
        Ok(None) => {
            error!("PTE missing after resolve_cow_at");
            drop(uframe2);
            return false;
        },
        Err(e) => {
            error!("try_find_user_pte failed after resolve_cow_at (error={e:?})");
            drop(uframe2);
            return false;
        },
    };
    if !after.flags().is_writable() {
        error!("PTE is not writable after resolve_cow_at");
        drop(uframe2);
        return false;
    }
    if after.is_cow() {
        error!("PTE still has CoW bit after resolve_cow_at");
        drop(uframe2);
        return false;
    }
    if after.frame_number().into_raw_value() == original_frame_number {
        error!("PTE was not repointed at a new frame");
        drop(uframe2);
        return false;
    }

    // Drop the lingering shared handle: this must release the last reference to the original
    // frame. If `resolve_cow_at` had failed to drop its own reference, refcount would still be
    // 1 here and the underlying frame would leak — that would not crash the test but would
    // show up as a frame-allocator inconsistency on subsequent runs.
    drop(uframe2);

    // Reclaim the private frame installed by `resolve_cow_at` before dropping `parent`. Dropping
    // an address space frees only its page-table structures, not the user frames their entries
    // map, so user frames must be reclaimed explicitly first — exactly as production does on the
    // harvest/exit path. The debug assertion in `Vmem::drop` enforces that no user pages remain
    // mapped at drop time.
    if let Err(e) = parent.clear_user_space() {
        error!("clear_user_space failed during test teardown (error={e:?})");
        return false;
    }
    drop(parent);

    true
}

///
/// # Description
///
/// Exercises the fast path in [`Vmem::resolve_cow_at`]: when the resolving address space
/// is the sole remaining owner of the shared frame (refcount == 1), the resolver must
/// simply clear the copy-on-write mark in place — without allocating a new frame, copying
/// any contents, or freeing the old frame.
///
/// Setup mirrors `test_cow_resolution_creates_private_frame` but drops the lingering
/// shared handle *before* invoking `resolve_cow_at`, leaving refcount = 1.
///
/// Asserts that after the call the PTE is writable, no longer CoW, and still points at
/// the *original* frame (proving no copy occurred).
///
fn test_cow_resolution_fast_path_when_sole_owner() -> bool {
    const TEST_VADDR_RAW: usize = ::config::memory_layout::USER_MMAP_BASE_RAW;

    let vaddr: PageAligned<VirtualAddress> =
        match PageAligned::<VirtualAddress>::from_raw_value(TEST_VADDR_RAW) {
            Ok(v) => v,
            Err(e) => {
                error!("PageAligned::from_raw_value failed (error={e:?})");
                return false;
            },
        };

    let pm: &ProcessManager = unsafe { ProcessManager::get() };
    let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };

    let mut parent: Vmem = match mm.new_vmem(pm.current_vmem()) {
        Ok(v) => v,
        Err(e) => {
            error!("new_vmem failed (error={e:?})");
            return false;
        },
    };

    let uframe1: UserFrame = match unsafe { PhysMemoryManager::get_mut() }.alloc_user_frame() {
        Ok(f) => f,
        Err(e) => {
            error!("alloc_user_frame failed (error={e:?})");
            return false;
        },
    };
    let uframe2: UserFrame = match uframe1.share() {
        Ok(f) => f,
        Err(e) => {
            error!("UserFrame::share failed (error={e:?})");
            return false;
        },
    };

    if let Err(e) = parent.map(uframe1, vaddr, AccessPermission::RDWR) {
        error!("Vmem::map failed (error={e:?})");
        drop(uframe2);
        return false;
    }

    if let Err(e) = parent.mark_user_page_cow(vaddr) {
        error!("mark_user_page_cow failed (error={e:?})");
        drop(uframe2);
        return false;
    }

    // Capture the original frame number, then drop the second handle so that the page
    // table holds the only remaining reference (refcount == 1). This is the precondition
    // for the fast path in `resolve_cow_at`.
    let before: PageTableEntry = match parent.try_find_user_pte(vaddr) {
        Ok(Some(p)) => p,
        Ok(None) => {
            error!("PTE missing after map + mark_user_page_cow");
            drop(uframe2);
            return false;
        },
        Err(e) => {
            error!("try_find_user_pte failed (error={e:?})");
            drop(uframe2);
            return false;
        },
    };
    let original_frame_number = before.frame_number().into_raw_value();
    drop(uframe2);

    // Resolve the CoW mapping. The fast path must clear the CoW mark in place.
    match parent.resolve_cow_at(vaddr) {
        Ok(true) => {},
        Ok(false) => {
            error!("resolve_cow_at returned false on a CoW mapping");
            return false;
        },
        Err(e) => {
            error!("resolve_cow_at failed (error={e:?})");
            return false;
        },
    };

    // The PTE must now be writable, no longer CoW, and still point at the original frame
    // (no copy was performed because refcount was already 1).
    let after: PageTableEntry = match parent.try_find_user_pte(vaddr) {
        Ok(Some(p)) => p,
        Ok(None) => {
            error!("PTE missing after resolve_cow_at");
            return false;
        },
        Err(e) => {
            error!("try_find_user_pte failed after resolve_cow_at (error={e:?})");
            return false;
        },
    };
    if !after.flags().is_writable() {
        error!("PTE is not writable after fast-path resolve_cow_at");
        return false;
    }
    if after.is_cow() {
        error!("PTE still has CoW bit after fast-path resolve_cow_at");
        return false;
    }
    if after.frame_number().into_raw_value() != original_frame_number {
        error!(
            "fast path unexpectedly repointed PTE at a new frame (old={original_frame_number:#x}, \
             new={:#x})",
            after.frame_number().into_raw_value()
        );
        return false;
    }

    // Reclaim the user frame before dropping `parent`; see the teardown note in
    // `test_cow_resolution_creates_private_frame`. The debug assertion in `Vmem::drop` requires
    // that no user pages remain mapped at drop time.
    if let Err(e) = parent.clear_user_space() {
        error!("clear_user_space failed during test teardown (error={e:?})");
        return false;
    }
    drop(parent);
    true
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Verifies that [`VirtMemoryManager::link_user_pages`] returns an error when `child`
/// already contains a user mapping overlapping one of `parent`'s, instead of silently
/// skipping the colliding address.
///
/// Setup:
///
/// - `parent` is given two consecutive writable user pages at `vaddr_a` and `vaddr_b`.
/// - `child` is pre-populated with an independent user page at `vaddr_b`, so that
///   address overlaps a `parent` mapping.
///
/// Expected post-conditions after the call returns `Err(ErrorCode::EntryExists)`:
///
/// - parent's `vaddr_a` PTE is untouched (writable, not CoW, original frame) because the
///   overlap is detected before any page is linked.
/// - parent's `vaddr_b` PTE is untouched (writable, not CoW, original frame).
/// - `child` has no mapping at `vaddr_a` (nothing was linked).
/// - `child`'s mapping at `vaddr_b` is unchanged (the pre-existing page is intact).
///
fn test_link_user_pages_errors_on_preexisting_child_overlap() -> bool {
    const VADDR_A_RAW: usize = ::config::memory_layout::USER_MMAP_BASE_RAW;
    const VADDR_B_RAW: usize = ::config::memory_layout::USER_MMAP_BASE_RAW + ::arch::mem::PAGE_SIZE;

    let vaddr_a: PageAligned<VirtualAddress> =
        match PageAligned::<VirtualAddress>::from_raw_value(VADDR_A_RAW) {
            Ok(v) => v,
            Err(e) => {
                error!("PageAligned::from_raw_value(vaddr_a) failed (error={e:?})");
                return false;
            },
        };
    let vaddr_b: PageAligned<VirtualAddress> =
        match PageAligned::<VirtualAddress>::from_raw_value(VADDR_B_RAW) {
            Ok(v) => v,
            Err(e) => {
                error!("PageAligned::from_raw_value(vaddr_b) failed (error={e:?})");
                return false;
            },
        };

    // SAFETY: pm/init() runs after the physical and virtual memory managers are
    // initialized; access is synchronized because the kernel is single-threaded with
    // interrupts disabled.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };

    let mut parent: Vmem = {
        let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };
        match mm.new_vmem(pm.current_vmem()) {
            Ok(v) => v,
            Err(e) => {
                error!("new_vmem(parent) failed (error={e:?})");
                return false;
            },
        }
    };
    let mut child: Vmem = {
        let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };
        match mm.new_vmem(pm.current_vmem()) {
            Ok(v) => v,
            Err(e) => {
                error!("new_vmem(child) failed (error={e:?})");
                return false;
            },
        }
    };

    // Allocate three independent user frames.
    let frame_a: UserFrame = match unsafe { PhysMemoryManager::get_mut() }.alloc_user_frame() {
        Ok(f) => f,
        Err(e) => {
            error!("alloc_user_frame(a) failed (error={e:?})");
            return false;
        },
    };
    let frame_b: UserFrame = match unsafe { PhysMemoryManager::get_mut() }.alloc_user_frame() {
        Ok(f) => f,
        Err(e) => {
            error!("alloc_user_frame(b) failed (error={e:?})");
            return false;
        },
    };
    let frame_c: UserFrame = match unsafe { PhysMemoryManager::get_mut() }.alloc_user_frame() {
        Ok(f) => f,
        Err(e) => {
            error!("alloc_user_frame(c) failed (error={e:?})");
            return false;
        },
    };

    let frame_a_num: usize = frame_a.address().into_frame_number().into_raw_value();
    let frame_b_num: usize = frame_b.address().into_frame_number().into_raw_value();
    let frame_c_num: usize = frame_c.address().into_frame_number().into_raw_value();

    // Map parent's two writable user pages.
    if let Err(e) = parent.map(frame_a, vaddr_a, AccessPermission::RDWR) {
        error!("parent.map(vaddr_a) failed (error={e:?})");
        return false;
    }
    if let Err(e) = parent.map(frame_b, vaddr_b, AccessPermission::RDWR) {
        error!("parent.map(vaddr_b) failed (error={e:?})");
        return false;
    }

    // Pre-map child at vaddr_b so it overlaps a parent mapping.
    if let Err(e) = child.map(frame_c, vaddr_b, AccessPermission::RDWR) {
        error!("child.map(vaddr_b) failed (error={e:?})");
        return false;
    }

    // Invoke the routine under test. Must fail with `EntryExists`: the child already
    // contains a user mapping overlapping the parent's at vaddr_b.
    {
        let mm_mut: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
        match mm_mut.link_user_pages(&mut parent, &mut child) {
            Ok(()) => {
                error!("link_user_pages unexpectedly succeeded on overlapping child");
                return false;
            },
            Err(e) if e.code == ErrorCode::EntryExists => {},
            Err(e) => {
                error!("link_user_pages failed with unexpected error (error={e:?})");
                return false;
            },
        }
    }

    // Parent's vaddr_a must be untouched: the overlap is detected before any linking.
    let parent_a: PageTableEntry = match parent.try_find_user_pte(vaddr_a) {
        Ok(Some(p)) => p,
        Ok(None) => {
            error!("parent PTE at vaddr_a missing after rejected link");
            return false;
        },
        Err(e) => {
            error!("try_find_user_pte(parent, vaddr_a) failed (error={e:?})");
            return false;
        },
    };
    if !parent_a.flags().is_writable() {
        error!("parent PTE at vaddr_a is not writable after rejected link");
        return false;
    }
    if parent_a.is_cow() {
        error!("parent PTE at vaddr_a unexpectedly carries CoW bit after rejected link");
        return false;
    }
    if parent_a.frame_number().into_raw_value() != frame_a_num {
        error!("parent PTE at vaddr_a points at the wrong frame after rejected link");
        return false;
    }

    // Parent's vaddr_b must be untouched as well.
    let parent_b: PageTableEntry = match parent.try_find_user_pte(vaddr_b) {
        Ok(Some(p)) => p,
        Ok(None) => {
            error!("parent PTE at vaddr_b missing after rejected link");
            return false;
        },
        Err(e) => {
            error!("try_find_user_pte(parent, vaddr_b) failed (error={e:?})");
            return false;
        },
    };
    if !parent_b.flags().is_writable() {
        error!("parent PTE at vaddr_b is not writable after rejected link");
        return false;
    }
    if parent_b.is_cow() {
        error!("parent PTE at vaddr_b unexpectedly carries CoW bit");
        return false;
    }
    if parent_b.frame_number().into_raw_value() != frame_b_num {
        error!("parent PTE at vaddr_b points at the wrong frame");
        return false;
    }

    // Child must have no mapping at vaddr_a: nothing was linked.
    match child.is_user_page_mapped(vaddr_a) {
        Ok(false) => {},
        Ok(true) => {
            error!("child unexpectedly has a mapping at vaddr_a after rejected link");
            return false;
        },
        Err(e) => {
            error!("child.is_user_page_mapped(vaddr_a) failed (error={e:?})");
            return false;
        },
    }

    // Child's pre-existing vaddr_b mapping must still point at frame_c.
    let child_b: PageTableEntry = match child.try_find_user_pte(vaddr_b) {
        Ok(Some(p)) => p,
        Ok(None) => {
            error!("child's pre-existing PTE at vaddr_b vanished");
            return false;
        },
        Err(e) => {
            error!("try_find_user_pte(child, vaddr_b) failed (error={e:?})");
            return false;
        },
    };
    if child_b.frame_number().into_raw_value() != frame_c_num {
        error!("child PTE at vaddr_b was overwritten (expected frame_c)");
        return false;
    }

    // Reclaim the frames installed in both address spaces before dropping them; as in
    // `test_cow_resolution_creates_private_frame`, dropping an address space frees only its
    // page-table structures, not the user frames their entries map. The debug assertion in
    // `Vmem::drop` requires that no user pages remain mapped at drop time.
    if let Err(e) = parent.clear_user_space() {
        error!("clear_user_space(parent) failed during test teardown (error={e:?})");
        return false;
    }
    if let Err(e) = child.clear_user_space() {
        error!("clear_user_space(child) failed during test teardown (error={e:?})");
        return false;
    }
    drop(parent);
    drop(child);

    true
}

///
/// # Description
///
/// Verifies that [`VirtMemoryManager::link_user_pages`] rolls back any pages it had
/// already linked into `child` when a later iteration fails.
///
/// The failure is forced by saturating the reference count of `frame_b` to `u8::MAX`
/// before invoking the routine. Iteration visits `vaddr_a` first (its page table entry
/// is at a lower index) and links it into `child` as CoW. The second iteration then
/// calls `share()` on `frame_b`, which overflows the refcount and returns
/// `ErrorCode::OutOfMemory`. This triggers the rollback path.
///
/// Expected post-conditions after the call returns `Err`:
///
/// - parent's `vaddr_a` PTE is read-only and copy-on-write, still pointing at `frame_a`.
///   The rollback deliberately leaves the parent's CoW mark in place because it cannot
///   reliably tell a page it marked from one that was already shared copy-on-write before
///   the call.
/// - parent's `vaddr_b` PTE is untouched (writable, not CoW, original frame).
/// - `child` has no mapping at either `vaddr_a` or `vaddr_b`.
///
fn test_link_user_pages_rolls_back_on_partial_failure() -> bool {
    const VADDR_A_RAW: usize = ::config::memory_layout::USER_MMAP_BASE_RAW;
    const VADDR_B_RAW: usize = ::config::memory_layout::USER_MMAP_BASE_RAW + ::arch::mem::PAGE_SIZE;

    let vaddr_a: PageAligned<VirtualAddress> =
        match PageAligned::<VirtualAddress>::from_raw_value(VADDR_A_RAW) {
            Ok(v) => v,
            Err(e) => {
                error!("PageAligned::from_raw_value(vaddr_a) failed (error={e:?})");
                return false;
            },
        };
    let vaddr_b: PageAligned<VirtualAddress> =
        match PageAligned::<VirtualAddress>::from_raw_value(VADDR_B_RAW) {
            Ok(v) => v,
            Err(e) => {
                error!("PageAligned::from_raw_value(vaddr_b) failed (error={e:?})");
                return false;
            },
        };

    // SAFETY: pm/init() runs after the physical and virtual memory managers are
    // initialized; access is synchronized because the kernel is single-threaded with
    // interrupts disabled.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };

    let mut parent: Vmem = {
        let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };
        match mm.new_vmem(pm.current_vmem()) {
            Ok(v) => v,
            Err(e) => {
                error!("new_vmem(parent) failed (error={e:?})");
                return false;
            },
        }
    };
    let mut child: Vmem = {
        let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };
        match mm.new_vmem(pm.current_vmem()) {
            Ok(v) => v,
            Err(e) => {
                error!("new_vmem(child) failed (error={e:?})");
                return false;
            },
        }
    };

    let frame_a: UserFrame = match unsafe { PhysMemoryManager::get_mut() }.alloc_user_frame() {
        Ok(f) => f,
        Err(e) => {
            error!("alloc_user_frame(a) failed (error={e:?})");
            return false;
        },
    };
    let frame_b: UserFrame = match unsafe { PhysMemoryManager::get_mut() }.alloc_user_frame() {
        Ok(f) => f,
        Err(e) => {
            error!("alloc_user_frame(b) failed (error={e:?})");
            return false;
        },
    };

    let frame_a_num: usize = frame_a.address().into_frame_number().into_raw_value();
    let frame_b_num: usize = frame_b.address().into_frame_number().into_raw_value();

    // Saturate frame_b's refcount so the next `share()` overflows. The fresh allocation
    // starts at refcount 1, so 254 extra references reach u8::MAX. Each new handle is
    // leaked to keep the bumped refcount in place; we never restore it (production
    // processes release frames via the explicit harvest/unmap path, mirroring the
    // disposition note on the CoW resolution tests).
    for i in 0..254 {
        match frame_b.share() {
            Ok(handle) => {
                handle.leak();
            },
            Err(e) => {
                error!("frame_b.share() failed at iteration {i} (error={e:?})");
                return false;
            },
        }
    }
    match frame_b.refcount() {
        Ok(rc) if rc == u8::MAX => {},
        Ok(rc) => {
            error!("frame_b refcount is {rc} after saturation (expected {})", u8::MAX);
            return false;
        },
        Err(e) => {
            error!("frame_b.refcount() failed (error={e:?})");
            return false;
        },
    }

    if let Err(e) = parent.map(frame_a, vaddr_a, AccessPermission::RDWR) {
        error!("parent.map(vaddr_a) failed (error={e:?})");
        return false;
    }
    if let Err(e) = parent.map(frame_b, vaddr_b, AccessPermission::RDWR) {
        error!("parent.map(vaddr_b) failed (error={e:?})");
        return false;
    }

    // Invoke the routine under test. Must fail: vaddr_a is linked, then `share()`
    // on frame_b overflows because its refcount is already u8::MAX.
    {
        let mm_mut: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
        match mm_mut.link_user_pages(&mut parent, &mut child) {
            Ok(()) => {
                error!("link_user_pages unexpectedly succeeded");
                return false;
            },
            Err(e) if e.code == ErrorCode::OutOfMemory => {},
            Err(e) => {
                error!("link_user_pages failed with unexpected error (error={e:?})");
                return false;
            },
        }
    }

    // Parent's vaddr_a: the child mapping is rolled back, but the parent's CoW mark is
    // intentionally left in place, so it is read-only + CoW, still pointing at frame_a.
    let parent_a: PageTableEntry = match parent.try_find_user_pte(vaddr_a) {
        Ok(Some(p)) => p,
        Ok(None) => {
            error!("parent PTE at vaddr_a missing after rollback");
            return false;
        },
        Err(e) => {
            error!("try_find_user_pte(parent, vaddr_a) failed (error={e:?})");
            return false;
        },
    };
    if parent_a.flags().is_writable() {
        error!("parent PTE at vaddr_a is unexpectedly writable after rollback");
        return false;
    }
    if !parent_a.is_cow() {
        error!("parent PTE at vaddr_a lost its CoW bit after rollback");
        return false;
    }
    if parent_a.frame_number().into_raw_value() != frame_a_num {
        error!("parent PTE at vaddr_a points at the wrong frame after rollback");
        return false;
    }

    // Parent's vaddr_b must be untouched.
    let parent_b: PageTableEntry = match parent.try_find_user_pte(vaddr_b) {
        Ok(Some(p)) => p,
        Ok(None) => {
            error!("parent PTE at vaddr_b missing after rollback");
            return false;
        },
        Err(e) => {
            error!("try_find_user_pte(parent, vaddr_b) failed (error={e:?})");
            return false;
        },
    };
    if !parent_b.flags().is_writable() {
        error!("parent PTE at vaddr_b is not writable after rollback");
        return false;
    }
    if parent_b.is_cow() {
        error!("parent PTE at vaddr_b unexpectedly carries CoW bit");
        return false;
    }
    if parent_b.frame_number().into_raw_value() != frame_b_num {
        error!("parent PTE at vaddr_b points at the wrong frame");
        return false;
    }

    // Child must have no mappings: vaddr_a was rolled back; vaddr_b was never reached.
    match child.is_user_page_mapped(vaddr_a) {
        Ok(false) => {},
        Ok(true) => {
            error!("child still has a mapping at vaddr_a after rollback");
            return false;
        },
        Err(e) => {
            error!("child.is_user_page_mapped(vaddr_a) failed (error={e:?})");
            return false;
        },
    }
    match child.is_user_page_mapped(vaddr_b) {
        Ok(false) => {},
        Ok(true) => {
            error!("child unexpectedly has a mapping at vaddr_b");
            return false;
        },
        Err(e) => {
            error!("child.is_user_page_mapped(vaddr_b) failed (error={e:?})");
            return false;
        },
    }

    // Reclaim the frames installed in both address spaces before dropping them (see the teardown
    // note on `test_link_user_pages_errors_on_preexisting_child_overlap`). This releases each
    // address space's own references; the 254 extra references on frame_b deliberately remain, so
    // that frame stays allocated for the duration of the test. The debug assertion in `Vmem::drop`
    // requires that no user pages remain mapped at drop time.
    if let Err(e) = parent.clear_user_space() {
        error!("clear_user_space(parent) failed during test teardown (error={e:?})");
        return false;
    }
    if let Err(e) = child.clear_user_space() {
        error!("clear_user_space(child) failed during test teardown (error={e:?})");
        return false;
    }
    drop(parent);
    drop(child);

    true
}

/// Runs all process-management in-kernel tests.
pub fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_mailbox_is_empty_tracks_buffered_messages);
    passed &= run_test!(test_kernel_process_has_no_special_resources);
    passed &= run_test!(test_cow_resolution_creates_private_frame);
    passed &= run_test!(test_cow_resolution_fast_path_when_sole_owner);
    passed &= run_test!(test_link_user_pages_errors_on_preexisting_child_overlap);
    passed &= run_test!(test_link_user_pages_rolls_back_on_partial_failure);
    passed &= super::process::test();
    passed
}
