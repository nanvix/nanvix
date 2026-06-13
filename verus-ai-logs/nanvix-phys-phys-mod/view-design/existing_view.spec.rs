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

