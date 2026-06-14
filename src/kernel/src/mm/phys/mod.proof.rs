// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// PhysMemoryManager - Proofs
//
// Lemma signatures that map the caller expectations recorded in
// `verus-ai-logs/nanvix-phys-phys-mod/caller_analysis.md` onto the `PhysMemView`
// transition vocabulary defined in `mod.spec.rs`. Bodies are deferred to the
// proving phase (`admit()`); the signatures fix the contracts those proofs must
// discharge.

verus! {

/// `init` *establishes* (not merely preserves) the allocator invariant.
///
/// Caller expectation: "after `Ok`, `instance().inv()` (i.e. `FrameAllocView::wf`)
/// holds for all later `mm::phys` operations." Booting from a well-formed initial
/// bitmap state yields an initialized, well-formed `PhysMemView`.
pub proof fn lemma_spec_initialize_establishes_inv(pre: PhysMemView, initial: FrameAllocView)
    requires
        initial.wf(),
    ensures
        pre.spec_initialize(initial).initialized,
        pre.spec_initialize(initial).inv(),
        pre.spec_initialize(initial).frames == initial,
{
    admit();
}

/// Booking one covered, currently-free frame preserves the subsystem invariant
/// and reserves exactly that frame.
///
/// Mirrors `Inner::book`'s `Ok` post-state: `addr` moves from `free_frames` to
/// `allocated_frames` with `refcounts[addr] == 1`, leaving every other frame
/// untouched.
pub proof fn lemma_spec_book_frame_preserves_inv(pre: PhysMemView, addr: int)
    requires
        pre.inv(),
        pre.initialized,
        pre.frames.free_frames.contains(addr),
    ensures
        pre.spec_book_frame(addr).inv(),
        pre.spec_book_frame(addr).initialized,
        pre.spec_book_frame(addr).frames.allocated_frames.contains(addr),
        !pre.spec_book_frame(addr).frames.free_frames.contains(addr),
        pre.spec_book_frame(addr).frames.refcounts[addr] == 1,
{
    admit();
}

/// Booking a set of currently-free frames at once preserves the invariant and
/// reserves all of them.
///
/// Generalizes `Inner::alloc_range`'s `Ok` post-state from a contiguous region
/// to an arbitrary frame set: every frame in `frames` ends up allocated and out
/// of `free_frames`.
pub proof fn lemma_spec_book_frames_preserves_inv(pre: PhysMemView, frames: Set<int>)
    requires
        pre.inv(),
        pre.initialized,
        frames.subset_of(pre.frames.free_frames),
        forall|a: int| frames.contains(a) ==> a % spec_page_size() == 0,
    ensures
        pre.spec_book_frames(frames).inv(),
        pre.spec_book_frames(frames).initialized,
        forall|a: int| frames.contains(a)
            ==> #[trigger] pre.spec_book_frames(frames).frames.allocated_frames.contains(a),
        forall|a: int| frames.contains(a)
            ==> #[trigger] pre.spec_book_frames(frames).frames.free_frames.contains(a) == false,
{
    admit();
}

/// `book_physical_memory_regions`: booking a contiguous region reserves exactly
/// the frames of that region.
///
/// Connects `PhysMemView::region_frames` (the frames of `[start, start+size)`)
/// to `spec_book_frames`: after booking, every region frame is allocated.
pub proof fn lemma_book_region_reserves_region_frames(
    pre: PhysMemView,
    start: int,
    size: int,
)
    requires
        pre.inv(),
        pre.initialized,
        PhysMemView::region_frames(start, size).subset_of(pre.frames.free_frames),
        start % spec_page_size() == 0,
        size % spec_page_size() == 0,
    ensures
        pre.spec_book_frames(PhysMemView::region_frames(start, size)).inv(),
        forall|a: int| PhysMemView::region_frames(start, size).contains(a)
            ==> #[trigger] pre.spec_book_frames(PhysMemView::region_frames(start, size))
                .frames.allocated_frames.contains(a),
{
    admit();
}

/// `book_mmio_regions`: an MMIO frame the allocator does NOT track is skipped,
/// leaving the abstract state unchanged.
///
/// Encodes the caller's "skip-if-not-covered" tolerance: a high MMIO address
/// (e.g. LAPIC above RAM) with `is_covered == false` must not change reservation
/// state and must not be an error.
pub proof fn lemma_book_mmio_skip_untracked(pre: PhysMemView, addr: int)
    requires
        pre.inv(),
        !pre.covered().contains(addr),
    ensures
        pre.inv(),
{
    admit();
}

/// `book_mmio_regions`: an MMIO frame the allocator tracks (`is_covered`) and
/// that is currently free gets booked, preserving the invariant.
///
/// Pairs with `lemma_book_mmio_skip_untracked`: covered MMIO frames are reserved
/// exactly like physical-RAM frames.
pub proof fn lemma_book_mmio_books_tracked(pre: PhysMemView, addr: int)
    requires
        pre.inv(),
        pre.initialized,
        pre.covered().contains(addr),
        pre.frames.free_frames.contains(addr),
    ensures
        pre.spec_book_frame(addr).inv(),
        pre.spec_book_frame(addr).frames.allocated_frames.contains(addr),
{
    admit();
}

/// Composite `init` contract: starting from an uninitialized subsystem, a
/// successful `init` ends initialized, well-formed, with all booked
/// physical-RAM and tracked-MMIO frames reserved (disjoint from free).
///
/// This is the top-level guarantee `kernel_vas::init` relies on. `reserved`
/// abstracts the union of all physical-region frames and all covered MMIO
/// frames booked during boot.
pub proof fn lemma_init_establishes_and_reserves(
    initial: FrameAllocView,
    reserved: Set<int>,
)
    requires
        initial.wf(),
        reserved.subset_of(initial.free_frames),
        forall|a: int| reserved.contains(a) ==> a % spec_page_size() == 0,
    ensures
        ({
            let pre = PhysMemView { initialized: false, frames: initial };
            let post = pre.spec_initialize(initial).spec_book_frames(reserved);
            &&& post.initialized
            &&& post.inv()
            &&& forall|a: int| reserved.contains(a)
                ==> #[trigger] post.frames.allocated_frames.contains(a)
            &&& post.frames.allocated_frames.disjoint(post.frames.free_frames)
        }),
{
    admit();
}

} // verus!
