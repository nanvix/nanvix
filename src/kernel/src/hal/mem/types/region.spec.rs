// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Memory Region — Specifications
//
// A memory region is a contiguous, non-wrapping half-open byte interval of the
// address space, `[start, start + size)`, tagged with metadata
// (type / permissions / cache policy). `TruncatedMemoryRegion<T>` is the same
// interval additionally constrained so both endpoints are page-aligned, making
// it directly usable for frame/page booking. Both region types share the single
// `MemoryRegionView` abstraction (the truncated wrapper forwards `view()` to its
// inner region).
//
// Addresses are abstracted to `int` through the universal `spec_addr` projection
// (defined in `page.spec.rs`, `pub uninterp spec fn spec_addr<T: Address>`),
// exactly as `PageAligned<T>` does. Using `spec_addr` — rather than a
// `T: View<V = int>` bound on the `View` impl — is what lets the `View` (and the
// `@`-based accessor contracts) be stated for a bare `T: Address`: the
// address-family `View` impls are all `cfg(verus_keep_ghost)`-gated and the
// generic exec `impl<T: Address> ...` blocks carry no `View` bound, so a
// conditional `View<V = int>` bound would make `self@` untypable inside the
// accessors. `spec_page_size()` is the canonical page size, re-exported from
// `crate::hal::mem`.

verus! {

use crate::hal::mem::spec_page_size;
use crate::hal::mem::spec_addr;

// Clone is value-preserving for the `Address` family: every address type is a
// newtype over a plain integer, so cloning copies the abstract address. This
// fact is what `MemoryRegion::start` (`self.start.clone()`) needs to discharge
// `spec_addr(&result) == self@.start`. It cannot be stated with
// `assume_specification` because the cloned receiver is a bare type parameter
// (`<T as Clone>::clone` — Verus rejects generic trait-method specs). It is
// therefore attached to the `Address` trait via `external_trait_specification`,
// the Verus-sanctioned mechanism for adding a spec to a trait method that all
// implementers honor (declared here in the kernel crate so it can reference the
// crate-local `spec_addr`). `Address: Clone` (supertrait), so `clone` is in the
// trait's method surface. External impls are trusted; verified impls must prove
// it — discharged when the `Address` family is verified.
#[verifier::external_trait_specification]
#[verifier::external_trait_extension(CloneAddrSpec via CloneAddrSpecImpl)]
pub trait ExCloneAddr: Sized {
    type ExternalTraitSpecificationFor: Clone;

    spec fn clone_view(&self) -> int;

    fn clone(&self) -> (result: Self)
        ensures
            result.clone_view() == self.clone_view(),
    ;
}

// Abstract value of a memory region: the geometry `(start, size)` (in bytes,
// as mathematical integers) plus the three metadata tags callers observe.
pub struct MemoryRegionView {
    /// First valid byte address of the region. Equals the `start` supplied at
    /// construction; for the truncated variant it is page-aligned. Also the
    /// `Ord` key for both region types.
    pub start: int,
    /// Length of the region in bytes. The interval is `[start, start + size)`.
    /// For the truncated variant it is a multiple of the page size.
    pub size: int,
    /// Classification of the region (Usable / Reserved / Mmio / Bad).
    pub typ: MemoryRegionType,
    /// Access permissions granted over the region.
    pub perm: AccessPermission,
    /// Optional MMIO caching policy; `None` for non-MMIO regions.
    pub cache_policy: Option<MmioCachePolicy>,
}

impl MemoryRegionView {
    /// Non-empty, non-wrapping interval lying within the address space. This is
    /// the geometry every constructed region guarantees and that caller
    /// arithmetic (`size - 1`, `start + size - 1`, `size / FRAME_SIZE`) relies
    /// on: `size >= 1` (`new` rejects `size == 0`) and the inclusive end
    /// `start + size - 1` never overflows the widest address representation.
    pub open spec fn wf_geometry(self) -> bool {
        &&& self.size >= 1
        &&& self.start >= 0
        &&& self.start + self.size <= usize::MAX as int + 1
    }

    /// Exclusive end of the half-open interval `[start, end)`.
    pub open spec fn spec_end(self) -> int {
        self.start + self.size
    }

    /// Inclusive last byte address (callers compute `start + size - 1`).
    pub open spec fn spec_last(self) -> int {
        self.start + self.size - 1
    }

    /// Whether `addr` lies in `[start, start + size)`.
    pub open spec fn contains(self, addr: int) -> bool {
        self.start <= addr < self.start + self.size
    }

    /// State transition of `MemoryRegion::set_cache_policy`: only the cache
    /// policy changes; geometry and the other tags are preserved (`..self`).
    pub open spec fn spec_set_cache_policy(self, policy: MmioCachePolicy) -> MemoryRegionView {
        MemoryRegionView { cache_policy: Some(policy), ..self }
    }
}

// `MemoryRegion<T>` abstracts to its geometry plus tags. The `view()` is
// `closed`: callers reference `self@.start` / `self@.size` but the field-to-
// storage mapping (and the `spec_addr` projection of the typed `start`) does not
// leak. Unconditional over `T: Address` (see module comment).
impl<T: Address> View for MemoryRegion<T> {
    type V = MemoryRegionView;

    closed spec fn view(&self) -> MemoryRegionView {
        MemoryRegionView {
            start: spec_addr(&self.start),
            size: self.size as int,
            typ: self.typ,
            perm: self.perm,
            cache_policy: self.cache_policy,
        }
    }
}

// `TruncatedMemoryRegion<T>` is the same interval with a stronger invariant; its
// `view()` forwards to the inner `MemoryRegion<PageAligned<T>>`.
impl<T: Address> View for TruncatedMemoryRegion<T> {
    type V = MemoryRegionView;

    closed spec fn view(&self) -> MemoryRegionView {
        self.0@
    }
}

// Well-formedness of a base memory region: non-empty, non-wrapping geometry.
impl<T: Address> MemoryRegion<T> {
    pub open spec fn inv(&self) -> bool {
        self@.wf_geometry()
    }
}

// Well-formedness of a truncated region: base geometry plus page alignment of
// both endpoints (established by `TruncatedMemoryRegion::new`'s `align_up`), so
// `size / FRAME_SIZE` is exact and `start` needs no re-alignment.
impl<T: Address> TruncatedMemoryRegion<T> {
    pub open spec fn inv(&self) -> bool {
        &&& self@.wf_geometry()
        &&& self@.start % spec_page_size() == 0
        &&& self@.size % spec_page_size() == 0
    }
}

} // verus!
