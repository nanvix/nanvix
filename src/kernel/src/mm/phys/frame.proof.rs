verus! {

use super::FrameAllocView;
use super::PhysMemView;
use super::phys_view;
use crate::hal::mem::spec_page_size;
use vstd::map::*;
use vstd::set_lib::*;
use vstd::relations::injective_on;
use vstd::arithmetic::mul::{
    lemma_mul_is_commutative,
    lemma_mul_is_distributive_sub,
    lemma_mul_nonzero,
};

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

/// The size of the free pool equals the number of clear bits in the bitmap.
///
/// `free_frames` is the image of the clear bitmap indices under the injective
/// map `i -> i * PAGE_SIZE`, so its cardinality is `num_bits - usage`.
///
/// `num_bits >= 0` is supplied by the caller: `Bitmap::inv()` references the
/// private backing slice, so the bound is not visible in this module; the caller
/// recovers it from the `usize` result of `number_of_bits()`.
proof fn lemma_free_count(inner: &Inner)
    requires
        inner.inv(),
        inner.bitmap@.num_bits >= 0,
    ensures
        inner@.free_frames.finite(),
        inner@.free_frames.len() + inner.bitmap@.usage() == inner.bitmap@.num_bits,
{
    let bv = inner.bitmap@;
    let n = bv.num_bits;
    let sb = bv.set_bits;
    let ps = spec_page_size();
    let f = |i: int| frame_addr_of(i);
    let full = set_int_range(0, n);
    let clear = full.difference(sb);

    assert(ps > 0);
    bv.lemma_set_bits_finite();
    lemma_int_range(0, n);

    assert(sb.subset_of(full));
    assert(full.intersect(sb) =~= sb);
    lemma_set_difference_len(full, sb);
    assert(clear.subset_of(full));
    assert(clear.finite()) by {
        lemma_len_subset(clear, full);
    }

    assert(injective_on(f, clear)) by {
        assert forall|x1: int, x2: int|
            clear.contains(x1) && clear.contains(x2) && f(x1) == f(x2) implies x1 == x2 by {
            if x1 != x2 {
                lemma_mul_is_commutative(x1, ps);
                lemma_mul_is_commutative(x2, ps);
                lemma_mul_is_distributive_sub(ps, x1, x2);
                lemma_mul_nonzero(ps, x1 - x2);
            }
        }
    }

    assert(inner@.free_frames =~= clear.map(f)) by {
        assert forall|addr: int| inner@.free_frames.contains(addr) implies #[trigger]
            clear.map(f).contains(addr) by {
            let i = choose|i: int|
                0 <= i < n && !sb.contains(i) && addr == frame_addr_of(i);
            assert(clear.contains(i));
            assert(addr == f(i));
        }
        assert forall|addr: int| #[trigger] clear.map(f).contains(addr)
            implies inner@.free_frames.contains(addr) by {
            let x = choose|x: int| clear.contains(x) && addr == f(x);
            assert(0 <= x < n && !sb.contains(x));
        }
    }
    lemma_map_size(clear, inner@.free_frames, f);
}

}
