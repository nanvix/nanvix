// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// PhysMemoryManager - Specifications
//
// `PhysMemoryManager` is a *stateless* facade over the global frame allocator
// (`Upool` carries no fields; kernel frames are drawn directly from `frame::*`).
// It therefore has no per-instance abstract state of its own: the caller-relevant
// state is the global frame-reservation state, modelled by the do-not-modify
// `phys_view()` / `FrameAllocView` in `mod.spec.rs`. All manager contracts are
// stated over `phys_view()` and the returned frame values -- exactly the monotone
// style used by the `frame::*` free-function shims (`book`, `alloc_range`, ...).
//
// See `verus-ai-logs/nanvix-phys-phys-manager/view_design.md` for why a bespoke
// `self@` View is *not* realizable here (a constant view cannot witness global
// mutation, and there is no `old(phys_view())`).

verus! {

use crate::hal::mem::spec_page_size;
use crate::mm::phys::{
    phys_view,
    FrameAllocView,
};
use vstd::seq::Seq;

/// The kernel watermark threshold, lifted to `int`.
///
/// Abstract form of the build-time constant `config::kernel::KERNEL_WATERMARK`:
/// the number of physical frames the allocator must keep in reserve for kernel
/// use, below which user allocations are refused.
pub open spec fn spec_kernel_watermark() -> int {
    config::kernel::KERNEL_WATERMARK as int
}

/// A user allocation of `count` frames is admissible only if, after servicing it,
/// at least `KERNEL_WATERMARK` frames would still be free.
///
/// Stated on the abstract free-set cardinality -- never on an allocator counter --
/// so it survives any frame-allocator reimplementation. `free_frames.len()` is
/// meaningful because the allocator keeps `free_frames` finite (see the shim
/// contracts, which (re)establish `phys_view().frames.free_frames.finite()`).
pub open spec fn spec_watermark_ok(v: FrameAllocView, count: int) -> bool {
    v.free_frames.len() >= spec_kernel_watermark() + count
}

/// `addrs` is an ascending, page-stride-contiguous run of frame addresses based
/// at `base`: `addrs[i] == base + i * PAGE_SIZE`.
///
/// Used to state the contiguity guarantee of `alloc_many_kernel_frames` over the
/// sequence of returned kernel-frame addresses (kernel stacks require a linear,
/// identity-mapped physical region).
pub open spec fn is_contiguous_run(addrs: Seq<int>, base: int) -> bool {
    forall|i: int|
        0 <= i < addrs.len() ==> #[trigger] addrs[i] == base + i * spec_page_size()
}

} // verus!
