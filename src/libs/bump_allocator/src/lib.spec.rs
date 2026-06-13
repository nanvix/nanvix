// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// FixedSizeBumpAllocator - Specifications
//
// This file contains the abstract View (`BumpView`), its invariant and geometry
// helpers, the numeric specification of `align_up`, and the proof-lemma targets
// that encode the caller expectations. See
// `verus-ai-logs/nanvix-phys-bump-allocator/view_design.md` for the design
// rationale.
//
// NOTE: Attaching `BumpView` as the `View` of `FixedSizeBumpAllocator` is a later
// phase (view_design.md, end of section 2). It requires an atomic-ghost / PointsTo
// token to model the interior-mutable `AtomicUsize` cursor, because the raw atomic
// value is not readable in spec (vstd: "NO support for reasoning about the values
// inside the atomics"). Until then `BumpView` is referenced by the proof lemmas in
// `lib.proof.rs`, which state the caller-facing guarantees over the abstract pool.

verus! {

//==================================================================================================
// std-library specifications not yet covered by vstd
//==================================================================================================

// `usize::div_ceil` rounds the quotient towards positive infinity. It panics on a
// zero divisor, hence the `y != 0` precondition. For unsigned operands the result
// never overflows, and equals `ceil(x / y) == (x + y - 1) / y`.
pub assume_specification [ <usize>::div_ceil ](x: usize, y: usize) -> (result: usize)
    requires
        y != 0,
    ensures
        result as int == (x as int + y as int - 1) / (y as int),
;

//==================================================================================================
// align_up - numeric specification
//==================================================================================================

/// Ghost constant: the stable base address revealed by a backend `S`'s
/// `as_mut_ptr()`. Uninterpreted because a static's address is opaque to Verus.
pub uninterp spec fn base_of<S: ?Sized>() -> int;

/// Abstract address of a freshly handed-out slot reference.
///
/// Uninterpreted: a Verus `&mut T` reference carries no spec-readable address
/// (only raw pointers expose `.addr()`), so — mirroring `raw-array`'s uninterpreted
/// `view(&self) -> Seq<T>` — the address a caller observes for a returned slot is
/// modeled abstractly. `alloc`/`alloc_as` assert their alignment and in-bounds
/// guarantees over `slot_ref_addr(slot)`.
pub uninterp spec fn slot_ref_addr<T: ?Sized>(r: &T) -> int;

/// Least multiple of `alignment` that is `>= value`; `None` when `alignment == 0`
/// or when that multiple does not fit in a `usize`.
///
/// This is the View-independent numeric meaning of `align_up`. The allocator's
/// `stride` (see `BumpView::inv`) is pinned to this function.
pub open spec fn align_up_spec(value: nat, alignment: nat) -> Option<nat> {
    if alignment == 0 {
        None
    } else {
        let m: int = ((value + alignment - 1) / (alignment as int)) * (alignment as int);
        if m > usize::MAX as int {
            None
        } else {
            Some(m as nat)
        }
    }
}

//==================================================================================================
// BumpView - abstract pool model
//==================================================================================================

/// Abstract view of a `FixedSizeBumpAllocator<N, A, S>`.
///
/// Pure ghost description of the slot pool: no atomics, no raw pointers, no cursor
/// mechanism. Carries the fixed pool geometry plus the single dynamic quantity
/// `allocated` (the number of slots handed out so far).
#[verifier::ext_equal]
pub struct BumpView {
    /// Base address of the backing region (`S::as_mut_ptr()` as an integer).
    pub base: int,
    /// Distance in bytes between consecutive slots (`align_up(unit_size, unit_align)`).
    pub stride: nat,
    /// Size of each slot in bytes (the const generic `N`).
    pub unit_size: nat,
    /// Required alignment of each slot in bytes (the const generic `A`).
    pub unit_align: nat,
    /// Number of slots the pool can ever yield (`S::NUM_UNITS`).
    pub capacity: nat,
    /// Total size in bytes of the backing region (`S::STORAGE_SIZE`).
    pub storage_size: nat,
    /// Number of slots already handed out. The only dynamic field.
    pub allocated: nat,
}

impl BumpView {
    /// Address of slot `i` (0-based). The pool's geometry, independent of the
    /// allocation mechanism.
    pub open spec fn slot_addr(self, i: int) -> int {
        self.base + i * (self.stride as int)
    }

    /// A slot index is *consumed* iff it is below the high-water mark.
    pub open spec fn is_consumed(self, i: int) -> bool {
        0 <= i < self.allocated
    }

    /// The allocator still has at least one free slot.
    pub open spec fn has_capacity(self) -> bool {
        self.allocated < self.capacity
    }

    /// Well-formedness of the abstract pool. Each clause backs a caller-visible
    /// guarantee (see `view_design.md` section 3).
    pub open spec fn inv(self) -> bool {
        // (a) Geometry is well formed: non-empty, non-overlapping slots.
        &&& self.unit_size > 0
        &&& self.unit_align > 0
        &&& self.stride >= self.unit_size
        // (b) Stride is the up-alignment of the unit size to the unit alignment,
        //     and every slot start inherits `unit_align` from an aligned base.
        &&& align_up_spec(self.unit_size, self.unit_align) == Some(self.stride)
        &&& (self.stride as int) % (self.unit_align as int) == 0
        &&& self.base % (self.unit_align as int) == 0
        // (c) The pool fits inside the backing region.
        &&& (self.capacity as int) * (self.stride as int) <= self.storage_size as int
        // (d) Addresses do not wrap the usize space.
        &&& self.base >= 0
        &&& self.base + (self.storage_size as int) <= usize::MAX as int + 1
        // (e) Monotone-capacity ceiling.
        &&& self.allocated <= self.capacity
    }

    /// The three geometric guarantees the kernel's `unsafe` soundness relies on,
    /// stated over the whole pool: every slot is aligned, in-bounds, and distinct.
    pub open spec fn geometry_ok(self) -> bool {
        &&& (forall|i: int|
            0 <= i < self.capacity ==> #[trigger] self.slot_addr(i) % (self.unit_align as int) == 0)
        &&& (forall|i: int|
            0 <= i < self.capacity ==> {
                &&& self.base <= #[trigger] self.slot_addr(i)
                &&& self.slot_addr(i) + (self.unit_size as int) <= self.base + (self.storage_size
                    as int)
            })
        &&& (forall|i: int, j: int|
            (0 <= i < self.capacity && 0 <= j < self.capacity && i != j)
                ==> (#[trigger] self.slot_addr(i)) != (#[trigger] self.slot_addr(j)))
    }

    /// Abstract transition performed by a successful allocation: one more slot is
    /// consumed, nothing else changes.
    pub open spec fn spec_alloc(self) -> BumpView {
        BumpView { allocated: (self.allocated + 1) as nat, ..self }
    }
}

//==================================================================================================
// FixedSizeBumpAllocator - view accessor
//==================================================================================================

/// Abstract view of the allocator's slot pool.
///
/// Uninterpreted (mirrors `raw-array`'s `impl View for RawArray`): the dynamic
/// `allocated` field abstracts the interior-mutable `AtomicUsize` cursor, whose
/// value is not spec-readable. `inv()` pins `base/stride/unit_size/unit_align/
/// capacity/storage_size` to the type-level constants. The `v -> v'` transition
/// (cross-call uniqueness, `allocated + 1`) needs a ghost token and is deferred
/// to the proving phase (see `lemma_alloc_transition`, view_design.md section 7).
///
/// A free `uninterp spec fn` is used rather than an `impl View`/inherent `spec fn
/// view`, because adding a second `impl` block (trait or inherent) on
/// `FixedSizeBumpAllocator` alongside the exec method block collides during Verus
/// front-end lowering of this crate (`include!`-composed spec/proof modules) and
/// triggers a duplicate-impl-path panic (`vir/src/context.rs`). Callers read the
/// view via `bump_view(self)` exactly as they would `self.view()`.
impl<const N: usize, const A: usize, S: BssStorage> View for FixedSizeBumpAllocator<N, A, S> {
    type V = BumpView;
    uninterp spec fn view(&self) -> BumpView;
}

} // verus!

