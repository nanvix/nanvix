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
proof fn lemma_aligned_addr_index(a: int)
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
proof fn lemma_frame_addr_injective(i: int, j: int)
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

/// Every frame address is page-aligned: `frame_addr_of(i) % spec_page_size() == 0`.
proof fn lemma_frame_addr_mod_zero(i: int)
    requires
        spec_page_size() > 0,
    ensures
        frame_addr_of(i) % spec_page_size() == 0,
{
    vstd::arithmetic::div_mod::lemma_mod_multiples_basic(i, spec_page_size());
}

/// Dividing a frame address by the page size recovers its index.
proof fn lemma_frame_addr_div(i: int)
    requires
        spec_page_size() > 0,
    ensures
        frame_addr_of(i) / spec_page_size() == i,
{
    vstd::arithmetic::div_mod::lemma_div_multiples_vanish(i, spec_page_size());
}

/// A non-page-aligned address can never be a tracked (allocated) frame.
proof fn lemma_alloc_unaligned(inner: &Inner, addr: int)
    requires
        inner.internal_inv(),
        addr % spec_page_size() != 0,
    ensures
        !inner@.allocated_frames.contains(addr),
{
    if inner@.allocated_frames.contains(addr) {
        let i = choose|i: int|
            #[trigger] inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
        assert(addr == frame_addr_of(i));
        lemma_frame_addr_mod_zero(i);
        assert(false);
    }
}

//==================================================================================================
// View membership lemmas
//==================================================================================================

/// A page-aligned address is in `allocated_frames` iff its bitmap index bit is set.
proof fn lemma_alloc_contains(inner: &Inner, addr: int)
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
proof fn lemma_free_contains(inner: &Inner, addr: int)
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
proof fn lemma_alloc_iff_key(inner: &Inner, addr: int)
    ensures
        inner@.allocated_frames.contains(addr) == inner@.refcounts.contains_key(addr),
{
    assert(inner@.refcounts.dom() =~= inner@.allocated_frames);
}

/// The integer value of the refcount slot for index `i`. A spec-fn wrapper so the value can be
/// named in `proof fn` postconditions without dereferencing the `&'static mut [u8]` field directly
/// (which Verus rejects in an `ensures`); reading the field in a spec-fn body is the same context
/// `view()`/`internal_inv()` already use.
closed spec fn spec_refcount_slot(inner: &Inner, i: int) -> int {
    inner.refcount@[i] as int
}

/// The refcount-map value at an allocated address equals the underlying refcount slot.
proof fn lemma_refcount_value(inner: &Inner, addr: int)
    requires
        inner@.refcounts.contains_key(addr),
    ensures
        inner@.refcounts[addr] == spec_refcount_slot(inner, addr / spec_page_size()),
{
}

//==================================================================================================
// State-transition lemmas (single-frame reserve / release / refcount update)
//==================================================================================================

/// Pure, field-level reconstruction of `Inner::view()` from its component spec values
/// (`set_bits`, `num_bits`, `refcount` sequence). This lets the single-frame transition lemmas
/// reason about the abstract view purely in terms of bitmap/refcount values, without passing
/// `&Inner` snapshots around.
closed spec fn view_of(set_bits: Set<int>, num_bits: int, refcount: Seq<u8>) -> FrameAllocView {
    FrameAllocView {
        allocated_frames: Set::new(|addr: int|
            exists|i: int| #[trigger] set_bits.contains(i) && addr == frame_addr_of(i)
        ),
        free_frames: Set::new(|addr: int|
            exists|i: int| {
                &&& 0 <= i < num_bits
                &&& !#[trigger] set_bits.contains(i)
                &&& addr == frame_addr_of(i)
            }
        ),
        refcounts: Map::new(
            |addr: int|
                exists|i: int| #[trigger] set_bits.contains(i) && addr == frame_addr_of(i),
            |addr: int| refcount[addr / spec_page_size()] as int,
        ),
    }
}

/// `Inner::view()` equals `view_of` applied to its own bitmap/refcount component values.
proof fn lemma_view_of(inner: &Inner)
    ensures
        inner@ == view_of(inner.bitmap@.set_bits, inner.bitmap@.num_bits, inner.refcount@),
{
    let v = view_of(inner.bitmap@.set_bits, inner.bitmap@.num_bits, inner.refcount@);
    assert(inner@.allocated_frames =~= v.allocated_frames);
    assert(inner@.free_frames =~= v.free_frames);
    assert(inner@.refcounts =~= v.refcounts);
    assert(inner@ == v);
}

/// Reserve one frame: setting a previously-clear, in-range bit and writing its refcount to 1
/// moves the frame from `free` to `allocated` with refcount 1.
proof fn lemma_reserve_one_v(sb: Set<int>, nb: int, rc: Seq<u8>, fnx: int, addr: int)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < nb,
        !sb.contains(fnx),
    ensures
        view_of(sb.insert(fnx), nb, rc.update(fnx, 1u8)) == (FrameAllocView {
            allocated_frames: view_of(sb, nb, rc).allocated_frames.insert(addr),
            free_frames: view_of(sb, nb, rc).free_frames.remove(addr),
            refcounts: view_of(sb, nb, rc).refcounts.insert(addr, 1int),
        }),
{
    lemma_aligned_addr_index(addr);
    assert(frame_addr_of(fnx) == addr);
    let pre = view_of(sb, nb, rc);
    let rc2 = rc.update(fnx, 1u8);
    let post = view_of(sb.insert(fnx), nb, rc2);

    assert(post.allocated_frames =~= pre.allocated_frames.insert(addr)) by {
        assert forall|a: int| post.allocated_frames.contains(a)
            implies pre.allocated_frames.insert(addr).contains(a) by {
            let i = choose|i: int| #[trigger] sb.insert(fnx).contains(i) && a == frame_addr_of(i);
            assert(sb.insert(fnx).contains(i) && a == frame_addr_of(i));
            if i != fnx {
                assert(sb.contains(i));
            }
        }
        assert forall|a: int| pre.allocated_frames.insert(addr).contains(a)
            implies post.allocated_frames.contains(a) by {
            if a == addr {
                assert(sb.insert(fnx).contains(fnx) && a == frame_addr_of(fnx));
            } else {
                let i = choose|i: int| #[trigger] sb.contains(i) && a == frame_addr_of(i);
                assert(sb.insert(fnx).contains(i) && a == frame_addr_of(i));
            }
        }
    }

    assert(post.free_frames =~= pre.free_frames.remove(addr)) by {
        assert forall|a: int| post.free_frames.contains(a)
            implies pre.free_frames.remove(addr).contains(a) by {
            let i = choose|i: int|
                0 <= i < nb && !(#[trigger] sb.insert(fnx).contains(i)) && a == frame_addr_of(i);
            assert(0 <= i < nb && !sb.insert(fnx).contains(i) && a == frame_addr_of(i));
            assert(i != fnx);
            lemma_frame_addr_injective_ne(i, fnx);
            assert(!sb.contains(i));
        }
        assert forall|a: int| pre.free_frames.remove(addr).contains(a)
            implies post.free_frames.contains(a) by {
            let i = choose|i: int|
                0 <= i < nb && !(#[trigger] sb.contains(i)) && a == frame_addr_of(i);
            assert(0 <= i < nb && !sb.contains(i) && a == frame_addr_of(i));
            assert(i != fnx);
            assert(!sb.insert(fnx).contains(i));
        }
    }

    assert(post.refcounts =~= pre.refcounts.insert(addr, 1int)) by {
        assert(post.refcounts.dom() =~= pre.refcounts.insert(addr, 1int).dom()) by {
            assert(post.allocated_frames =~= pre.allocated_frames.insert(addr));
        }
        assert forall|a: int| post.refcounts.dom().contains(a)
            implies #[trigger] post.refcounts[a] == pre.refcounts.insert(addr, 1int)[a] by {
            let i = choose|i: int| #[trigger] sb.insert(fnx).contains(i) && a == frame_addr_of(i);
            assert(sb.insert(fnx).contains(i) && a == frame_addr_of(i));
            lemma_frame_addr_div(i);
            assert(a / spec_page_size() == i);
            if a == addr {
                assert(rc2[fnx] == 1u8);
            } else {
                assert(i != fnx);
                assert(rc2[i] == rc[i]);
            }
        }
    }

    assert(post == FrameAllocView {
        allocated_frames: pre.allocated_frames.insert(addr),
        free_frames: pre.free_frames.remove(addr),
        refcounts: pre.refcounts.insert(addr, 1int),
    });
}

/// Release one frame's last reference: clearing a previously-set, in-range bit and writing its
/// refcount to 0 moves the frame from `allocated` back to `free` and drops its refcount entry.
proof fn lemma_release_one_v(sb: Set<int>, nb: int, rc: Seq<u8>, fnx: int, addr: int)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < nb,
        sb.contains(fnx),
    ensures
        view_of(sb.remove(fnx), nb, rc.update(fnx, 0u8)) == (FrameAllocView {
            allocated_frames: view_of(sb, nb, rc).allocated_frames.remove(addr),
            free_frames: view_of(sb, nb, rc).free_frames.insert(addr),
            refcounts: view_of(sb, nb, rc).refcounts.remove(addr),
        }),
{
    lemma_aligned_addr_index(addr);
    assert(frame_addr_of(fnx) == addr);
    let pre = view_of(sb, nb, rc);
    let rc2 = rc.update(fnx, 0u8);
    let post = view_of(sb.remove(fnx), nb, rc2);

    assert(post.allocated_frames =~= pre.allocated_frames.remove(addr)) by {
        assert forall|a: int| post.allocated_frames.contains(a)
            implies pre.allocated_frames.remove(addr).contains(a) by {
            let i = choose|i: int| #[trigger] sb.remove(fnx).contains(i) && a == frame_addr_of(i);
            assert(sb.remove(fnx).contains(i) && a == frame_addr_of(i));
            assert(i != fnx);
            lemma_frame_addr_injective_ne(i, fnx);
            assert(sb.contains(i));
        }
        assert forall|a: int| pre.allocated_frames.remove(addr).contains(a)
            implies post.allocated_frames.contains(a) by {
            let i = choose|i: int| #[trigger] sb.contains(i) && a == frame_addr_of(i);
            assert(sb.contains(i) && a == frame_addr_of(i));
            assert(i != fnx);
            assert(sb.remove(fnx).contains(i));
        }
    }

    assert(post.free_frames =~= pre.free_frames.insert(addr)) by {
        assert forall|a: int| post.free_frames.contains(a)
            implies pre.free_frames.insert(addr).contains(a) by {
            let i = choose|i: int|
                0 <= i < nb && !(#[trigger] sb.remove(fnx).contains(i)) && a == frame_addr_of(i);
            assert(0 <= i < nb && !sb.remove(fnx).contains(i) && a == frame_addr_of(i));
            if i != fnx {
                assert(!sb.contains(i));
            }
        }
        assert forall|a: int| pre.free_frames.insert(addr).contains(a)
            implies post.free_frames.contains(a) by {
            if a == addr {
                assert(0 <= fnx < nb && !sb.remove(fnx).contains(fnx) && a == frame_addr_of(fnx));
            } else {
                let i = choose|i: int|
                    0 <= i < nb && !(#[trigger] sb.contains(i)) && a == frame_addr_of(i);
                assert(0 <= i < nb && !sb.contains(i) && a == frame_addr_of(i));
                assert(!sb.remove(fnx).contains(i));
            }
        }
    }

    assert(post.refcounts =~= pre.refcounts.remove(addr)) by {
        assert(post.refcounts.dom() =~= pre.refcounts.remove(addr).dom()) by {
            assert(post.allocated_frames =~= pre.allocated_frames.remove(addr));
        }
        assert forall|a: int| post.refcounts.dom().contains(a)
            implies #[trigger] post.refcounts[a] == pre.refcounts.remove(addr)[a] by {
            let i = choose|i: int| #[trigger] sb.remove(fnx).contains(i) && a == frame_addr_of(i);
            assert(sb.remove(fnx).contains(i) && a == frame_addr_of(i));
            assert(i != fnx);
            lemma_frame_addr_div(i);
            assert(a / spec_page_size() == i);
            assert(rc2[i] == rc[i]);
        }
    }

    assert(post == FrameAllocView {
        allocated_frames: pre.allocated_frames.remove(addr),
        free_frames: pre.free_frames.insert(addr),
        refcounts: pre.refcounts.remove(addr),
    });
}

/// Update one allocated frame's refcount in place: rewriting a set, in-range bit's refcount slot
/// (the bit stays set) leaves the allocated/free partition unchanged and updates the refcount map.
proof fn lemma_update_refcount_v(sb: Set<int>, nb: int, rc: Seq<u8>, fnx: int, addr: int, nv: u8)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < nb,
        sb.contains(fnx),
    ensures
        view_of(sb, nb, rc.update(fnx, nv)) == (FrameAllocView {
            allocated_frames: view_of(sb, nb, rc).allocated_frames,
            free_frames: view_of(sb, nb, rc).free_frames,
            refcounts: view_of(sb, nb, rc).refcounts.insert(addr, nv as int),
        }),
{
    lemma_aligned_addr_index(addr);
    assert(frame_addr_of(fnx) == addr);
    let pre = view_of(sb, nb, rc);
    let rc2 = rc.update(fnx, nv);
    let post = view_of(sb, nb, rc2);

    assert(post.allocated_frames =~= pre.allocated_frames);
    assert(post.free_frames =~= pre.free_frames);

    assert(post.refcounts =~= pre.refcounts.insert(addr, nv as int)) by {
        assert(pre.allocated_frames.contains(addr)) by {
            assert(sb.contains(fnx) && addr == frame_addr_of(fnx));
        }
        assert(post.refcounts.dom() =~= pre.refcounts.insert(addr, nv as int).dom());
        assert forall|a: int| post.refcounts.dom().contains(a)
            implies #[trigger] post.refcounts[a] == pre.refcounts.insert(addr, nv as int)[a] by {
            let i = choose|i: int| #[trigger] sb.contains(i) && a == frame_addr_of(i);
            assert(sb.contains(i) && a == frame_addr_of(i));
            lemma_frame_addr_div(i);
            assert(a / spec_page_size() == i);
            if a == addr {
                assert(rc2[fnx] == nv);
            } else {
                assert(i != fnx);
                assert(rc2[i] == rc[i]);
            }
        }
    }

    assert(post == FrameAllocView {
        allocated_frames: pre.allocated_frames,
        free_frames: pre.free_frames,
        refcounts: pre.refcounts.insert(addr, nv as int),
    });
}

/// Contrapositive helper of `lemma_frame_addr_injective`: distinct indices have distinct frame
/// addresses (used to discharge `frame_addr_of(i) != addr` side conditions).
proof fn lemma_frame_addr_injective_ne(i: int, j: int)
    requires
        spec_page_size() > 0,
        i != j,
    ensures
        frame_addr_of(i) != frame_addr_of(j),
{
    if frame_addr_of(i) == frame_addr_of(j) {
        lemma_frame_addr_injective(i, j);
    }
}

}
