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

// Relates the integer frame index `pa / PAGE_SIZE` of a page-aligned address `pa` to coverage by
// the bitmap: `pa` is tracked (allocated or free) iff its frame index is within `num_bits`.
proof fn lemma_is_covered(inner: &Inner, pa: int, frame_number: int)
    requires
        inner.inv(),
        pa >= 0,
        pa % spec_page_size() == 0,
        frame_number == pa / spec_page_size(),
    ensures
        (frame_number < inner.bitmap@.num_bits) <==> (
            inner@.allocated_frames.contains(pa)
            || inner@.free_frames.contains(pa)
        ),
{
    let ps: int = spec_page_size();
    let nbits: int = inner.bitmap@.num_bits;
    assert(ps > 0);
    assert(inner.bitmap.inv());
    assert(inner.bitmap@.wf());
    // pa == frame_number * ps (since pa is a multiple of ps).
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(pa, ps);
    assert(pa == ps * frame_number);
    assert(pa == frame_number * ps) by (nonlinear_arith)
        requires pa == ps * frame_number;
    assert(frame_number >= 0) by (nonlinear_arith)
        requires pa >= 0, ps > 0, pa == frame_number * ps;
    // Forward: if the frame index is within range, `pa` is either allocated or free.
    if frame_number < nbits {
        assert(pa == frame_addr_of(frame_number));
        if inner.bitmap@.set_bits.contains(frame_number) {
            assert(inner@.allocated_frames.contains(pa));
        } else {
            assert(inner@.free_frames.contains(pa));
        }
    }
    // Backward: tracked addresses have a frame index within range.
    if inner@.allocated_frames.contains(pa) {
        let i = choose|i: int|
            #[trigger] inner.bitmap@.set_bits.contains(i) && pa == frame_addr_of(i);
        assert(inner.bitmap@.set_bits.contains(i) && pa == frame_addr_of(i));
        assert(0 <= i < nbits);
        assert(pa == i * ps);
        assert(i == frame_number) by (nonlinear_arith)
            requires ps > 0, i * ps == frame_number * ps;
    }
    if inner@.free_frames.contains(pa) {
        let i = choose|i: int|
            0 <= i < nbits && !(#[trigger] inner.bitmap@.set_bits.contains(i))
                && pa == frame_addr_of(i);
        assert(0 <= i < nbits && pa == frame_addr_of(i));
        assert(pa == i * ps);
        assert(i == frame_number) by (nonlinear_arith)
            requires ps > 0, i * ps == frame_number * ps;
    }
}

}
