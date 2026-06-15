// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::vstd::layout::valid_layout;

// ==================================================================================================
// Kheap — Specifications
//
// Abstract view of the kernel heap allocator as seen by its callers:
//   KheapView { allocations: Map<base_addr, size> }
// ==================================================================================================

verus! {

// --------------------------------------------------------------------------------------------------
// External specifications (core::alloc)
// --------------------------------------------------------------------------------------------------

// Layout is opaque — Verus must treat its contents as abstract.
#[verifier::external_body]
#[verifier::external_type_specification]
pub struct ExLayout(core::alloc::Layout);

#[verifier::external_type_specification]
pub struct ExAllocError(core::alloc::AllocError);

// Logical projections of Layout — uninterpreted so we can refer to them in specs.
pub uninterp spec fn spec_layout_size(layout: core::alloc::Layout) -> usize;
pub uninterp spec fn spec_layout_align(layout: core::alloc::Layout) -> usize;

pub assume_specification[ core::alloc::Layout::size ](layout: &core::alloc::Layout) -> (result: usize)
    ensures
        result == spec_layout_size(*layout),
;

pub assume_specification[ core::alloc::Layout::align ](layout: &core::alloc::Layout) -> (result: usize)
    ensures
        result == spec_layout_align(*layout),
        valid_layout(spec_layout_size(*layout), result),
;

// --------------------------------------------------------------------------------------------------
// KheapView — abstract state exposed to callers
//
// Follows the caller-observable distillation: the only thing a caller can
// observe about the heap is which addresses are currently live and how many
// bytes each holds. Everything else — capacity, tiering, alignment, base —
// is an implementation concern that lives in the concrete `inv()`.
// --------------------------------------------------------------------------------------------------

/// Abstract state of the kernel heap allocator from the caller's perspective.
#[verifier::ext_equal]
pub ghost struct KheapView {
    /// Live allocations keyed by base address: addr -> size in bytes.
    pub allocations: Map<int, nat>,
}

impl KheapView {
    /// Well-formedness invariant visible to callers.
    ///
    /// Matches HeapSpec's H-INV-1/H-INV-2: every allocation has positive size
    /// at a non-null address, and no two allocations overlap.
    pub open spec fn inv(&self) -> bool {
        &&& forall|a: int| #[trigger] self.allocations.dom().contains(a) ==> {
                &&& self.allocations[a] > 0
                &&& a > 0
            }
        &&& forall|a1: int, a2: int| #![auto]
                self.allocations.dom().contains(a1)
                && self.allocations.dom().contains(a2)
                && a1 != a2
                ==> a1 + self.allocations[a1] as int <= a2
                    || a2 + self.allocations[a2] as int <= a1
    }

    /// Abstract state of a freshly constructed heap: no live allocations.
    pub open spec fn new() -> KheapView {
        KheapView { allocations: Map::empty() }
    }

    /// Abstract effect of a successful allocation at `addr`.
    pub open spec fn spec_allocate(self, addr: int, size: nat) -> KheapView {
        KheapView { allocations: self.allocations.insert(addr, size) }
    }

    /// Abstract effect of a successful deallocation at `addr`.
    pub open spec fn spec_deallocate(self, addr: int) -> KheapView {
        KheapView { allocations: self.allocations.remove(addr) }
    }
}

impl Kheap {
    /// Public invariant: abstract well-formedness plus an implementation-owned invariant.
    pub open spec fn inv(&self) -> bool {
        &&& self@.inv()
        &&& self.internal_inv()
    }
}
} // verus!
