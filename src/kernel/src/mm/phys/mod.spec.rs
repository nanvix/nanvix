// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// PhysMemoryManager - Specifications
//
// This file contains specification functions and view types.

verus! {

use crate::hal::mem::spec_page_size;
use vstd::map::*;

pub uninterp spec fn byte_at_address(ptr: int) -> u8;

/// Abstract view of the frame allocator (`frame::Inner`).
///
/// Captures which physical frames are currently allocated vs. free,
/// together with a per-frame reference count that models shared
/// ownership (e.g. copy-on-write after `share()`).
pub struct FrameAllocView
{
    pub allocated_frames: Set<int>,
    pub free_frames: Set<int>,
    /// Maps each allocated frame address to its reference count.
    /// A frame is present in the map iff it is currently allocated.
    pub refcounts: Map<int, int>,
}

impl FrameAllocView
{
    pub open spec fn wf(&self) -> bool
    {
        // Page-alignment
        &&& forall|addr: int| self.allocated_frames.contains(addr) ==> addr % spec_page_size() == 0
        &&& forall|addr: int| self.free_frames.contains(addr) ==> addr % spec_page_size() == 0
        // Disjoint
        &&& self.allocated_frames.disjoint(self.free_frames)
        // Allocated ↔ refcount consistency: a frame is allocated iff it has a positive refcount
        &&& forall|addr: int| #[trigger] self.allocated_frames.contains(addr) <==>
            self.refcounts.contains_key(addr) && self.refcounts[addr] > 0
        // Free frames have no refcount entry
        &&& forall|addr: int| #[trigger] self.free_frames.contains(addr) ==>
            !self.refcounts.contains_key(addr)
        // Refcount bounded by u8 range
        &&& forall|addr: int| self.refcounts.contains_key(addr) ==>
            0 < self.refcounts[addr] <= 255
    }
}

} // end verus!

//==================================================================================================
// std `LinkedList` type specification
//==================================================================================================
//
// vstd does not provide a specification for `alloc::collections::LinkedList`. Verified functions in
// this module (`init`) take `LinkedList` parameters, so Verus needs to know the type. We declare it
// as an opaque external type. We deliberately do NOT provide a `View`/iterator specification: doing
// so requires implementing vstd's `View`/`ForLoopGhostIterator` traits for the foreign std types
// `LinkedList`/`Iter`, which the Rust orphan rule forbids from a downstream crate (E0117), and the
// pinned `vstd` dependency cannot be extended. Consequently the `for region in list.iter()` loops in
// `book_physical_memory_regions` / `book_mmio_regions` cannot be body-verified here; those two
// functions are marked `external_body` (see `verus-unsupported.md`).

verus! {

use core::alloc::Allocator;

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
#[verifier::reject_recursive_types(A)]
pub struct ExLinkedList<T, A: Allocator>(LinkedList<T, A>);

} // end verus!

//==================================================================================================
// `mm::phys` boot-subsystem View
//==================================================================================================

verus! {

use vstd::set_lib::set_int_range;

/// Abstract state of the global physical-memory subsystem managed by `mm::phys`.
///
/// Models the boot lifecycle of the frame-allocator singleton plus its
/// frame-reservation state. The `frames` component mirrors `frame::instance()@`;
/// the `initialized` flag mirrors the `INSTANCE_INIT` lifecycle gate.
pub struct PhysMemView {
    /// Whether the frame-allocator singleton has been established
    /// (i.e. `init` ran once and returned `Ok`). Models one-shot/monotonic
    /// boot: `false` before `init`, `true` after a successful `init`.
    pub initialized: bool,
    /// Abstract reservation state of the global frame allocator: which physical
    /// frames are reserved (allocated) vs. free, with per-frame refcounts.
    /// Meaningful (well-formed) only once `initialized` is `true`.
    pub frames: FrameAllocView,
}

impl PhysMemView {
    /// Well-formedness: once the allocator is established, the frame-allocator
    /// invariant holds. Before initialization there is no allocator, so no
    /// constraint applies.
    pub open spec fn inv(self) -> bool {
        self.initialized ==> self.frames.wf()
    }

    /// Frames the allocator actually tracks (covered by the bitmap): the union
    /// of reserved and free frames. The abstract form of `is_covered`.
    pub open spec fn covered(self) -> Set<int> {
        self.frames.allocated_frames.union(self.frames.free_frames)
    }

    /// The set of frame addresses occupied by a region `[start, start+size)`.
    pub open spec fn region_frames(start: int, size: int) -> Set<int> {
        let first = start / spec_page_size();
        let last = (start + size) / spec_page_size();
        set_int_range(first, last).map(|i: int| i * spec_page_size())
    }

    /// `init` established the allocator from the caller-supplied bitmap state;
    /// lifecycle flips on.
    pub open spec fn spec_initialize(self, initial: FrameAllocView) -> PhysMemView {
        PhysMemView { initialized: true, frames: initial }
    }

    /// Book one covered frame: move `addr` from free to allocated, refcount = 1.
    /// Mirrors `Inner::book`'s `Ok` post-state.
    pub open spec fn spec_book_frame(self, addr: int) -> PhysMemView {
        PhysMemView {
            frames: FrameAllocView {
                allocated_frames: self.frames.allocated_frames.insert(addr),
                free_frames: self.frames.free_frames.remove(addr),
                refcounts: self.frames.refcounts.insert(addr, 1int),
            },
            ..self
        }
    }

    /// Book a set of frames at once: move them all from free to allocated, each
    /// with refcount = 1. Mirrors `Inner::alloc_range`'s `Ok` post-state and
    /// generalizes it from a contiguous region to an arbitrary frame set.
    pub open spec fn spec_book_frames(self, frames: Set<int>) -> PhysMemView {
        PhysMemView {
            frames: FrameAllocView {
                allocated_frames: self.frames.allocated_frames.union(frames),
                free_frames: self.frames.free_frames.difference(frames),
                refcounts: self.frames.refcounts.union_prefer_right(
                    Map::new(|a: int| frames.contains(a), |a: int| 1int),
                ),
            },
            ..self
        }
    }
}

/// Abstract view of the global physical-memory subsystem at the current program
/// point.
///
/// The subsystem state lives in module-level `static`s (`frame::INSTANCE` /
/// `frame::INSTANCE_INIT`) that a Verus `spec fn` cannot read directly, so this
/// is an *uninterpreted* function: the exec wrappers in `frame.rs` and the
/// `book_*`/`init` functions pin down its value through their `ensures` clauses.
/// `initialized` mirrors `INSTANCE_INIT`; `frames` mirrors `frame::instance()@`.
///
/// This is the handle the caller-facing contract of `init` is stated over: after
/// a successful `init`, `phys_view().initialized` and `phys_view().inv()` hold, so
/// every later `frame::*` / `PhysMemoryManager::*` operation may rely on the
/// frame-allocator invariant.
pub uninterp spec fn phys_view() -> PhysMemView;

} // end verus!

