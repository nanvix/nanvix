// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// KernelFrame - Specifications
//
// `KernelFrame` is an owned, move-only RAII handle to a single page-sized
// physical frame allocated for kernel use and identity-mapped into the kernel
// address space. Its abstract value is exactly the base physical address of the
// frame it owns (`KernelFrame@ : int`, defined by the `View` impl in
// `kframe.rs`). No caller inspects storage layout; frames are named purely
// through `@` / `base()`.
//
// All mutable allocator state (which frames are allocated, per-frame refcounts)
// lives in the do-not-modify `FrameAllocView` reached through `phys_view()`
// (`mod.spec.rs`). Because `phys_view()` is a zero-argument uninterpreted spec
// function (a single fixed value at the current program point), the `Drop`
// effect is expressed as a monotone single-state fact over `phys_view()`,
// exactly like the `frame::free` shim contract -- there is no
// `old(phys_view())` to state a before/after transition against.

verus! {

// Bring the global physical-memory view into scope so the `Drop` contract in
// `kframe.rs` can state invariant preservation over `phys_view()`.
use crate::mm::phys::phys_view;

// `<PageAligned<PhysicalAddress> as Address>::from_raw_value` is a trait method
// of the external `sys::mm::Address` trait, below this module's verification
// boundary. `KernelFrame::new` calls it only to obtain the page-aligned physical
// address it then identity-maps; no abstract `mm::phys` fact depends on the
// returned value, so a trivial trusted contract suffices to make the call
// verifiable. Replaced when `hal::mem` is verified.
pub assume_specification<T: crate::hal::mem::Address> [
    <crate::hal::mem::PageAligned<T> as crate::hal::mem::Address>::from_raw_value
](raw_addr: usize) -> (result: Result<
    crate::hal::mem::PageAligned<T>,
    ::sys::error::Error,
>);

} // verus!
