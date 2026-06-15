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

// Relates the integer frame index `fn_ = pa / PAGE_SIZE` of a page-aligned address `pa` to the
// abstract view of the allocator. Every method that turns an address parameter into a bitmap index
// uses these equivalences to connect the executable index to the spec-level frame sets/maps.
proof fn lemma_frame_facts(inner: &Inner, pa: int, fn_: int)
    requires
        inner.inv(),
        pa >= 0,
        pa % spec_page_size() == 0,
        fn_ == pa / spec_page_size(),
    ensures
        pa == fn_ * spec_page_size(),
        fn_ >= 0,
        inner@.allocated_frames.contains(pa) <==> (
            0 <= fn_ < inner.bitmap@.num_bits && inner.bitmap@.set_bits.contains(fn_)
        ),
        inner@.refcounts.contains_key(pa) <==> (
            0 <= fn_ < inner.bitmap@.num_bits && inner.bitmap@.set_bits.contains(fn_)
        ),
        inner@.refcounts.contains_key(pa) ==> inner@.refcounts[pa] == inner.refcount@[fn_],
        (0 <= fn_ < inner.refcount@.len()) ==> (
            inner.refcount@[fn_] > 0 <==> (
                0 <= fn_ < inner.bitmap@.num_bits && inner.bitmap@.set_bits.contains(fn_)
            )
        ),
        inner@.free_frames.contains(pa) <==> (
            0 <= fn_ < inner.bitmap@.num_bits && !inner.bitmap@.set_bits.contains(fn_)
        ),
{
    let ps: int = spec_page_size();
    let nbits: int = inner.bitmap@.num_bits;
    assert(ps > 0);
    assert(inner.bitmap.inv());
    assert(inner.bitmap@.wf());
    // pa == fn_ * ps (since pa is a multiple of ps).
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(pa, ps);
    assert(pa == ps * fn_);
    assert(pa == fn_ * ps) by (nonlinear_arith)
        requires pa == ps * fn_;
    assert(fn_ >= 0) by (nonlinear_arith)
        requires pa >= 0, ps > 0, pa == fn_ * ps;

    // allocated_frames.contains(pa) <==> (fn_ in range && set).
    assert(inner@.allocated_frames.contains(pa) <==> (
        0 <= fn_ < nbits && inner.bitmap@.set_bits.contains(fn_)
    )) by {
        if inner@.allocated_frames.contains(pa) {
            let i = choose|i: int|
                #[trigger] inner.bitmap@.set_bits.contains(i) && pa == frame_addr_of(i);
            assert(inner.bitmap@.set_bits.contains(i) && pa == frame_addr_of(i));
            assert(0 <= i < nbits);
            assert(pa == i * ps);
            assert(i == fn_) by (nonlinear_arith)
                requires ps > 0, i * ps == fn_ * ps;
        }
        if 0 <= fn_ < nbits && inner.bitmap@.set_bits.contains(fn_) {
            assert(pa == frame_addr_of(fn_));
            assert(inner@.allocated_frames.contains(pa));
        }
    };

    // refcounts share the same key predicate as allocated_frames.
    assert(inner@.refcounts.contains_key(pa) <==> inner@.allocated_frames.contains(pa));
    if inner@.refcounts.contains_key(pa) {
        assert(inner@.refcounts[pa] == inner.refcount@[pa / ps]);
        assert(pa / ps == fn_);
    }

    // refcount slice (for indices within its length) is positive iff the corresponding bit is set.
    if 0 <= fn_ < inner.refcount@.len() {
        if fn_ < nbits {
            assert(inner.bitmap@.set_bits.contains(fn_) <==> inner.refcount@[fn_] > 0);
        } else {
            assert(inner.refcount@[fn_] == 0);
            assert(!inner.bitmap@.set_bits.contains(fn_));
        }
    }

    // free_frames.contains(pa) <==> (fn_ in range && clear).
    assert(inner@.free_frames.contains(pa) <==> (
        0 <= fn_ < nbits && !inner.bitmap@.set_bits.contains(fn_)
    )) by {
        if inner@.free_frames.contains(pa) {
            let i = choose|i: int|
                0 <= i < nbits && !(#[trigger] inner.bitmap@.set_bits.contains(i))
                    && pa == frame_addr_of(i);
            assert(0 <= i < nbits && pa == frame_addr_of(i));
            assert(pa == i * ps);
            assert(i == fn_) by (nonlinear_arith)
                requires ps > 0, i * ps == fn_ * ps;
        }
        if 0 <= fn_ < nbits && !inner.bitmap@.set_bits.contains(fn_) {
            assert(pa == frame_addr_of(fn_));
            assert(inner@.free_frames.contains(pa));
        }
    };
}

}
