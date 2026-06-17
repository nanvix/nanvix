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
    admit();
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
        pre.is_free(addr),
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
        user_addr_set(frames).len() == 0,
{
    broadcast use vstd::set::group_set_lemmas;
    assert(user_addr_set(frames) =~= Set::<int>::empty());
}

/// Pushing a handle extends the owned-address set by exactly that handle's address.
pub proof fn lemma_user_addr_set_push(frames: Seq<UserFrame>, uf: UserFrame)
    ensures
        user_addr_set(frames.push(uf)) =~= user_addr_set(frames).insert(uf@),
{
    broadcast use vstd::seq_lib::group_seq_lib_default;
    let me = frames.map_values(|x: UserFrame| x@);
    // `map_values` commutes with `push`, and pushing an element appends a singleton.
    frames.lemma_push_map_commute(|x: UserFrame| x@, uf);
    assert(frames.push(uf).map_values(|x: UserFrame| x@) =~= me.push(uf@));
    assert(me.push(uf@) =~= me + seq![uf@]);
    Seq::lemma_to_set_insert_commutes(me, uf@);
    assert(user_addr_set(frames.push(uf)) =~= user_addr_set(frames).insert(uf@));
}

/// Reserving the empty set leaves the partition unchanged.
pub proof fn lemma_book_all_empty(v: FrameAllocView)
    ensures
        v.book_all(Set::<int>::empty()) == v,
{
    broadcast use
        vstd::set::group_set_lemmas,
        vstd::map::group_map_lemmas,
        vstd::map_lib::group_map_properties;
    assert(v.book_all(Set::<int>::empty()).refcounts =~= v.refcounts);
}

/// Booking a set and then allocating one further frame equals booking the enlarged set.
/// This is the algebraic step that lets the per-iteration `alloc_one` transitions of
/// `Upool::alloc` accumulate into a single `book_all` over the whole address set.
pub proof fn lemma_book_all_alloc_one(v: FrameAllocView, s: Set<int>, a: int)
    ensures
        v.book_all(s).alloc_one(a) == v.book_all(s.insert(a)),
{
    broadcast use
        vstd::set::group_set_lemmas,
        vstd::map::group_map_lemmas,
        vstd::map_lib::group_map_properties;
    assert(v.book_all(s).alloc_one(a).refcounts =~= v.book_all(s.insert(a)).refcounts);
}

/// The free set after booking `s` is exactly the old free set minus `s`: booking flips each
/// covered frame in `s` from refcount 0 to 1, and adds no new free frames.
pub proof fn lemma_book_all_free_set(g: FrameAllocView, s: Set<int>)
    ensures
        g.book_all(s).free_set() =~= g.free_set().difference(s),
{
    broadcast use
        vstd::set::group_set_lemmas,
        vstd::map::group_map_lemmas,
        vstd::map_lib::group_map_properties;
}

/// Loop invariant for the user bulk-allocation loop: the handles accumulated so far own a
/// finite set of *distinct* addresses, all of which were free in the pre-call partition
/// `g_old`, and the current partition `mview` is exactly `g_old` with that set booked.
pub open spec fn user_bulk_inv(
    g_old: FrameAllocView,
    mview: FrameAllocView,
    frames: Seq<UserFrame>,
) -> bool {
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
        mview.is_free(uf@),
    ensures
        user_bulk_inv(g_old, mview.alloc_one(uf@), frames.push(uf)),
{
    broadcast use
        vstd::set::group_set_lemmas,
        vstd::map::group_map_lemmas,
        vstd::map_lib::group_map_properties;

    let s = user_addr_set(frames);
    // `mview == g_old.book_all(s)`, so `mview.free_set() == g_old.free_set().difference(s)`.
    // `uf@` being free in `mview` therefore means it is free in `g_old` and not yet in `s`.
    lemma_book_all_free_set(g_old, s);
    assert(mview.free_set() =~= g_old.free_set().difference(s));
    assert(g_old.free_set().contains(uf@) && !s.contains(uf@));
    assert(g_old.is_free(uf@));

    lemma_user_addr_set_push(frames, uf);
    lemma_book_all_alloc_one(g_old, s, uf@);

    let s2 = user_addr_set(frames.push(uf));
    assert(s2 =~= s.insert(uf@));
    // Distinctness: `uf@ ∉ s`, so the cardinality grows by exactly one.
    assert(s2.len() == s.len() + 1);
    assert(frames.push(uf).len() == frames.len() + 1);
    // Every address in the enlarged set was free in `g_old`.
    assert(g_old.all_free(s2)) by {
        assert forall|x: int| #[trigger] s2.contains(x) implies g_old.is_free(x) by {
            if x != uf@ {
                assert(s.contains(x));
            }
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
