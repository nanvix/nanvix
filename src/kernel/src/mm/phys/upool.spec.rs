// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// UserFrame / Upool - Specifications
//
// `UserFrame` is an RAII owning handle to one reference of a reference-counted
// physical user frame, abstracted by the physical address of the frame it owns
// (`UserFrame@ : int`, defined by the `View` impl below). `Upool` is a
// stateless allocation facade with no caller-observable state, so it has no
// `View`.
//
// All mutable state (which frames are allocated, and per-frame refcounts) lives
// in the do-not-modify `FrameAllocView` reached through `phys_view()`
// (`mod.spec.rs`). Because `phys_view()` is a zero-argument uninterpreted spec
// function (a single fixed value at the current program point), the frame
// allocator's effect is expressed as monotone single-state facts over
// `phys_view().frames`, exactly like the `frame::*` / `PhysMemoryManager::*`
// shim contracts -- there is no `old(phys_view())` to state a before/after
// transition against.

verus! {

use crate::hal::mem::{
    spec_frame_number,
    spec_max_frame_number,
    spec_page_size,
};
use crate::mm::phys::phys_view;
use crate::mm::phys::PhysAuth;

/// Abstract view of a [`UserFrame`]: the physical address of the frame it owns.
///
/// Lets allocator contracts name a returned user frame's address (e.g. "the
/// returned frame is now allocated") without exposing any storage detail.
impl View for UserFrame {
    type V = int;

    closed spec fn view(&self) -> int {
        self.addr@
    }
}

impl UserFrame {
    /// A user-frame handle is well-formed iff the frame address it names is
    /// well-formed, mirroring `FrameAddress::inv()` over the handle's abstract
    /// view (`self@ == self.addr@`): the frame is page-aligned *and* has a
    /// representable frame number (`spec_frame_number(self@) <=
    /// spec_max_frame_number()`). Keeping the two invariants in lock-step means
    /// the representability conjunct — exactly the precondition the global
    /// allocator shims (`frame::share`, `frame::refcount`) demand of the address
    /// they receive (`frame.inv()`) — always holds for a well-formed handle.
    /// Stated over the public view rather than the private `addr` field so the
    /// `pub open spec fn` is well-formed everywhere. It denotes a real physical
    /// frame, not an arbitrary byte.
    ///
    /// This is the only abstraction-level fact provable about a handle in
    /// isolation. Stronger facts -- "this frame is currently allocated", "its
    /// refcount is `n`" -- are properties of `phys_view().frames` keyed by
    /// `self@`, not invariants of the handle (a sibling `share`/`drop` changes
    /// them without changing this handle), so they are stated in
    /// `requires`/`ensures` over `phys_view()` instead.
    pub open spec fn inv(&self) -> bool {
        &&& self@ % spec_page_size() == 0
        &&& spec_frame_number(self@) <= spec_max_frame_number()
    }
}

} // verus!
