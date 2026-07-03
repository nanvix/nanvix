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
/// which lets the view below build its domain set without an `exists`
/// quantifier (see `Set::map_by`), so it stays stable under the always-finite
/// Verus set/map model.
pub open spec fn addr_to_frame(addr: int) -> int {
    addr / spec_page_size()
}

/// The set of all covered frame addresses for a `num_bits`-wide bitmap:
/// `{ frame_addr_of(i) | 0 <= i < num_bits }`. Covered frames include both
/// free (refcount 0) and allocated (refcount > 0) frames.
pub open spec fn covered_addrs(num_bits: int) -> Set<int> {
    BitmapView::range_set(0, num_bits).map_by(|i: int| frame_addr_of(i), |addr: int| addr_to_frame(addr))
}

impl View for Inner {
    type V = FrameAllocView;

    closed spec fn view(&self) -> FrameAllocView {
        FrameAllocView {
            refcounts: Map::new(
                covered_addrs(self.bitmap@.num_bits),
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
            !self.bitmap@.set_bits.contains(i) <==> #[trigger] self.refcount@[i] == 0
        )
        // refcount bounded by u8
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits && self.bitmap@.set_bits.contains(i) ==>
            0 < self.refcount@[i] <= 255
        // Every covered bitmap index yields a non-negative, representable frame address
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits ==> {
            &&& frame_addr_of(i) >= 0
            &&& frame_addr_of(i) <= usize::MAX as int
            &&& i <= FrameNumber::spec_max() as int
        }
        // Tail-zero: refcount slots beyond the bitmap range must be zero
        &&& forall|i: int| self.bitmap@.num_bits <= i < self.refcount@.len() ==>
            self.refcount@[i] == 0
    }
}

}
