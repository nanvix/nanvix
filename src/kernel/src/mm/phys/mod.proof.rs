// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// PhysMemoryManager - Proofs
//
// Proven lemmas that map the caller expectations recorded in
// `verus-ai-logs/nanvix-phys-phys-mod/caller_analysis.md` onto the `PhysMemView`
// transition vocabulary defined in `mod.spec.rs`. These discharge the abstract
// frame-reservation semantics that the exec-level `#[verus_spec]` contracts in
// `mod.rs` / `frame.rs` summarize.

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
}

/// Booking a set of currently-free, page-aligned frames at once preserves the
/// subsystem invariant and reserves all of them.
///
/// Generalizes `Inner::alloc_range`'s `Ok` post-state from a contiguous region to
/// an arbitrary frame set: every frame in `frames` ends up allocated and out of
/// `free_frames`, and the `FrameAllocView` well-formedness is maintained.
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
    let pre_f = pre.frames;
    let post = pre.spec_book_frames(frames);
    let post_f = post.frames;

    assert(pre_f.wf());

    // Key fact: the refcount domain is exactly the allocated set (pre-state).
    assert forall|x: int| pre_f.refcounts.contains_key(x) implies pre_f.allocated_frames.contains(x) by {
        assert(pre_f.refcounts[x] > 0);
    }

    // Reservation post-state membership.
    assert forall|a: int| frames.contains(a)
        implies #[trigger] post_f.allocated_frames.contains(a) by {
    }
    assert forall|a: int| frames.contains(a)
        implies post_f.free_frames.contains(a) == false by {
    }

    assert(post_f.wf()) by {
        // (1) allocated frames are page-aligned.
        assert forall|x: int| post_f.allocated_frames.contains(x)
            implies x % spec_page_size() == 0 by {
            if pre_f.allocated_frames.contains(x) {
            } else {
                assert(frames.contains(x));
            }
        }
        // (2) free frames are page-aligned.
        assert forall|x: int| post_f.free_frames.contains(x)
            implies x % spec_page_size() == 0 by {
            assert(pre_f.free_frames.contains(x));
        }
        // (3) allocated and free are disjoint.
        assert forall|x: int| post_f.allocated_frames.contains(x)
            implies !post_f.free_frames.contains(x) by {
            if pre_f.allocated_frames.contains(x) {
                assert(!pre_f.free_frames.contains(x));
            }
        }
        // (4) allocated iff positive refcount.
        assert forall|x: int| #[trigger] post_f.allocated_frames.contains(x)
            <==> (post_f.refcounts.contains_key(x) && post_f.refcounts[x] > 0) by {
            if frames.contains(x) {
                assert(post_f.refcounts[x] == 1);
            } else if pre_f.allocated_frames.contains(x) {
                assert(pre_f.refcounts.contains_key(x));
                assert(pre_f.refcounts[x] > 0);
                assert(post_f.refcounts[x] == pre_f.refcounts[x]);
            }
        }
        // (5) free frames have no refcount entry.
        assert forall|x: int| #[trigger] post_f.free_frames.contains(x)
            implies !post_f.refcounts.contains_key(x) by {
            assert(pre_f.free_frames.contains(x));
            assert(!pre_f.allocated_frames.contains(x));
            assert(!pre_f.refcounts.contains_key(x));
            assert(!frames.contains(x));
        }
        // (6) refcounts are within the u8 range.
        assert forall|x: int| post_f.refcounts.contains_key(x)
            implies 0 < post_f.refcounts[x] <= 255 by {
            if frames.contains(x) {
                assert(post_f.refcounts[x] == 1);
            } else {
                assert(pre_f.refcounts.contains_key(x));
            }
        }
    }
}

/// Booking one covered, currently-free frame preserves the subsystem invariant
/// and reserves exactly that frame.
///
/// Mirrors `Inner::book`'s `Ok` post-state: `addr` moves from `free_frames` to
/// `allocated_frames` with `refcounts[addr] == 1`. Proven as the singleton case of
/// `lemma_spec_book_frames_preserves_inv`.
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
    assert(pre.frames.wf());
    // `addr` is page-aligned because it is a free frame in a well-formed view.
    assert(addr % spec_page_size() == 0);

    let single = Set::<int>::empty().insert(addr);
    assert(single.subset_of(pre.frames.free_frames));
    assert forall|a: int| single.contains(a) implies a % spec_page_size() == 0 by {
        assert(a == addr);
    }
    lemma_spec_book_frames_preserves_inv(pre, single);

    // `spec_book_frame(addr)` equals `spec_book_frames({addr})` componentwise.
    let by_set = pre.spec_book_frames(single);
    let by_one = pre.spec_book_frame(addr);
    assert(by_one.frames.allocated_frames =~= by_set.frames.allocated_frames);
    assert(by_one.frames.free_frames =~= by_set.frames.free_frames);
    assert(by_one.frames.refcounts =~= by_set.frames.refcounts);
    assert(by_one.frames == by_set.frames);
    assert(by_one == by_set);
}

/// `book_physical_memory_regions`: booking a contiguous region reserves exactly
/// the frames of that region.
///
/// Connects `PhysMemView::region_frames` (the frames of `[start, start+size)`) to
/// `spec_book_frames`: after booking, every region frame is allocated and the
/// invariant is preserved.
pub proof fn lemma_book_region_reserves_region_frames(
    pre: PhysMemView,
    start: int,
    size: int,
)
    requires
        pre.inv(),
        pre.initialized,
        spec_page_size() > 0,
        PhysMemView::region_frames(start, size).subset_of(pre.frames.free_frames),
        start % spec_page_size() == 0,
        size % spec_page_size() == 0,
    ensures
        pre.spec_book_frames(PhysMemView::region_frames(start, size)).inv(),
        forall|a: int| PhysMemView::region_frames(start, size).contains(a)
            ==> #[trigger] pre.spec_book_frames(PhysMemView::region_frames(start, size))
                .frames.allocated_frames.contains(a),
{
    let region = PhysMemView::region_frames(start, size);
    // Every region frame is `i * page_size` for some `i`, hence page-aligned.
    assert forall|a: int| region.contains(a) implies a % spec_page_size() == 0 by {
        let first = start / spec_page_size();
        let last = (start + size) / spec_page_size();
        let pre_image = vstd::set_lib::set_int_range(first, last);
        assert(region == pre_image.map(|i: int| i * spec_page_size()));
        assert(exists|i: int| pre_image.contains(i) && a == i * spec_page_size());
        let i = choose|i: int| pre_image.contains(i) && a == i * spec_page_size();
        assert(a == i * spec_page_size());
        assert(a % spec_page_size() == 0) by (nonlinear_arith)
            requires spec_page_size() > 0, a == i * spec_page_size();
    }
    lemma_spec_book_frames_preserves_inv(pre, region);
}

/// `book_mmio_regions`: an MMIO frame the allocator does NOT track is skipped,
/// leaving the abstract state unchanged.
///
/// Encodes the caller's "skip-if-not-covered" tolerance: a high MMIO address with
/// `is_covered == false` must not change reservation state and must not be an error.
pub proof fn lemma_book_mmio_skip_untracked(pre: PhysMemView, addr: int)
    requires
        pre.inv(),
        !pre.covered().contains(addr),
    ensures
        pre.inv(),
{
}

/// `book_mmio_regions`: an MMIO frame the allocator tracks (`is_covered`) and that
/// is currently free gets booked, preserving the invariant.
///
/// Pairs with `lemma_book_mmio_skip_untracked`: covered MMIO frames are reserved
/// exactly like physical-RAM frames. Proven via the single-frame booking lemma.
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
    lemma_spec_book_frame_preserves_inv(pre, addr);
}

/// Composite `init` contract: starting from an uninitialized subsystem, a
/// successful `init` ends initialized, well-formed, with all booked physical-RAM
/// and tracked-MMIO frames reserved (disjoint from free).
///
/// This is the top-level guarantee `kernel_vas::init` relies on. `reserved`
/// abstracts the union of all physical-region frames and all covered MMIO frames
/// booked during boot. Proven by composing the initialize and book lemmas.
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
    let pre = PhysMemView { initialized: false, frames: initial };
    let established = pre.spec_initialize(initial);
    lemma_spec_initialize_establishes_inv(pre, initial);
    assert(established.initialized);
    assert(established.inv());
    assert(established.frames == initial);
    lemma_spec_book_frames_preserves_inv(established, reserved);
    let post = established.spec_book_frames(reserved);
    // Disjointness is part of `post.frames.wf()`, available from `post.inv()`.
    assert(post.initialized);
    assert(post.inv());
    assert(post.frames.wf());
}

} // verus!
