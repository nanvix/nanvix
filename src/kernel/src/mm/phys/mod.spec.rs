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

//==================================================================================================
// LinkedList standard-library shim
//==================================================================================================
//
// `vstd` does not (yet) provide a specification for `alloc::collections::LinkedList`
// or its iterator, so iterating one in a `for` loop is rejected by the Verus
// front-end. The following block supplies the missing stdlib specification, mirroring
// the `VecDeque` shim shipped in `vstd::std_specs::vecdeque` (both expose an
// `Iter<'a, T>` yielding `&'a T`). This is an external-bottom trust boundary on the
// standard library, identical in spirit to the existing vstd iterator shims.

use ::alloc::collections::linked_list::Iter as LinkedListIter;
use ::alloc::collections::LinkedList;
use ::core::alloc::Allocator;
use vstd::pervasive::ForLoopGhostIterator;
use vstd::pervasive::ForLoopGhostIteratorNew;

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::accept_recursive_types(T)]
pub struct ExLinkedList<T, A: Allocator>(LinkedList<T, A>);

impl<T, A: Allocator> View for LinkedList<T, A> {
    type V = Seq<T>;

    uninterp spec fn view(&self) -> Seq<T>;
}

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::accept_recursive_types(T)]
pub struct ExLinkedListIter<'a, T: 'a>(LinkedListIter<'a, T>);

impl<'a, T: 'a> View for LinkedListIter<'a, T> {
    type V = (int, Seq<T>);

    uninterp spec fn view(self: &LinkedListIter<'a, T>) -> (int, Seq<T>);
}

pub assume_specification<'a, T>[ LinkedListIter::<'a, T>::next ](
    elements: &mut LinkedListIter<'a, T>,
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

impl<'a, T> ForLoopGhostIteratorNew for LinkedListIter<'a, T> {
    type GhostIter = LinkedListIterGhostIterator<'a, T>;

    open spec fn ghost_iter(&self) -> LinkedListIterGhostIterator<'a, T> {
        LinkedListIterGhostIterator { pos: self@.0, elements: self@.1, phantom: None }
    }
}

impl<'a, T: 'a> ForLoopGhostIterator for LinkedListIterGhostIterator<'a, T> {
    type ExecIter = LinkedListIter<'a, T>;

    type Item = T;

    type Decrease = int;

    open spec fn exec_invariant(&self, exec_iter: &LinkedListIter<'a, T>) -> bool {
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
        _exec_iter: &LinkedListIter<'a, T>,
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
    v: &'a LinkedList<T, A>,
) -> (r: LinkedListIter<'a, T>)
    ensures
        r@ == (0int, v@),
;

} // end verus!

