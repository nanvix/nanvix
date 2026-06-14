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

//==================================================================================================
// Arithmetic helper lemmas relating frame addresses and bitmap indices
//==================================================================================================

/// For a page-aligned address `a`, dividing by the page size then multiplying recovers `a`.
pub proof fn lemma_aligned_addr_index(a: int)
    requires
        a % spec_page_size() == 0,
        spec_page_size() > 0,
    ensures
        frame_addr_of(a / spec_page_size()) == a,
{
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(a, spec_page_size());
    vstd::arithmetic::mul::lemma_mul_is_commutative(a / spec_page_size(), spec_page_size());
}

/// `frame_addr_of` is injective: distinct indices map to distinct frame addresses.
pub proof fn lemma_frame_addr_injective(i: int, j: int)
    requires
        frame_addr_of(i) == frame_addr_of(j),
        spec_page_size() > 0,
    ensures
        i == j,
{
    vstd::arithmetic::mul::lemma_mul_is_commutative(i, spec_page_size());
    vstd::arithmetic::mul::lemma_mul_is_commutative(j, spec_page_size());
    vstd::arithmetic::mul::lemma_mul_equality_converse(spec_page_size(), i, j);
}

//==================================================================================================
// View membership lemmas
//==================================================================================================

/// A page-aligned address is in `allocated_frames` iff its bitmap index bit is set.
pub proof fn lemma_alloc_contains(inner: &Inner, addr: int)
    requires
        inner.internal_inv(),
        addr % spec_page_size() == 0,
    ensures
        inner@.allocated_frames.contains(addr)
            <==> inner.bitmap@.set_bits.contains(addr / spec_page_size()),
{
    let i = addr / spec_page_size();
    lemma_aligned_addr_index(addr);
    if inner@.allocated_frames.contains(addr) {
        let j = choose|j: int|
            #[trigger] inner.bitmap@.set_bits.contains(j) && addr == frame_addr_of(j);
        assert(inner.bitmap@.set_bits.contains(j) && addr == frame_addr_of(j));
        lemma_frame_addr_injective(j, i);
        assert(inner.bitmap@.set_bits.contains(i));
    }
    if inner.bitmap@.set_bits.contains(i) {
        assert(addr == frame_addr_of(i));
        assert(inner@.allocated_frames.contains(addr));
    }
}

/// A page-aligned address is in `free_frames` iff its bitmap index is in range and unset.
pub proof fn lemma_free_contains(inner: &Inner, addr: int)
    requires
        inner.internal_inv(),
        addr % spec_page_size() == 0,
    ensures
        inner@.free_frames.contains(addr) <==> {
            let i = addr / spec_page_size();
            &&& 0 <= i < inner.bitmap@.num_bits
            &&& !inner.bitmap@.set_bits.contains(i)
        },
{
    let i = addr / spec_page_size();
    lemma_aligned_addr_index(addr);
    if inner@.free_frames.contains(addr) {
        let j = choose|j: int|
            0 <= j < inner.bitmap@.num_bits && !(#[trigger] inner.bitmap@.set_bits.contains(j))
                && addr == frame_addr_of(j);
        assert(0 <= j < inner.bitmap@.num_bits && !inner.bitmap@.set_bits.contains(j)
            && addr == frame_addr_of(j));
        lemma_frame_addr_injective(j, i);
    }
    if 0 <= i < inner.bitmap@.num_bits && !inner.bitmap@.set_bits.contains(i) {
        assert(addr == frame_addr_of(i));
        assert(inner@.free_frames.contains(addr));
    }
}

/// The refcount map's domain coincides with the allocated-frame set.
pub proof fn lemma_alloc_iff_key(inner: &Inner, addr: int)
    ensures
        inner@.allocated_frames.contains(addr) == inner@.refcounts.contains_key(addr),
{
    assert(inner@.refcounts.dom() =~= inner@.allocated_frames);
}

/// The refcount-map value at an allocated address equals the underlying refcount slot.
pub proof fn lemma_refcount_value(inner: &Inner, addr: int)
    requires
        inner@.refcounts.contains_key(addr),
    ensures
        inner@.refcounts[addr] == inner.refcount@[addr / spec_page_size()] as int,
{
}

}
