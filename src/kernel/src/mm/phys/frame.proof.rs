verus! {

use super::FrameAllocView;
use ::bitmap::BitmapView;
use crate::hal::mem::spec_page_size;
use vstd::map::*;

/// Helper: convert a bitmap index to a frame (physical) address.
pub open spec fn frame_addr_of(i: int) -> int {
    i * spec_page_size()
}

/// Helper: convert a frame (physical) address back to a bitmap index.
///
/// This is the left inverse of [`frame_addr_of`] on page-aligned addresses,
/// which lets the view below build its set/map without an `exists` quantifier
/// (see `Set::map_by`).
pub open spec fn addr_to_frame(addr: int) -> int {
    addr / spec_page_size()
}

impl View for Inner {
    type V = FrameAllocView;

    closed spec fn view(&self) -> FrameAllocView {
        // Set of all covered frame addresses: { frame_addr_of(i) | 0 <= i < num_bits }.
        // Built with `map_by` (forward `frame_addr_of`, reverse `addr_to_frame`) so the
        // membership test is exists-free and stays stable under the planned
        // finite-sets-and-maps Verus update.
        let covered_frames: Set<int> = BitmapView::range_set(0, self.bitmap@.num_bits)
            .map_by(|i: int| frame_addr_of(i), |addr: int| addr_to_frame(addr));
        FrameAllocView {
            refcounts: covered_frames.mk_map(
                |addr: int| self.refcount@[addr_to_frame(addr)] as int,
            ),
        }
    }
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
