verus! {

use crate::hal::mem::spec_page_size;

//==================================================================================================
// View
//==================================================================================================

/// Abstract state of a memory region: the half-open geometry `[start, start + size)`
/// together with the caller-visible metadata. Shared by `MemoryRegion<T>` and its
/// page-snapped newtype `TruncatedMemoryRegion<T>`.
pub struct MemoryRegionView {
    /// Numeric base address of the region (raw value of `start`). Sole ordering key.
    pub start: int,
    /// Byte length of the region; the range covered is `[start, start + size)`.
    pub size: int,
    /// Region classification.
    pub typ: MemoryRegionType,
    /// Access permission attached to the region.
    pub perm: AccessPermission,
    /// Optional MMIO caching policy; `None` for non-MMIO regions.
    pub cache_policy: Option<MmioCachePolicy>,
}

impl MemoryRegionView {
    /// Geometry well-formedness shared by every region kind: the range is non-empty.
    pub open spec fn wf(self) -> bool {
        self.size > 0
    }

    /// Page-granular geometry: both endpoints sit on a page boundary.
    pub open spec fn is_page_aligned(self) -> bool {
        &&& self.start % spec_page_size() == 0
        &&& self.size % spec_page_size() == 0
    }

    /// Abstract transition for `set_cache_policy`: only `cache_policy` changes.
    pub open spec fn spec_set_cache_policy(self, policy: MmioCachePolicy) -> MemoryRegionView {
        MemoryRegionView { cache_policy: Some(policy), ..self }
    }
}

impl<T: Address + View<V = int>> View for MemoryRegion<T> {
    type V = MemoryRegionView;

    closed spec fn view(&self) -> MemoryRegionView {
        MemoryRegionView {
            start: self.start@,
            size: self.size as int,
            typ: self.typ,
            perm: self.perm,
            cache_policy: self.cache_policy,
        }
    }
}

impl<T: Address + View<V = int>> View for TruncatedMemoryRegion<T> {
    type V = MemoryRegionView;

    closed spec fn view(&self) -> MemoryRegionView {
        self.0@
    }
}

//==================================================================================================
// Invariants
//==================================================================================================

impl<T: Address + View<V = int>> MemoryRegion<T> {
    /// A general memory region carries only the non-empty geometry guarantee.
    pub open spec fn inv(&self) -> bool {
        self@.wf()
    }
}

impl<T: Address + View<V = int>> TruncatedMemoryRegion<T> {
    /// A truncated region additionally guarantees page-aligned geometry — the
    /// load-bearing property for frame-count and MMIO overlap arithmetic.
    pub open spec fn inv(&self) -> bool {
        &&& self@.wf()
        &&& self@.is_page_aligned()
    }
}

//==================================================================================================
// Dependency contracts for the not-yet-verified address layer.
//
// Cloning a `T: Address` yields an equal abstract address. `Clone::clone` is a
// std-library trait method ("returns a copy of the value"); view preservation is
// the abstract consequence of that documented contract. This is a placeholder
// until the address layer exposes a clone specification of its own.
//==================================================================================================

pub assume_specification<T: Address>[ <T as Clone>::clone ](a: &T) -> (res: T)
    ensures
        res@ == a@,
;

} // verus!
