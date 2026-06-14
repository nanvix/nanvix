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
// std `LinkedList` support
//==================================================================================================
//
// vstd does not provide a specification for `alloc::collections::LinkedList` or its iterator.
// The boot-orchestration functions iterate the supplied region lists with
// `for region in list.iter() { ... }`, so we mirror the std-iterator support that vstd already
// ships for `core::slice::Iter` (see `vstd/std_specs/slice.rs`). These are trust-boundary
// specifications for std-library functions that vstd does not yet cover.

verus! {

use ::alloc::collections::linked_list::Iter;
use core::alloc::Allocator;
use vstd::pervasive::{ForLoopGhostIterator, ForLoopGhostIteratorNew};

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
#[verifier::reject_recursive_types(A)]
pub struct ExLinkedList<T, A: Allocator>(LinkedList<T, A>);

impl<T, A: Allocator> View for LinkedList<T, A> {
    type V = Seq<T>;

    uninterp spec fn view(&self) -> Seq<T>;
}

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExLinkedListIter<'a, T: 'a>(Iter<'a, T>);

impl<T> View for Iter<'_, T> {
    type V = (int, Seq<T>);

    uninterp spec fn view(&self) -> (int, Seq<T>);
}

pub assume_specification<'a, T>[ Iter::<'a, T>::next ](
    elements: &mut Iter<'a, T>,
) -> (r: Option<&'a T>)
    ensures
        ({
            let (old_index, old_seq) = old(elements)@;
            match r {
                None => {
                    &&& elements@ == old(elements)@
                    &&& old_index >= old_seq.len()
                },
                Some(element) => {
                    let (new_index, new_seq) = elements@;
                    &&& 0 <= old_index < old_seq.len()
                    &&& new_seq == old_seq
                    &&& new_index == old_index + 1
                    &&& element == old_seq[old_index]
                },
            }
        }),
;

pub struct LinkedListIterGhostIterator<'a, T> {
    pub pos: int,
    pub elements: Seq<T>,
    pub phantom: Option<&'a T>,
}

impl<'a, T> ForLoopGhostIteratorNew for Iter<'a, T> {
    type GhostIter = LinkedListIterGhostIterator<'a, T>;

    open spec fn ghost_iter(&self) -> LinkedListIterGhostIterator<'a, T> {
        LinkedListIterGhostIterator { pos: self@.0, elements: self@.1, phantom: None }
    }
}

impl<'a, T: 'a> ForLoopGhostIterator for LinkedListIterGhostIterator<'a, T> {
    type ExecIter = Iter<'a, T>;

    type Item = T;

    type Decrease = int;

    open spec fn exec_invariant(&self, exec_iter: &Iter<'a, T>) -> bool {
        &&& self.pos == exec_iter@.0
        &&& self.elements == exec_iter@.1
    }

    open spec fn ghost_invariant(&self, init: Option<&Self>) -> bool {
        init matches Some(init) ==> {
            &&& init.pos == 0
            &&& init.elements == self.elements
            &&& 0 <= self.pos <= self.elements.len()
        }
    }

    open spec fn ghost_ensures(&self) -> bool {
        self.pos == self.elements.len()
    }

    open spec fn ghost_decrease(&self) -> Option<int> {
        Some(self.elements.len() - self.pos)
    }

    open spec fn ghost_peek_next(&self) -> Option<T> {
        if 0 <= self.pos < self.elements.len() {
            Some(self.elements[self.pos])
        } else {
            None
        }
    }

    open spec fn ghost_advance(
        &self,
        _exec_iter: &Iter<'a, T>,
    ) -> LinkedListIterGhostIterator<'a, T> {
        Self { pos: self.pos + 1, ..*self }
    }
}

impl<'a, T> View for LinkedListIterGhostIterator<'a, T> {
    type V = Seq<T>;

    open spec fn view(&self) -> Seq<T> {
        self.elements.take(self.pos)
    }
}

pub assume_specification<'a, T, A: Allocator>[ LinkedList::<T, A>::iter ](
    s: &'a LinkedList<T, A>,
) -> (iter: Iter<'a, T>)
    ensures
        iter@.0 == 0int,
        iter@.1 == s@,
;

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

} // end verus!

