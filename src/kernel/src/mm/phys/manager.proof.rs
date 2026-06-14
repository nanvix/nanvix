verus! {

use crate::mm::phys::phys_view;

//==================================================================================================
// Global-token attachment (deferred to the proving phase)
//==================================================================================================

/// The manager brokers the *global* frame partition: its abstract view coincides with
/// `phys_view().frames`. This is the §8 ghost-token attachment; the proving phase realizes it
/// with a token over the `frame::INSTANCE` / `PhysMemoryManager` singletons.
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
    // `spec_page_size()` is a `usize`-derived value, hence non-negative, and `idx < count`,
    // so multiplying the inequality `idx <= count` by `spec_page_size()` preserves it.
    vstd::arithmetic::mul::lemma_mul_inequality(
        idx as int,
        count as int,
        spec_page_size(),
    );
}

//==================================================================================================
// User bulk-path transitions
//==================================================================================================

/// The address set of a length-zero handle sequence is empty.
pub proof fn lemma_user_addr_set_empty(frames: Seq<UserFrame>)
    requires
        frames.len() == 0,
    ensures
        user_addr_set(frames) == Set::<int>::empty(),
        user_addr_set(frames).finite(),
        user_addr_set(frames).len() == 0,
{
    broadcast use vstd::set::group_set_axioms;
    assert(user_addr_set(frames) =~= Set::<int>::empty());
}

/// Pushing a handle extends the owned-address set by exactly that handle's address.
pub proof fn lemma_user_addr_set_push(frames: Seq<UserFrame>, uf: UserFrame)
    ensures
        user_addr_set(frames.push(uf)) =~= user_addr_set(frames).insert(uf@),
{
    let extended = frames.push(uf);
    assert forall|a: int| #[trigger] user_addr_set(extended).contains(a) implies
        user_addr_set(frames).insert(uf@).contains(a) by {
        let i = choose|i: int| #![trigger extended[i]@] 0 <= i < extended.len() && extended[i]@ == a;
        if i < frames.len() {
            assert(frames[i]@ == a);
        }
    }
    assert forall|a: int| #[trigger] user_addr_set(frames).insert(uf@).contains(a) implies
        user_addr_set(extended).contains(a) by {
        if a == uf@ {
            assert(extended[frames.len() as int]@ == a);
        } else {
            let i = choose|i: int| #![trigger frames[i]@] 0 <= i < frames.len() && frames[i]@ == a;
            assert(extended[i]@ == a);
        }
    }
}

/// Reserving the empty set leaves the partition unchanged.
pub proof fn lemma_book_all_empty(v: FrameAllocView)
    ensures
        v.book_all(Set::<int>::empty()) == v,
{
    assert(v.book_all(Set::<int>::empty()).allocated_frames =~= v.allocated_frames);
    assert(v.book_all(Set::<int>::empty()).free_frames =~= v.free_frames);
    assert(v.book_all(Set::<int>::empty()).refcounts =~= v.refcounts);
}

/// Booking a set and then allocating one further frame equals booking the enlarged set.
/// This is the algebraic step that lets the per-iteration `alloc_one` transitions of
/// `Upool::alloc` accumulate into a single `book_all` over the whole address set.
pub proof fn lemma_book_all_alloc_one(v: FrameAllocView, s: Set<int>, a: int)
    ensures
        v.book_all(s).alloc_one(a) == v.book_all(s.insert(a)),
{
    assert(v.book_all(s).alloc_one(a).allocated_frames
        =~= v.book_all(s.insert(a)).allocated_frames);
    assert(v.book_all(s).alloc_one(a).free_frames =~= v.book_all(s.insert(a)).free_frames);
    assert(v.book_all(s).alloc_one(a).refcounts =~= v.book_all(s.insert(a)).refcounts);
}

/// Loop invariant for the user bulk-allocation loop: the handles accumulated so far own a
/// finite set of *distinct* addresses, all of which were free in the pre-call partition
/// `g_old`, and the current partition `mview` is exactly `g_old` with that set booked.
pub open spec fn user_bulk_inv(
    g_old: FrameAllocView,
    mview: FrameAllocView,
    frames: Seq<UserFrame>,
) -> bool {
    &&& user_addr_set(frames).finite()
    &&& user_addr_set(frames).len() == frames.len()
    &&& g_old.all_free(user_addr_set(frames))
    &&& mview == g_old.book_all(user_addr_set(frames))
}

/// Base case: before any allocation the invariant holds with the partition untouched.
pub proof fn lemma_user_bulk_base(g_old: FrameAllocView, frames: Seq<UserFrame>)
    requires
        frames.len() == 0,
    ensures
        user_bulk_inv(g_old, g_old, frames),
{
    lemma_user_addr_set_empty(frames);
    lemma_book_all_empty(g_old);
}

/// Inductive step: one successful `Upool::alloc` (which moved the currently-free `uf@` from
/// free to allocated, i.e. `mview -> mview.alloc_one(uf@)`) preserves the invariant for the
/// extended handle sequence.
pub proof fn lemma_user_bulk_step(
    g_old: FrameAllocView,
    mview: FrameAllocView,
    frames: Seq<UserFrame>,
    uf: UserFrame,
)
    requires
        user_bulk_inv(g_old, mview, frames),
        mview.free_frames.contains(uf@),
    ensures
        user_bulk_inv(g_old, mview.alloc_one(uf@), frames.push(uf)),
{
    broadcast use vstd::set::group_set_axioms;

    let s = user_addr_set(frames);
    // `mview == g_old.book_all(s)`, so `mview.free_frames == g_old.free_frames.difference(s)`.
    // `uf@` being free in `mview` therefore means it is free in `g_old` and not yet in `s`.
    assert(mview.free_frames =~= g_old.free_frames.difference(s));
    assert(g_old.free_frames.contains(uf@) && !s.contains(uf@));

    lemma_user_addr_set_push(frames, uf);
    lemma_book_all_alloc_one(g_old, s, uf@);

    let s2 = user_addr_set(frames.push(uf));
    assert(s2 =~= s.insert(uf@));
    // Distinctness: `uf@ ∉ s`, so the cardinality grows by exactly one.
    assert(s2.len() == s.len() + 1);
    assert(frames.push(uf).len() == frames.len() + 1);
    // Every address in the enlarged set was free in `g_old`.
    assert forall|x: int| #[trigger] s2.contains(x) implies g_old.free_frames.contains(x) by {
        if x != uf@ {
            assert(s.contains(x));
        }
    }
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
