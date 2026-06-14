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

/// Effect of a successful bulk user allocation: the `count` handles in `frames` own distinct
/// frames that were all free and become reserved as a set.
pub proof fn lemma_user_bulk_ok(
    pre: FrameAllocView,
    post: FrameAllocView,
    frames: Seq<UserFrame>,
    count: nat,
)
    requires
        pre.wf(),
    ensures
        frames.len() == count,
        crate::mm::phys::manager::user_addr_set(frames).len() == count,
        post == pre.book_all(crate::mm::phys::manager::user_addr_set(frames)),
        pre.all_free(crate::mm::phys::manager::user_addr_set(frames)),
        post.wf(),
{
    admit();
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
