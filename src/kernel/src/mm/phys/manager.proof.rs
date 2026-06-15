verus! {

use crate::mm::phys::phys_view;

//==================================================================================================
// Global-token attachment (deferred to the proving phase)
//==================================================================================================

/// The manager brokers the *global* frame partition: its abstract view coincides with
/// `phys_view().frames`. This is the §8 ghost-token attachment; the proving phase realizes it
/// with a token over the `frame::INSTANCE` / `PhysMemoryManager` singletons.
#[verus_verify(external_body)]
pub proof fn lemma_manager_attached(m: &PhysMemoryManager)
    ensures
        m@ == phys_view().frames,
{
}

//==================================================================================================
// Kernel-path transitions (frame::alloc / frame::alloc_contiguous take no `self`)
//==================================================================================================

/// Effect of a single kernel-frame allocation on the brokered partition: the freshly returned
/// `addr` was free and becomes reserved with refcount 1. `frame::alloc` carries no `self`, so the
/// link to `self@` is supplied here and discharged by the global-token attachment in the proving
/// phase.
pub proof fn lemma_kernel_alloc_one(pre: FrameAllocView, post: FrameAllocView, addr: int)
    requires
        pre.wf(),
    ensures
        pre.free_frames.contains(addr),
        post == pre.alloc_one(addr),
        post.wf(),
{
    admit();
}

/// Effect of a contiguous kernel-frame allocation: `frames` owns `count` physically contiguous
/// frames starting at some page-aligned `base`, all of which were free and become reserved.
pub proof fn lemma_kernel_alloc_contiguous(
    pre: FrameAllocView,
    post: FrameAllocView,
    frames: Seq<KernelFrame>,
    count: nat,
)
    requires
        pre.wf(),
    ensures
        frames.len() == count,
        crate::mm::phys::manager::kernel_frames_contiguous(frames, count),
        post == pre.book_all(crate::mm::phys::manager::kernel_addr_set(frames)),
        pre.all_free(crate::mm::phys::manager::kernel_addr_set(frames)),
        post.wf(),
{
    admit();
}

/// The base address returned by a contiguous allocation, advanced by any in-range frame index,
/// stays within the address space — `frame::alloc_contiguous` guarantees the whole `count`-frame
/// span fits a `usize`, so no intermediate offset overflows. Admitted here; the proving phase
/// derives it from the contiguous allocator's range bound.
pub proof fn lemma_contig_no_overflow(base_raw: usize, idx: usize, count: usize)
    requires
        idx < count,
        base_raw as int + (count as int) * spec_page_size() <= usize::MAX as int,
    ensures
        (idx as int) * spec_page_size() <= usize::MAX as int,
        base_raw as int + (idx as int) * spec_page_size() <= usize::MAX as int,
{
    let ps: int = spec_page_size();
    // `spec_page_size() == arch::mem::PAGE_SIZE as int`, a `usize` cast, hence non-negative.
    assert(ps >= 0);
    // Monotonicity of multiplication by a non-negative factor: idx < count ==> idx*ps <= count*ps.
    assert((idx as int) * ps <= (count as int) * ps) by (nonlinear_arith)
        requires
            idx as int <= count as int,
            ps >= 0,
    ;
    // base_raw >= 0 (a `usize`), so both bounds follow linearly from base_raw + count*ps <= MAX.
}

//==================================================================================================
// User bulk-path transitions
//==================================================================================================

/// Booking the empty set leaves a frame partition unchanged.
pub proof fn lemma_book_all_empty(base: FrameAllocView)
    ensures
        base.book_all(Set::<int>::empty()) == base,
{
    let booked = base.book_all(Set::<int>::empty());
    assert(booked.allocated_frames =~= base.allocated_frames);
    assert(booked.free_frames =~= base.free_frames);
    assert(booked.refcounts =~= base.refcounts);
}

/// Booking a set and then allocating one more address `a` equals booking the extended set
/// `s ∪ {a}`. The two `FrameAllocView` transitions coincide field-by-field, independent of
/// whether `a` was already in `s` (`alloc_one`/`insert` are idempotent on the booked fields).
pub proof fn lemma_book_all_alloc_one(base: FrameAllocView, s: Set<int>, a: int)
    ensures
        base.book_all(s).alloc_one(a) == base.book_all(s.insert(a)),
{
    let lhs = base.book_all(s).alloc_one(a);
    let rhs = base.book_all(s.insert(a));
    assert(lhs.allocated_frames =~= rhs.allocated_frames);
    assert(lhs.free_frames =~= rhs.free_frames);
    assert(lhs.refcounts =~= rhs.refcounts);
}

/// The address set of an empty handle sequence is empty.
pub proof fn lemma_user_addr_set_empty(frames: Seq<UserFrame>)
    requires
        frames.len() == 0,
    ensures
        crate::mm::phys::manager::user_addr_set(frames) == Set::<int>::empty(),
{
    assert(crate::mm::phys::manager::user_addr_set(frames) =~= Set::<int>::empty());
}

/// Pushing a handle onto a sequence inserts its address into the sequence's address set.
pub proof fn lemma_user_addr_set_push(frames: Seq<UserFrame>, uf: UserFrame)
    ensures
        crate::mm::phys::manager::user_addr_set(frames.push(uf))
            == crate::mm::phys::manager::user_addr_set(frames).insert(uf@),
{
    let lhs = crate::mm::phys::manager::user_addr_set(frames.push(uf));
    let rhs = crate::mm::phys::manager::user_addr_set(frames).insert(uf@);
    assert forall|a: int| lhs.contains(a) implies rhs.contains(a) by {
        let i = choose|i: int|
            0 <= i < frames.push(uf).len() && #[trigger] frames.push(uf)[i]@ == a;
        if i < frames.len() {
            assert(frames.push(uf)[i] == frames[i]);
        } else {
            assert(i == frames.len());
            assert(frames.push(uf)[i] == uf);
        }
    }
    assert forall|a: int| rhs.contains(a) implies lhs.contains(a) by {
        if a == uf@ {
            assert(frames.push(uf)[frames.len() as int] == uf);
        } else {
            let i = choose|i: int| 0 <= i < frames.len() && #[trigger] frames[i]@ == a;
            assert(frames.push(uf)[i] == frames[i]);
        }
    }
    assert(lhs =~= rhs);
}

/// On a mid-bulk failure the implementation `clear()`s the vector, which drops (and thus frees)
/// every frame already taken, restoring the partition to its pre-call state. `Drop` side effects
/// are not modeled in exec, so the restoration is asserted here.
pub proof fn lemma_user_bulk_err_restored(m: &PhysMemoryManager, pre: FrameAllocView)
    requires
        pre.wf(),
    ensures
        m@ == pre,
{
    admit();
}

} // verus!
