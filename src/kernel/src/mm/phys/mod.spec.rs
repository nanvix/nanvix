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
/// Models every frame tracked by the allocator as a single map from frame
/// address to reference count:
///
/// - A frame address is a key of `refcounts` iff the allocator covers it.
/// - A covered frame is *free* iff its reference count is `0`.
/// - A covered frame is *allocated* iff its reference count is positive; a
///   count greater than one models shared ownership (e.g. copy-on-write after
///   `share()`).
///
/// Keeping a single map (rather than separate `allocated`/`free` sets that
/// duplicate the same information) makes the specs simpler to write and prove.
#[verifier::ext_equal]
pub struct FrameAllocView
{
    /// Maps each covered frame address to its reference count
    /// (`0` = free, `> 0` = allocated).
    pub refcounts: Map<int, int>,
}

impl FrameAllocView
{
    /// `true` if the allocator covers (tracks) the frame at `addr`.
    pub open spec fn is_covered(&self, addr: int) -> bool
    {
        self.refcounts.contains_key(addr)
    }

    /// `true` if the frame at `addr` is covered and currently allocated.
    pub open spec fn is_allocated(&self, addr: int) -> bool
    {
        self.refcounts.contains_key(addr) && self.refcounts[addr] > 0
    }

    /// `true` if the frame at `addr` is covered and currently free.
    pub open spec fn is_free(&self, addr: int) -> bool
    {
        self.refcounts.contains_key(addr) && self.refcounts[addr] == 0
    }

    /// `true` if no covered frame is free (every covered frame is allocated).
    pub open spec fn no_free_frames(&self) -> bool
    {
        forall|addr: int| #[trigger] self.refcounts.contains_key(addr) ==> self.refcounts[addr] > 0
    }

    /// `true` if every frame address in `frames` is covered and free.
    pub open spec fn all_free(&self, frames: Set<int>) -> bool
    {
        forall|addr: int| #[trigger] frames.contains(addr) ==> self.is_free(addr)
    }

    pub open spec fn wf(&self) -> bool
    {
        // The set of covered frames is finite (it mirrors the finite bitmap),
        // so future proofs can count free/allocated frames via `dom().len()`.
        &&& self.refcounts.dom().finite()
        // Every covered frame address is page-aligned.
        &&& forall|addr: int| #[trigger] self.refcounts.contains_key(addr) ==>
            addr % spec_page_size() == 0
        // Reference counts fit the u8 range (0 = free, up to 255 owners).
        &&& forall|addr: int| #[trigger] self.refcounts.contains_key(addr) ==>
            0 <= self.refcounts[addr] <= 255
    }
}

} // end verus!

