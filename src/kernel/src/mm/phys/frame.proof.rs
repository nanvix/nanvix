verus! {

use super::FrameAllocView;
use crate::hal::mem::spec_page_size;
use vstd::map::*;

/// Helper: convert a bitmap index to a frame (physical) address.
pub open spec fn frame_addr_of(i: int) -> int {
    i * spec_page_size()
}

impl View for Inner {
    type V = FrameAllocView;

    closed spec fn view(&self) -> FrameAllocView {
        FrameAllocView {
            allocated_frames: Set::new(|addr: int|
                exists|i: int|
                    #[trigger] self.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i)
            ),
            free_frames: Set::new(|addr: int|
                exists|i: int| {
                    &&& 0 <= i < self.bitmap@.num_bits
                    &&& !#[trigger] self.bitmap@.set_bits.contains(i)
                    &&& addr == frame_addr_of(i)
                }
            ),
            refcounts: Map::new(
                |addr: int|
                    exists|i: int|
                        #[trigger] self.bitmap@.set_bits.contains(i)
                        && addr == frame_addr_of(i),
                |addr: int| {
                    let i = addr / spec_page_size();
                    self.refcount@[i] as int
                },
            ),
        }
    }
}

pub proof fn lemma_bound_test(a: int)
    requires
        a % spec_page_size() == 0,
        0 <= a <= usize::MAX as int,
    ensures
        crate::hal::mem::types::address::phys::spec_frame_number(a)
            <= crate::hal::mem::types::address::phys::spec_max_frame_number(),
{
}

impl Inner {
    pub closed spec fn internal_inv(&self) -> bool
    {
        &&& self.bitmap.inv()
        &&& spec_page_size() > 0
        // refcount slice covers all bitmap-managed frames
        &&& self.refcount@.len() >= self.bitmap@.num_bits
        // bitmap bit set iff refcount > 0
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits ==> (
            #[trigger] self.bitmap@.set_bits.contains(i) <==> self.refcount@[i] > 0
        )
        // bitmap bit clear iff refcount == 0
        // NOTE: logically implied by the above when refcount is u8 (>= 0),
        // but kept explicit to help the SMT solver without relying on type bounds.
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits ==> (
            !self.bitmap@.set_bits.contains(i) <==> self.refcount@[i] == 0
        )
        // refcount bounded by u8
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits && self.bitmap@.set_bits.contains(i) ==>
            0 < self.refcount@[i] <= 255
        // Every covered bitmap index yields a non-negative, representable frame address
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits ==> {
            &&& frame_addr_of(i) >= 0
            &&& frame_addr_of(i) <= usize::MAX as int
        }
        // Tail-zero: refcount slots beyond the bitmap range must be zero
        &&& forall|i: int| self.bitmap@.num_bits <= i < self.refcount@.len() ==>
            self.refcount@[i] == 0
    }
}

}
