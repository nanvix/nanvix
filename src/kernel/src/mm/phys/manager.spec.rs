verus! {

use crate::hal::mem::spec_page_size;
use crate::mm::phys::FrameAllocView;

// `Result::and_then`: on `Ok`, the closure is applied to the payload (its precondition must hold);
// on `Err`, the error is forwarded unchanged. Mirrors `core`'s implementation. vstd ships no
// specification for it.
pub assume_specification<T, E, U, F: FnOnce(T) -> Result<U, E>>[ Result::<T, E>::and_then ](
    result: Result<T, E>,
    op: F,
) -> (res: Result<U, E>)
    requires
        result is Ok ==> op.requires((result->Ok_0,)),
    ensures
        result is Err ==> res == Err::<U, E>(result->Err_0),
        result is Ok ==> op.ensures((result->Ok_0,), res),
;

// `Result::inspect_err`: runs a side-effecting closure on the error (if any) and returns the
// receiver unchanged. The closure is used purely for its effects, so no abstract obligation is
// threaded. vstd ships no specification for it.
pub assume_specification<T, E, F: FnOnce(&E)>[ Result::<T, E>::inspect_err ](
    result: Result<T, E>,
    f: F,
) -> (res: Result<T, E>)
    ensures
        res == result,
;

// `Vec::capacity`: reports spare storage; opaque with respect to the abstract sequence. vstd
// ships no specification for it.
pub assume_specification<T, A: ::core::alloc::Allocator>[ Vec::<T, A>::capacity ](
    vec: &Vec<T, A>,
) -> (c: usize)
;

//==================================================================================================
// Build-time kernel watermark
//==================================================================================================

/// The kernel watermark: the number of physical frames the kernel always keeps free for itself.
///
/// User allocations are gated by this threshold; kernel allocations bypass it. Mirrors the
/// build-time constant `config::kernel::KERNEL_WATERMARK` so specs need not thread the value.
pub open spec fn spec_kernel_watermark() -> nat {
    config::kernel::KERNEL_WATERMARK as nat
}

//==================================================================================================
// Allocation / watermark vocabulary (on the existing FrameAllocView)
//==================================================================================================

impl FrameAllocView {
    /// Number of frames currently free, i.e. available to hand out. This is the single quantity
    /// the kernel watermark reads. Models `frame::free_count()`.
    pub open spec fn free_count(self) -> nat {
        self.free_frames.len()
    }

    /// A user allocation of `count` frames is admissible: fulfilling it would still leave at
    /// least `KERNEL_WATERMARK` frames free for the kernel. This is the predicate behind the
    /// user-vs-kernel asymmetry that `check_user_watermark` enforces.
    pub open spec fn user_alloc_ok(self, count: nat) -> bool {
        self.free_count() >= count + spec_kernel_watermark()
    }

    /// Allocate a single currently-free frame `addr`: move it from `free_frames` to
    /// `allocated_frames` with refcount 1. (Equivalent to `book_all(set![addr])`.)
    pub open spec fn alloc_one(self, addr: int) -> FrameAllocView {
        FrameAllocView {
            allocated_frames: self.allocated_frames.insert(addr),
            free_frames: self.free_frames.remove(addr),
            refcounts: self.refcounts.insert(addr, 1int),
        }
    }
}

//==================================================================================================
// Manager view
//==================================================================================================

impl View for PhysMemoryManager {
    type V = FrameAllocView;

    /// The manager brokers the global physical-frame partition. Its abstract state is exactly
    /// that partition, realized here through the user page pool it owns; the attachment to the
    /// global frame allocator (`self@ == phys_view().frames`) is pinned in the proving phase.
    closed spec fn view(&self) -> FrameAllocView {
        self.upool@
    }
}

impl PhysMemoryManager {
    /// Well-formedness invariant: the brokered frame partition is well formed (free/allocated
    /// disjoint, page-aligned, refcount <-> allocated consistent, refcounts in 1..=255).
    ///
    /// Liveness is structural (a `&mut self` is only obtainable after `init` succeeded) and the
    /// watermark is a per-user-allocation gate, not an invariant — kernel allocations are
    /// designed to dip below it — so neither appears here.
    pub open spec fn inv(&self) -> bool {
        self@.wf()
    }
}

//==================================================================================================
// Handle-set helpers
//==================================================================================================

/// The set of physical frame addresses owned by a sequence of user-frame handles.
pub open spec fn user_addr_set(frames: Seq<UserFrame>) -> Set<int> {
    Set::new(|a: int| exists|i: int| 0 <= i < frames.len() && #[trigger] frames[i]@ == a)
}

/// The set of physical frame addresses owned by a sequence of kernel-frame handles.
pub open spec fn kernel_addr_set(frames: Seq<KernelFrame>) -> Set<int> {
    Set::new(|a: int| exists|i: int| 0 <= i < frames.len() && #[trigger] frames[i]@ == a)
}

} // verus!
