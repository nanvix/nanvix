verus! {

use super::FrameAllocView;
use ::bitmap::BitmapView;
use crate::hal::mem::spec_page_size;
use crate::hal::mem::spec_max_frame_number;
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
            !self.bitmap@.set_bits.contains(i) <==> self.refcount@[i] == 0
        )
        // refcount bounded by u8
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits && self.bitmap@.set_bits.contains(i) ==>
            0 < self.refcount@[i] <= 255
        // Every covered bitmap index yields a non-negative, representable frame address
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits ==> {
            &&& frame_addr_of(i) >= 0
            &&& frame_addr_of(i) <= usize::MAX as int
            &&& i <= spec_max_frame_number()
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

/// Frame addresses split additively over the index: `frame_addr_of(base + j)` is
/// `frame_addr_of(base) + j * spec_page_size()`.
proof fn lemma_frame_addr_split(base: int, j: int)
    ensures
        frame_addr_of(base + j) == frame_addr_of(base) + j * spec_page_size(),
{
    vstd::arithmetic::mul::lemma_mul_is_distributive_add_other_way(spec_page_size(), base, j);
}

/// Division by the page size distributes over a sum of two page-aligned values.
proof fn lemma_aligned_div_sum(a: int, b: int)
    requires
        spec_page_size() > 0,
        a % spec_page_size() == 0,
        b % spec_page_size() == 0,
    ensures
        (a + b) / spec_page_size() == a / spec_page_size() + b / spec_page_size(),
{
    let ps = spec_page_size();
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(a, ps);
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(b, ps);
    // a == (a/ps)*ps and b == (b/ps)*ps, so a + b == (a/ps + b/ps)*ps.
    assert(a == ps * (a / ps));
    assert(b == ps * (b / ps));
    assert(a + b == ps * (a / ps + b / ps)) by {
        vstd::arithmetic::mul::lemma_mul_is_distributive_add(ps, a / ps, b / ps);
    }
    vstd::arithmetic::div_mod::lemma_div_multiples_vanish(a / ps + b / ps, ps);
}

/// A positive, page-aligned size spans at least one frame.
proof fn lemma_size_div_pos(size: int)
    requires
        spec_page_size() > 0,
        size > 0,
        size % spec_page_size() == 0,
    ensures
        size / spec_page_size() >= 1,
{
    let ps = spec_page_size();
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(size, ps);
    vstd::arithmetic::div_mod::lemma_div_pos_is_pos(size, ps);
    assert(size == ps * (size / ps));
    if size / ps == 0 {
        assert(size == ps * 0);
    }
}

/// A non-page-aligned address can never be a tracked (allocated) frame.
//==================================================================================================
// Refcount-slot accessors
//==================================================================================================

/// The integer value of the refcount slot for index `i`. A spec-fn wrapper so the value can be
/// named in `proof fn` postconditions without dereferencing the `&'static mut [u8]` field directly
/// (which Verus rejects in an `ensures`); reading the field in a spec-fn body is the same context
/// `view()`/`internal_inv()` already use.
closed spec fn spec_refcount_slot(inner: &Inner, i: int) -> int {
    inner.refcount@[i] as int
}

/// The full refcount sequence of `inner`. A spec-fn wrapper so the `&'static mut [u8]` field can
/// be named in `proof fn` postconditions (Verus rejects dereferencing a `&mut` field directly in
/// an `ensures`); reading the field in a spec-fn body is the same context `view()` already uses.
closed spec fn spec_refcount_seq(inner: &Inner) -> Seq<u8> {
    inner.refcount@
}

//==================================================================================================
// Covered-frame membership lemmas (collapsed single-refcount-map view)
//==================================================================================================

/// A non-page-aligned address is never covered by the allocator.
proof fn lemma_uncovered_unaligned(inner: &Inner, addr: int)
    requires
        inner.internal_inv(),
        addr % spec_page_size() != 0,
    ensures
        !inner@.is_covered(addr),
{
    broadcast use vstd::set_lib::group_set_lib_default, vstd::map::group_map_lemmas;
    if inner@.is_covered(addr) {
        let i = addr_to_frame(addr);
        lemma_frame_addr_mod_zero(i);
        assert(addr == frame_addr_of(i));
        assert(false);
    }
}

/// A page-aligned address is covered iff its bitmap index is in range.
proof fn lemma_covered_iff(inner: &Inner, addr: int)
    requires
        inner.internal_inv(),
        addr % spec_page_size() == 0,
    ensures
        inner@.is_covered(addr) <==> 0 <= addr / spec_page_size() < inner.bitmap@.num_bits,
{
    broadcast use vstd::set_lib::group_set_lib_default, vstd::map::group_map_lemmas;
    let i = addr / spec_page_size();
    lemma_aligned_addr_index(addr);
    assert(frame_addr_of(i) == addr);
}

/// The refcount-map value at a covered address equals the underlying refcount slot.
proof fn lemma_refcount_value(inner: &Inner, addr: int)
    requires
        inner@.refcounts.contains_key(addr),
    ensures
        inner@.refcounts[addr] == spec_refcount_slot(inner, addr / spec_page_size()),
{
    broadcast use vstd::set_lib::group_set_lib_default, vstd::map::group_map_lemmas;
}

/// A page-aligned address is allocated iff its bitmap index is in range and its bit is set.
proof fn lemma_alloc_contains(inner: &Inner, addr: int)
    requires
        inner.internal_inv(),
        addr % spec_page_size() == 0,
    ensures
        inner@.is_allocated(addr) <==> {
            let i = addr / spec_page_size();
            &&& 0 <= i < inner.bitmap@.num_bits
            &&& inner.bitmap@.set_bits.contains(i)
        },
{
    broadcast use vstd::set_lib::group_set_lib_default, vstd::map::group_map_lemmas;
    let i = addr / spec_page_size();
    lemma_covered_iff(inner, addr);
    if inner@.is_covered(addr) {
        lemma_refcount_value(inner, addr);
    } else {
        if 0 <= i < inner.bitmap@.num_bits && inner.bitmap@.set_bits.contains(i) {
            assert(false);
        }
    }
}

/// A page-aligned address is free iff its bitmap index is in range and its bit is unset.
proof fn lemma_free_contains(inner: &Inner, addr: int)
    requires
        inner.internal_inv(),
        addr % spec_page_size() == 0,
    ensures
        inner@.is_free(addr) <==> {
            let i = addr / spec_page_size();
            &&& 0 <= i < inner.bitmap@.num_bits
            &&& !inner.bitmap@.set_bits.contains(i)
        },
{
    broadcast use vstd::set_lib::group_set_lib_default, vstd::map::group_map_lemmas;
    let i = addr / spec_page_size();
    lemma_covered_iff(inner, addr);
    if inner@.is_covered(addr) {
        lemma_refcount_value(inner, addr);
    }
}

//==================================================================================================
// Pure view reconstruction and state-transition lemmas
//==================================================================================================

/// Pure, field-level reconstruction of `Inner::view()` from its bitmap width and refcount
/// sequence. In the collapsed model the abstract view depends only on `num_bits` (which fixes the
/// covered domain) and `refcount` (the per-frame counts); `set_bits` is retained in the signature
/// for symmetry with the bitmap mutations but does not affect the result.
closed spec fn view_of(set_bits: Set<int>, num_bits: int, refcount: Seq<u8>) -> FrameAllocView {
    FrameAllocView {
        refcounts: Map::new(
            covered_addrs(num_bits),
            |addr: int| refcount[addr_to_frame(addr)] as int,
        ),
    }
}

/// `Inner::view()` equals `view_of` applied to its own bitmap/refcount component values.
proof fn lemma_view_of(inner: &Inner)
    ensures
        inner@ == view_of(inner.bitmap@.set_bits, inner.bitmap@.num_bits, spec_refcount_seq(inner)),
{
    broadcast use vstd::map::group_map_lemmas;
    let v = view_of(inner.bitmap@.set_bits, inner.bitmap@.num_bits, spec_refcount_seq(inner));
    assert(inner@.refcounts =~= v.refcounts);
    assert(inner@ =~= v);
}

/// The set of frame addresses of a contiguous index range `[start, start + count)`.
closed spec fn spec_range_frames(start: int, count: int) -> Set<int> {
    BitmapView::range_set(start, start + count).map_by(|i: int| frame_addr_of(i), |addr: int| addr_to_frame(addr))
}

/// Core single-slot transition: writing refcount slot `fnx` (page index of `addr`) to value `v`
/// updates exactly the abstract refcount at `addr`, leaving the covered domain and every other
/// count unchanged. The `set_bits` arguments are irrelevant to the collapsed view and may differ.
proof fn lemma_view_of_set_slot(
    sb2: Set<int>,
    sb: Set<int>,
    nb: int,
    rc: Seq<u8>,
    fnx: int,
    addr: int,
    v: u8,
)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < nb,
        nb <= rc.len(),
    ensures
        view_of(sb2, nb, rc.update(fnx, v)) == (FrameAllocView {
            refcounts: view_of(sb, nb, rc).refcounts.insert(addr, v as int),
        }),
{
    broadcast use
        vstd::set_lib::group_set_lib_default,
        vstd::map::group_map_lemmas,
        vstd::map_lib::group_map_properties;
    lemma_aligned_addr_index(addr);
    lemma_frame_addr_div(fnx);
    assert(frame_addr_of(fnx) == addr);
    let lhs = view_of(sb2, nb, rc.update(fnx, v)).refcounts;
    let rhs = view_of(sb, nb, rc).refcounts.insert(addr, v as int);
    assert(covered_addrs(nb).contains(addr));
    assert(lhs.dom() =~= rhs.dom());
    assert forall|a: int| lhs.dom().contains(a) implies #[trigger] lhs[a] == rhs[a] by {
        let j = addr_to_frame(a);
        assert(covered_addrs(nb).contains(a));
        assert(a == frame_addr_of(j));
        lemma_frame_addr_div(j);
        if a == addr {
            assert(j == fnx);
        } else {
            assert(j != fnx) by {
                lemma_frame_addr_injective_ne(j, fnx);
            }
        }
    }
    assert(lhs =~= rhs);
    assert(view_of(sb2, nb, rc.update(fnx, v)) =~= (FrameAllocView { refcounts: rhs }));
}

/// Reserve one frame: writing its refcount slot to 1 sets the abstract refcount at `addr` to 1.
proof fn lemma_reserve_one_v(sb: Set<int>, nb: int, rc: Seq<u8>, fnx: int, addr: int)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < nb,
        nb <= rc.len(),
    ensures
        view_of(sb.insert(fnx), nb, rc.update(fnx, 1u8)) == (FrameAllocView {
            refcounts: view_of(sb, nb, rc).refcounts.insert(addr, 1int),
        }),
{
    lemma_view_of_set_slot(sb.insert(fnx), sb, nb, rc, fnx, addr, 1u8);
}

/// Release one frame's last reference: writing its refcount slot to 0 sets the abstract refcount
/// at `addr` to 0 (the frame stays covered but becomes free).
proof fn lemma_release_one_v(sb: Set<int>, nb: int, rc: Seq<u8>, fnx: int, addr: int)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < nb,
        nb <= rc.len(),
    ensures
        view_of(sb.remove(fnx), nb, rc.update(fnx, 0u8)) == (FrameAllocView {
            refcounts: view_of(sb, nb, rc).refcounts.insert(addr, 0int),
        }),
{
    lemma_view_of_set_slot(sb.remove(fnx), sb, nb, rc, fnx, addr, 0u8);
}

/// Update one frame's refcount in place: rewriting its refcount slot to `nv` sets the abstract
/// refcount at `addr` to `nv`, leaving the covered domain unchanged.
proof fn lemma_update_refcount_v(sb: Set<int>, nb: int, rc: Seq<u8>, fnx: int, addr: int, nv: u8)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < nb,
        nb <= rc.len(),
    ensures
        view_of(sb, nb, rc.update(fnx, nv)) == (FrameAllocView {
            refcounts: view_of(sb, nb, rc).refcounts.insert(addr, nv as int),
        }),
{
    lemma_view_of_set_slot(sb, sb, nb, rc, fnx, addr, nv);
}

/// Reserve a contiguous range of frames: writing refcount slots `[start, start + count)` to 1
/// merges the range into the abstract refcount map with count 1, leaving the covered domain fixed.
proof fn lemma_reserve_range_v(
    sb: Set<int>,
    nb: int,
    rc: Seq<u8>,
    rc2: Seq<u8>,
    start: int,
    count: int,
)
    requires
        spec_page_size() > 0,
        0 <= start,
        count > 0,
        start + count <= nb,
        nb <= rc.len(),
        rc2.len() == rc.len(),
        forall|k: int|
            0 <= k < rc.len() ==> #[trigger] rc2[k] == (if start <= k < start + count {
                1u8
            } else {
                rc[k]
            }),
    ensures
        view_of(sb.union(BitmapView::range_set(start, start + count)), nb, rc2) == (FrameAllocView {
            refcounts: view_of(sb, nb, rc).refcounts.union_prefer_right(
                Map::new(spec_range_frames(start, count), |addr: int| 1int),
            ),
        }),
{
    broadcast use
        vstd::set_lib::group_set_lib_default,
        vstd::map::group_map_lemmas,
        vstd::map_lib::group_map_properties;
    let frames = spec_range_frames(start, count);
    let range_map = Map::new(frames, |addr: int| 1int);
    let lhs = view_of(sb.union(BitmapView::range_set(start, start + count)), nb, rc2).refcounts;
    let rhs = view_of(sb, nb, rc).refcounts.union_prefer_right(range_map);
    assert(lhs.dom() =~= rhs.dom()) by {
        assert forall|a: int| frames.contains(a) implies covered_addrs(nb).contains(a) by {
            let k = addr_to_frame(a);
            assert(a == frame_addr_of(k));
            assert(start <= k < start + count);
        }
    }
    assert forall|a: int| lhs.dom().contains(a) implies #[trigger] lhs[a] == rhs[a] by {
        let j = addr_to_frame(a);
        assert(covered_addrs(nb).contains(a));
        assert(a == frame_addr_of(j));
        lemma_frame_addr_div(j);
        if frames.contains(a) {
            let k = addr_to_frame(a);
            assert(a == frame_addr_of(k));
            assert(start <= k < start + count);
            assert(k == j);
            assert(rc2[j] == 1u8);
        } else {
            assert(!(start <= j < start + count)) by {
                if start <= j < start + count {
                    assert(frames.contains(frame_addr_of(j)));
                }
            }
            assert(rc2[j] == rc[j]);
        }
    }
    assert(lhs =~= rhs);
    assert(view_of(sb.union(BitmapView::range_set(start, start + count)), nb, rc2)
        =~= (FrameAllocView { refcounts: rhs }));
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


//==================================================================================================
// Bridging lemmas: derive abstract view predicates from concrete bitmap/refcount values
//==================================================================================================

/// Expose the `internal_inv` link between bitmap bits and refcount slots so callers (which see
/// `internal_inv()` only as an opaque predicate) can use it on captured component values.
proof fn lemma_internal_inv_facts(inner: &Inner)
    requires
        inner.internal_inv(),
    ensures
        spec_page_size() > 0,
        spec_refcount_seq(inner).len() >= inner.bitmap@.num_bits,
        forall|i: int| 0 <= i < inner.bitmap@.num_bits ==>
            (#[trigger] inner.bitmap@.set_bits.contains(i) <==> spec_refcount_slot(inner, i) > 0),
        forall|i: int| 0 <= i < inner.bitmap@.num_bits ==>
            (!(#[trigger] inner.bitmap@.set_bits.contains(i)) <==> spec_refcount_slot(inner, i) == 0),
{
}

/// `view_of(.., rc)` reports `addr` covered with refcount slot value at index `fnx`.
proof fn lemma_view_of_refcount_val(sb: Set<int>, nb: int, rc: Seq<u8>, fnx: int, addr: int)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < nb,
        nb <= rc.len(),
    ensures
        view_of(sb, nb, rc).is_covered(addr),
        view_of(sb, nb, rc).refcounts[addr] == rc[fnx] as int,
{
    broadcast use vstd::set_lib::group_set_lib_default, vstd::map::group_map_lemmas;
    lemma_aligned_addr_index(addr);
    lemma_frame_addr_div(fnx);
    assert(frame_addr_of(fnx) == addr);
    assert(covered_addrs(nb).contains(addr));
}

/// `view_of(.., rc)` reports `addr` free when its refcount slot is 0 and the index is in range.
proof fn lemma_view_of_is_free(sb: Set<int>, nb: int, rc: Seq<u8>, fnx: int, addr: int)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < nb,
        nb <= rc.len(),
        rc[fnx] == 0,
    ensures
        view_of(sb, nb, rc).is_free(addr),
{
    lemma_view_of_refcount_val(sb, nb, rc, fnx, addr);
}

/// `view_of(.., rc)` reports `addr` allocated when its refcount slot is positive and in range.
proof fn lemma_view_of_is_allocated(sb: Set<int>, nb: int, rc: Seq<u8>, fnx: int, addr: int)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < nb,
        nb <= rc.len(),
        rc[fnx] > 0,
    ensures
        view_of(sb, nb, rc).is_allocated(addr),
{
    lemma_view_of_refcount_val(sb, nb, rc, fnx, addr);
}

/// If every in-range refcount slot is positive, no covered frame is free.
proof fn lemma_view_of_no_free(sb: Set<int>, nb: int, rc: Seq<u8>)
    requires
        spec_page_size() > 0,
        nb <= rc.len(),
        forall|i: int| 0 <= i < nb ==> #[trigger] rc[i] > 0,
    ensures
        view_of(sb, nb, rc).no_free_frames(),
{
    broadcast use vstd::set_lib::group_set_lib_default, vstd::map::group_map_lemmas;
    let v = view_of(sb, nb, rc);
    assert forall|addr: int| #[trigger] v.refcounts.contains_key(addr) implies v.refcounts[addr] > 0 by {
        let i = addr_to_frame(addr);
        assert(covered_addrs(nb).contains(addr));
        assert(addr == frame_addr_of(i));
        lemma_frame_addr_div(i);
    }
}

/// `view_of(.., rc)` reports every address of `frames` free when each maps to an in-range,
/// zero-refcount, page-aligned index.
proof fn lemma_view_of_all_free(sb: Set<int>, nb: int, rc: Seq<u8>, frames: Set<int>)
    requires
        spec_page_size() > 0,
        nb <= rc.len(),
        forall|addr: int| #[trigger] frames.contains(addr) ==> {
            &&& addr % spec_page_size() == 0
            &&& 0 <= addr / spec_page_size() < nb
            &&& rc[addr / spec_page_size()] == 0
        },
    ensures
        view_of(sb, nb, rc).all_free(frames),
{
    let v = view_of(sb, nb, rc);
    assert forall|addr: int| #[trigger] frames.contains(addr) implies v.is_free(addr) by {
        lemma_view_of_is_free(sb, nb, rc, addr / spec_page_size(), addr);
    }
}

}
