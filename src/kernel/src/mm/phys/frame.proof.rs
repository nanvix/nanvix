verus! {

use super::FrameAllocView;
use super::region_frame_addrs;
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

// A non-page-aligned address can never be a tracked (allocated) frame.
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
        assert forall|a: int| #[trigger] frames.contains(a) implies covered_addrs(nb).contains(a) by {
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

//==================================================================================================
// Operation-level bridge lemmas
//
// These capture the recurring reasoning of the frame-allocator method bodies so the bodies stay
// readable. Each lemma is phrased over the captured component snapshots (`pre_sb`/`pre_nb`/`pre_rc`)
// and the post-mutation `&Inner`, mirroring the facts the original inline proof blocks produced.
//==================================================================================================

/// Snapshot the pre-state view and unpack the bitmap/refcount invariant link into facts over the
/// captured component values, so later proof obligations can read them off `pre_sb`/`pre_rc`.
proof fn lemma_capture_inv_facts(
    inner: &Inner,
    g_old: FrameAllocView,
    pre_sb: Set<int>,
    pre_nb: int,
    pre_rc: Seq<u8>,
)
    requires
        inner.internal_inv(),
        g_old == inner@,
        pre_sb == inner.bitmap@.set_bits,
        pre_nb == inner.bitmap@.num_bits,
        pre_rc == spec_refcount_seq(inner),
    ensures
        spec_page_size() > 0,
        g_old == view_of(pre_sb, pre_nb, pre_rc),
        pre_rc.len() >= pre_nb,
        forall|i: int| 0 <= i < pre_nb ==> (#[trigger] pre_sb.contains(i) <==> pre_rc[i] > 0),
        forall|i: int| 0 <= i < pre_nb ==> (!(#[trigger] pre_sb.contains(i)) <==> pre_rc[i] == 0),
        forall|i: int| pre_nb <= i < pre_rc.len() ==> #[trigger] pre_rc[i] == 0,
        forall|i: int| 0 <= i < pre_nb ==> {
            &&& #[trigger] frame_addr_of(i) >= 0
            &&& frame_addr_of(i) <= usize::MAX as int
            &&& i <= FrameNumber::spec_max() as int
        },
{
    lemma_view_of(inner);
    lemma_internal_inv_facts(inner);
    assert forall|i: int| pre_nb <= i < pre_rc.len() implies #[trigger] pre_rc[i] == 0 by {}
    assert forall|i: int| 0 <= i < pre_nb implies {
        &&& #[trigger] frame_addr_of(i) >= 0
        &&& frame_addr_of(i) <= usize::MAX as int
        &&& i <= FrameNumber::spec_max() as int
    } by {}
}

/// A frame whose refcount slot is non-zero and in range is allocated: its bit is set, its index is
/// below `num_bits`, and the abstract refcount equals the concrete slot value.
proof fn lemma_frame_allocated(inner: &Inner, addr: int, fnx: int)
    requires
        inner.internal_inv(),
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < spec_refcount_seq(inner).len(),
        spec_refcount_seq(inner)[fnx] != 0,
    ensures
        fnx < inner.bitmap@.num_bits,
        inner.bitmap@.set_bits.contains(fnx),
        inner@.is_allocated(addr),
        inner@.is_covered(addr),
        inner@.refcounts[addr] == spec_refcount_slot(inner, fnx),
{
    assert(fnx < inner.bitmap@.num_bits);
    assert(inner.bitmap@.set_bits.contains(fnx));
    lemma_alloc_contains(inner, addr);
    assert(inner@.is_allocated(addr));
    assert(inner@.is_covered(addr));
    lemma_refcount_value(inner, addr);
}

/// Post-state of an in-place refcount write (no bitmap change): the abstract refcount at `addr`
/// becomes the new slot value `nv`, every other count and the covered domain unchanged.
proof fn lemma_post_update_slot(
    inner: &Inner,
    addr: int,
    fnx: int,
    nv: u8,
    g_old: FrameAllocView,
    pre_sb: Set<int>,
    pre_nb: int,
    pre_rc: Seq<u8>,
)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < pre_nb,
        pre_nb <= pre_rc.len(),
        inner.bitmap@.set_bits == pre_sb,
        inner.bitmap@.num_bits == pre_nb,
        spec_refcount_seq(inner) == pre_rc.update(fnx, nv),
        g_old == view_of(pre_sb, pre_nb, pre_rc),
    ensures
        inner@ == (FrameAllocView { refcounts: g_old.refcounts.insert(addr, nv as int) }),
{
    lemma_view_of(inner);
    lemma_update_refcount_v(pre_sb, pre_nb, pre_rc, fnx, addr, nv);
}

/// Post-state of releasing a frame's last reference (bit cleared, slot written to 0): the abstract
/// refcount at `addr` becomes 0 (the frame stays covered but free).
proof fn lemma_post_release_one(
    inner: &Inner,
    addr: int,
    fnx: int,
    g_old: FrameAllocView,
    pre_sb: Set<int>,
    pre_nb: int,
    pre_rc: Seq<u8>,
)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < pre_nb,
        pre_nb <= pre_rc.len(),
        inner.bitmap@.set_bits == pre_sb.remove(fnx),
        inner.bitmap@.num_bits == pre_nb,
        spec_refcount_seq(inner) == pre_rc.update(fnx, 0u8),
        g_old == view_of(pre_sb, pre_nb, pre_rc),
    ensures
        inner@ == (FrameAllocView { refcounts: g_old.refcounts.insert(addr, 0int) }),
{
    lemma_view_of(inner);
    lemma_release_one_v(pre_sb, pre_nb, pre_rc, fnx, addr);
}

/// Post-state of reserving a single free frame (bit set, slot written to 1): `addr` was free in the
/// pre-state and its abstract refcount becomes 1.
proof fn lemma_post_reserve_one(
    inner: &Inner,
    addr: int,
    fnx: int,
    g_old: FrameAllocView,
    pre_sb: Set<int>,
    pre_nb: int,
    pre_rc: Seq<u8>,
)
    requires
        spec_page_size() > 0,
        addr % spec_page_size() == 0,
        fnx == addr / spec_page_size(),
        0 <= fnx < pre_nb,
        pre_nb <= pre_rc.len(),
        pre_rc[fnx] == 0,
        inner.bitmap@.set_bits == pre_sb.insert(fnx),
        inner.bitmap@.num_bits == pre_nb,
        spec_refcount_seq(inner) == pre_rc.update(fnx, 1u8),
        g_old == view_of(pre_sb, pre_nb, pre_rc),
    ensures
        g_old.is_free(addr),
        inner@ == (FrameAllocView { refcounts: g_old.refcounts.insert(addr, 1int) }),
{
    lemma_view_of_is_free(pre_sb, pre_nb, pre_rc, fnx, addr);
    lemma_view_of(inner);
    lemma_reserve_one_v(pre_sb, pre_nb, pre_rc, fnx, addr);
}

/// `lemma_post_reserve_one` keyed by the frame *index* rather than its address: the reserved frame
/// is `frame_addr_of(idx)`. Lets `alloc`'s success arm reserve a freshly-allocated frame number
/// without restating the address-alignment side conditions.
proof fn lemma_post_reserve_one_by_index(
    inner: &Inner,
    idx: int,
    g_old: FrameAllocView,
    pre_sb: Set<int>,
    pre_nb: int,
    pre_rc: Seq<u8>,
)
    requires
        spec_page_size() > 0,
        0 <= idx < pre_nb,
        pre_nb <= pre_rc.len(),
        pre_rc[idx] == 0,
        inner.bitmap@.set_bits == pre_sb.insert(idx),
        inner.bitmap@.num_bits == pre_nb,
        spec_refcount_seq(inner) == pre_rc.update(idx, 1u8),
        g_old == view_of(pre_sb, pre_nb, pre_rc),
    ensures
        g_old.is_free(frame_addr_of(idx)),
        inner@ == (FrameAllocView { refcounts: g_old.refcounts.insert(frame_addr_of(idx), 1int) }),
{
    lemma_frame_addr_mod_zero(idx);
    lemma_frame_addr_div(idx);
    lemma_post_reserve_one(inner, frame_addr_of(idx), idx, g_old, pre_sb, pre_nb, pre_rc);
}

/// A full bitmap (allocation failed) has no free covered frame: every in-range refcount slot is
/// positive, so `no_free_frames()` holds on the unchanged pre-state view.
proof fn lemma_alloc_full_no_free(
    inner: &Inner,
    g_old: FrameAllocView,
    pre_sb: Set<int>,
    pre_nb: int,
    pre_rc: Seq<u8>,
)
    requires
        inner.internal_inv(),
        inner.bitmap@.is_full(),
        pre_sb == inner.bitmap@.set_bits,
        pre_nb == inner.bitmap@.num_bits,
        pre_rc == spec_refcount_seq(inner),
        g_old == inner@,
    ensures
        g_old == view_of(pre_sb, pre_nb, pre_rc),
        g_old.no_free_frames(),
{
    lemma_view_of(inner);
    assert forall|i: int| 0 <= i < pre_nb implies #[trigger] pre_rc[i] > 0 by {
        assert(inner.bitmap@.set_bits.contains(i));
    }
    lemma_view_of_no_free(pre_sb, pre_nb, pre_rc);
}

/// Re-establish `internal_inv` after booking the contiguous index range `[lo, hi)`: the bitmap has
/// the whole range set and the refcount loop wrote exactly the range's slots to 1.
proof fn lemma_reestablish_inv_range(
    inner: &Inner,
    pre_sb: Set<int>,
    pre_nb: int,
    pre_rc: Seq<u8>,
    lo: int,
    hi: int,
)
    requires
        spec_page_size() > 0,
        0 <= lo <= hi <= pre_nb,
        pre_nb <= pre_rc.len(),
        forall|i: int| 0 <= i < pre_nb ==> (#[trigger] pre_sb.contains(i) <==> pre_rc[i] > 0),
        forall|i: int| 0 <= i < pre_nb ==> (!(#[trigger] pre_sb.contains(i)) <==> pre_rc[i] == 0),
        forall|i: int| pre_nb <= i < pre_rc.len() ==> #[trigger] pre_rc[i] == 0,
        forall|i: int| 0 <= i < pre_nb ==> {
            &&& #[trigger] frame_addr_of(i) >= 0
            &&& frame_addr_of(i) <= usize::MAX as int
            &&& i <= FrameNumber::spec_max() as int
        },
        inner.bitmap.inv(),
        inner.bitmap@.num_bits == pre_nb,
        inner.bitmap@.set_bits == pre_sb.union(BitmapView::range_set(lo, hi)),
        spec_refcount_seq(inner).len() == pre_rc.len(),
        forall|k: int| 0 <= k < pre_rc.len() ==>
            #[trigger] spec_refcount_seq(inner)[k] == (if lo <= k < hi { 1u8 } else { pre_rc[k] }),
    ensures
        inner.internal_inv(),
{
    broadcast use vstd::set_lib::group_set_lib_default;
    assert forall|k: int| 0 <= k < pre_nb implies {
        &&& (inner.bitmap@.set_bits.contains(k) <==> #[trigger] inner.refcount@[k] > 0)
        &&& (!inner.bitmap@.set_bits.contains(k) <==> inner.refcount@[k] == 0)
        &&& (inner.bitmap@.set_bits.contains(k) ==> 0 < inner.refcount@[k] <= 255)
    } by {
        if lo <= k < hi {
            assert(BitmapView::range_set(lo, hi).contains(k));
            assert(inner.refcount@[k] == 1u8);
        } else {
            assert(!BitmapView::range_set(lo, hi).contains(k));
            assert(inner.refcount@[k] == pre_rc[k]);
            assert(pre_sb.contains(k) <==> pre_rc[k] > 0);
        }
    }
    assert forall|k: int| pre_nb <= k < inner.refcount@.len() implies inner.refcount@[k] == 0 by {
        assert(inner.refcount@[k] == pre_rc[k]);
    }
}

/// Post-state of `alloc_contiguous`: the booked run `[start, start + count)` was entirely free and
/// the resulting view merges the run (count 1) into the pre-state refcount map. `frames` is the
/// contract's address set `{ base + i * PS | 0 <= i < count }`.
proof fn lemma_alloc_contiguous_post(
    inner: &Inner,
    base: int,
    start: int,
    count: int,
    g_old: FrameAllocView,
    pre_sb: Set<int>,
    pre_nb: int,
    pre_rc: Seq<u8>,
)
    requires
        spec_page_size() > 0,
        base == frame_addr_of(start),
        0 <= start,
        count > 0,
        start + count <= pre_nb,
        pre_nb <= pre_rc.len(),
        forall|i: int| 0 <= i < pre_nb ==> (!(#[trigger] pre_sb.contains(i)) <==> pre_rc[i] == 0),
        forall|j: int| start <= j < start + count ==> !pre_sb.contains(j),
        inner.bitmap@.set_bits == pre_sb.union(BitmapView::range_set(start, start + count)),
        inner.bitmap@.num_bits == pre_nb,
        spec_refcount_seq(inner).len() == pre_rc.len(),
        forall|k: int| 0 <= k < pre_rc.len() ==>
            #[trigger] spec_refcount_seq(inner)[k] == (if start <= k < start + count { 1u8 } else { pre_rc[k] }),
        g_old == view_of(pre_sb, pre_nb, pre_rc),
    ensures
        ({
            let frames = Set::range(0, count).map_by(
                |i: int| base + i * spec_page_size(),
                |addr: int| (addr - base) / spec_page_size(),
            );
            &&& g_old.all_free(frames)
            &&& inner@ == (FrameAllocView {
                refcounts: g_old.refcounts.union_prefer_right(Map::new(frames, |addr: int| 1int)),
            })
        }),
{
    broadcast use
        vstd::set_lib::group_set_lib_default,
        vstd::map::group_map_lemmas,
        vstd::map_lib::group_map_properties;
    let frames = Set::range(0, count).map_by(
        |i: int| base + i * spec_page_size(),
        |addr: int| (addr - base) / spec_page_size(),
    );
    assert(frames =~= spec_range_frames(start, count)) by {
        assert forall|addr: int| #[trigger] frames.contains(addr) implies
            spec_range_frames(start, count).contains(addr) by {
            let i = (addr - base) / spec_page_size();
            lemma_frame_addr_split(start, i);
            lemma_frame_addr_div(start + i);
        }
        assert forall|addr: int| spec_range_frames(start, count).contains(addr) implies
            #[trigger] frames.contains(addr) by {
            let k = addr / spec_page_size();
            lemma_frame_addr_split(start, k - start);
            lemma_frame_addr_div(k);
        }
    }
    assert forall|addr: int| #[trigger] frames.contains(addr) implies {
        &&& addr % spec_page_size() == 0
        &&& 0 <= addr / spec_page_size() < pre_nb
        &&& pre_rc[addr / spec_page_size()] == 0
    } by {
        let i = (addr - base) / spec_page_size();
        lemma_frame_addr_split(start, i);
        lemma_frame_addr_div(start + i);
        lemma_frame_addr_mod_zero(start + i);
        assert(!pre_sb.contains(start + i));
    }
    lemma_view_of_all_free(pre_sb, pre_nb, pre_rc, frames);
    lemma_view_of(inner);
    lemma_reserve_range_v(pre_sb, pre_nb, pre_rc, inner.refcount@, start, count);
    assert(Map::new(frames, |addr: int| 1int)
        =~= Map::new(spec_range_frames(start, count), |addr: int| 1int));
    assert(inner@.refcounts =~= g_old.refcounts.union_prefer_right(
        Map::new(frames, |addr: int| 1int)));
    assert(inner@ == (FrameAllocView {
        refcounts: g_old.refcounts.union_prefer_right(Map::new(frames, |addr: int| 1int)),
    }));
}

/// If the abstract view has a contiguous run of `count` free frames based at address `base`, then
/// the underlying bitmap has a contiguous run of `count` clear bits at index `base / PAGE_SIZE`.
/// This is the bridge that lets `alloc_contiguous`'s `Err` arm state that no such free run exists.
proof fn lemma_free_run_implies_bitmap_range(inner: &Inner, base: int, count: int)
    requires
        inner.internal_inv(),
        count > 0,
        inner@.contiguous_free_run_at(base, count),
    ensures
        inner.bitmap@.exists_contiguous_free_range(count),
{
    broadcast use
        vstd::set_lib::group_set_lib_default,
        vstd::map::group_map_lemmas;
    lemma_internal_inv_facts(inner);
    let ps = spec_page_size();
    let frames = Set::range(0, count).map_by(
        |i: int| base + i * ps,
        |addr: int| (addr - base) / ps,
    );
    // Each run element `base + i * ps` (0 <= i < count) is a member of `frames`.
    assert forall|i: int| 0 <= i < count implies #[trigger] frames.contains(base + i * ps) by {
        lemma_frame_addr_div(i);
    }
    // The base frame (i = 0) is free, hence covered, hence page-aligned.
    assert(frames.contains(base + 0 * ps));
    assert(inner@.is_free(base));
    if base % ps != 0 {
        lemma_uncovered_unaligned(inner, base);
    }
    let b = base / ps;
    lemma_aligned_addr_index(base);
    // Lower bound: the base frame (index `b`) is in range.
    lemma_free_contains(inner, base);
    // Upper bound: the last frame (index `b + count - 1`) is in range.
    assert(frames.contains(base + (count - 1) * ps));
    lemma_frame_addr_split(b, count - 1);
    lemma_frame_addr_div(b + count - 1);
    lemma_frame_addr_mod_zero(b + count - 1);
    lemma_free_contains(inner, base + (count - 1) * ps);
    // Every bit in `[b, b + count)` is clear.
    assert forall|k: int| b <= k < b + count implies !inner.bitmap@.is_bit_set(k) by {
        let i = k - b;
        lemma_frame_addr_split(b, i);
        lemma_frame_addr_div(b + i);
        lemma_frame_addr_mod_zero(b + i);
        assert(frames.contains(base + i * ps));
        lemma_free_contains(inner, base + i * ps);
    }
    assert(inner.bitmap@.has_free_range_at(b, count));
}

/// Contrapositive helper: if the bitmap has no contiguous run of `count` clear bits, then the
/// abstract view has no contiguous run of `count` free frames.
proof fn lemma_no_bitmap_range_implies_no_free_run(inner: &Inner, count: int)
    requires
        inner.internal_inv(),
        count > 0,
        !inner.bitmap@.exists_contiguous_free_range(count),
    ensures
        !inner@.exists_contiguous_free_run(count),
{
    if inner@.exists_contiguous_free_run(count) {
        let base = choose|base: int| inner@.contiguous_free_run_at(base, count);
        lemma_free_run_implies_bitmap_range(inner, base, count);
    }
}

/// No-overflow / divisibility geometry for `alloc_range`: a non-empty page-aligned region spans at
/// least one frame, the exclusive upper bound divides cleanly, and `start + nframes` does not
/// overflow `usize`.
proof fn lemma_alloc_range_geometry(rstart: int, rsize: int, ps: int, start_fn: int, nfr: int)
    requires
        ps == spec_page_size(),
        ps == 4096,
        rsize > 0,
        rstart % ps == 0,
        rsize % ps == 0,
        rstart <= usize::MAX as int,
        rsize <= usize::MAX as int,
        start_fn == rstart / ps,
        nfr == rsize / ps,
    ensures
        nfr >= 1,
        (rstart + rsize) / ps == start_fn + nfr,
        start_fn <= usize::MAX as int / ps,
        nfr <= usize::MAX as int / ps,
        start_fn + nfr <= usize::MAX as int,
{
    lemma_size_div_pos(rsize);
    lemma_aligned_div_sum(rstart, rsize);
    vstd::arithmetic::div_mod::lemma_div_is_ordered(rstart, usize::MAX as int, ps);
    vstd::arithmetic::div_mod::lemma_div_is_ordered(rsize, usize::MAX as int, ps);
}

/// A requested range index that is not covered by the bitmap cannot be free, so the whole requested
/// `frames` set is not all-free.
proof fn lemma_range_uncovered_not_all_free(
    inner: &Inner,
    index: int,
    rstart: int,
    rsize: int,
    ps: int,
    start_fn: int,
    nfr: int,
    g_old: FrameAllocView,
)
    requires
        inner.internal_inv(),
        ps == spec_page_size(),
        g_old == inner@,
        0 <= start_fn,
        rstart / ps == start_fn,
        (rstart + rsize) / ps == start_fn + nfr,
        start_fn <= index < start_fn + nfr,
        index >= inner.bitmap@.num_bits,
    ensures
        !g_old.all_free(Set::range(rstart / ps, (rstart + rsize) / ps).map(|i: int| i * spec_page_size())),
{
    broadcast use vstd::set_lib::group_set_lib_default;
    let a = frame_addr_of(index);
    let frame_numbers = Set::range(rstart / ps, (rstart + rsize) / ps);
    let frames = frame_numbers.map(|i: int| i * spec_page_size());
    assert(frame_numbers.contains(index));
    assert(frames.contains(a));
    lemma_frame_addr_mod_zero(index);
    lemma_frame_addr_div(index);
    lemma_covered_iff(inner, a);
    assert(!g_old.is_covered(a));
    assert(!g_old.is_free(a));
    if g_old.all_free(frames) {
        assert(g_old.is_free(a));
    }
}

/// A requested range index that is covered but already allocated is not free, so the whole
/// requested `frames` set is not all-free.
proof fn lemma_range_allocated_not_all_free(
    inner: &Inner,
    index: int,
    rstart: int,
    rsize: int,
    ps: int,
    start_fn: int,
    nfr: int,
    g_old: FrameAllocView,
)
    requires
        inner.internal_inv(),
        ps == spec_page_size(),
        g_old == inner@,
        0 <= start_fn,
        rstart / ps == start_fn,
        (rstart + rsize) / ps == start_fn + nfr,
        start_fn <= index < start_fn + nfr,
        index < inner.bitmap@.num_bits,
        inner.bitmap@.set_bits.contains(index),
    ensures
        !g_old.all_free(Set::range(rstart / ps, (rstart + rsize) / ps).map(|i: int| i * spec_page_size())),
{
    broadcast use vstd::set_lib::group_set_lib_default;
    let a = frame_addr_of(index);
    let frame_numbers = Set::range(rstart / ps, (rstart + rsize) / ps);
    let frames = frame_numbers.map(|i: int| i * spec_page_size());
    assert(frame_numbers.contains(index));
    assert(frames.contains(a));
    lemma_frame_addr_mod_zero(index);
    lemma_frame_addr_div(index);
    lemma_alloc_contains(inner, a);
    assert(g_old.is_allocated(a));
    assert(!g_old.is_free(a));
    if g_old.all_free(frames) {
        assert(g_old.is_free(a));
    }
}

/// Post-state of `alloc_range`: re-establish `internal_inv` after booking, prove the requested
/// `frames` set was all-free in the pre-state, and reconstruct the post-state view as the pre-state
/// map merged with the booked run (count 1). `frames` is the contract's
/// `{ i * PS | start_fn <= i < start_fn + nfr }`.
proof fn lemma_alloc_range_post(
    inner: &Inner,
    rstart: int,
    rsize: int,
    ps: int,
    lo: int,
    nfr: int,
    g_old: FrameAllocView,
    pre_sb: Set<int>,
    pre_nb: int,
    pre_rc: Seq<u8>,
)
    requires
        ps == spec_page_size(),
        ps > 0,
        rstart / ps == lo,
        (rstart + rsize) / ps == lo + nfr,
        0 <= lo,
        nfr >= 1,
        lo + nfr <= pre_nb,
        pre_nb <= pre_rc.len(),
        forall|i: int| 0 <= i < pre_nb ==> (#[trigger] pre_sb.contains(i) <==> pre_rc[i] > 0),
        forall|i: int| 0 <= i < pre_nb ==> (!(#[trigger] pre_sb.contains(i)) <==> pre_rc[i] == 0),
        forall|i: int| pre_nb <= i < pre_rc.len() ==> #[trigger] pre_rc[i] == 0,
        forall|i: int| 0 <= i < pre_nb ==> {
            &&& #[trigger] frame_addr_of(i) >= 0
            &&& frame_addr_of(i) <= usize::MAX as int
            &&& i <= FrameNumber::spec_max() as int
        },
        forall|j: int| lo <= j < lo + nfr ==> !pre_sb.contains(j),
        inner.bitmap.inv(),
        inner.bitmap@.num_bits == pre_nb,
        inner.bitmap@.set_bits == pre_sb.union(BitmapView::range_set(lo, lo + nfr)),
        spec_refcount_seq(inner).len() == pre_rc.len(),
        forall|k: int| 0 <= k < pre_rc.len() ==>
            #[trigger] spec_refcount_seq(inner)[k] == (if lo <= k < lo + nfr { 1u8 } else { pre_rc[k] }),
        g_old == view_of(pre_sb, pre_nb, pre_rc),
    ensures
        inner.internal_inv(),
        ({
            let frames = Set::range(rstart / ps, (rstart + rsize) / ps).map(|i: int| i * spec_page_size());
            &&& g_old.all_free(frames)
            &&& inner@ == (FrameAllocView {
                refcounts: g_old.refcounts.union_prefer_right(Map::new(frames, |addr: int| 1int)),
            })
        }),
{
    lemma_reestablish_inv_range(inner, pre_sb, pre_nb, pre_rc, lo, lo + nfr);
    broadcast use
        vstd::set_lib::group_set_lib_default,
        vstd::map::group_map_lemmas,
        vstd::map_lib::group_map_properties;
    lemma_view_of(inner);
    assert forall|x: int| pre_sb.contains(x) implies 0 <= x < pre_nb by {
        assert(inner.bitmap@.set_bits.contains(x));
    }
    lemma_reserve_range_v(pre_sb, pre_nb, pre_rc, inner.refcount@, lo, nfr);
    let frame_numbers = Set::range(rstart / ps, (rstart + rsize) / ps);
    let frames = frame_numbers.map(|i: int| i * spec_page_size());
    assert(frame_numbers == Set::range(lo, lo + nfr));
    assert(frames =~= spec_range_frames(lo, nfr)) by {
        assert forall|addr: int| #[trigger] frames.contains(addr) implies
            spec_range_frames(lo, nfr).contains(addr) by {
            let i = choose|i: int|
                frame_numbers.contains(i) && addr == #[trigger] (i * spec_page_size());
            lemma_frame_addr_div(i);
            assert(lo <= i < lo + nfr && addr == frame_addr_of(i));
        }
        assert forall|addr: int| spec_range_frames(lo, nfr).contains(addr) implies
            #[trigger] frames.contains(addr) by {
            let i = addr / spec_page_size();
            lemma_frame_addr_div(i);
            assert(frame_numbers.contains(i));
            assert(addr == i * spec_page_size());
        }
    }
    assert forall|addr: int| #[trigger] frames.contains(addr) implies {
        &&& addr % spec_page_size() == 0
        &&& 0 <= addr / spec_page_size() < pre_nb
        &&& pre_rc[addr / spec_page_size()] == 0
    } by {
        let i = choose|i: int|
            frame_numbers.contains(i) && addr == #[trigger] (i * spec_page_size());
        assert(lo <= i < lo + nfr);
        lemma_frame_addr_div(i);
        lemma_frame_addr_mod_zero(i);
        assert(!pre_sb.contains(i));
    }
    lemma_view_of_all_free(pre_sb, pre_nb, pre_rc, frames);
    assert(Map::new(frames, |addr: int| 1int)
        =~= Map::new(spec_range_frames(lo, nfr), |addr: int| 1int));
    assert(inner@.refcounts =~= g_old.refcounts.union_prefer_right(
        Map::new(frames, |addr: int| 1int)));
    assert(inner@ == (FrameAllocView {
        refcounts: g_old.refcounts.union_prefer_right(Map::new(frames, |addr: int| 1int)),
    }));
}

}
