// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// identity_map - Specifications
//
// To its callers, `mm::virt::identity_map` is the **kernel lazy identity map**:
// a partial, monotonically-growing set of physical pages reachable
// (identity-mapped) through the kernel address space, plus a one-shot
// "is the mapper live yet?" status. The only externally meaningful operation in
// scope, `identity_map_page`, is an idempotent side effect -- "make sure this
// physical page is reachable" -- not a query returning data.
//
// The module operates over module-global `static`s (`KERNEL_PD_PADDR`,
// `KERNEL_CR3`) and raw page-table memory, none of which a Verus `spec fn` can
// read directly. The abstract state is therefore exposed through the
// uninterpreted accessor `identity_map_view()`, exactly mirroring the
// `phys_view()` pattern of `mm::phys` (`mod.spec.rs`): the exec shims pin down
// the caller-relevant facts through their `ensures` clauses. Because a single
// fixed accessor cannot name both a pre-state and a post-state of a global
// mutation (there is no `old(identity_map_view())`), the exec contracts state
// monotone single-state facts; the full `old -> new` transition vocabulary
// (`spec_identity_map_page`) and its laws live here and in
// `identity_map.proof.rs` for the manager-level reasoning that can thread an
// authority token.
//
// See `verus-ai-logs/nanvix-phys-virt-identity-map/view_design.md` for the
// design rationale.

verus! {

use vstd::set::Set;
use crate::hal::mem::spec_page_size;

/// Abstract state of the kernel lazy identity map.
///
/// This is the caller-visible state behind the free functions in
/// `mm::virt::identity_map` (which operate over module-global statics, not a
/// `self` receiver). `identity_map_page` is specified as a transition on this
/// View; the private helpers refine it.
#[verifier::ext_equal]
pub struct IdentityMapView {
    /// Whether `init` has published the kernel page directory.
    ///
    /// Before this is `true` (boot page tables still active), every
    /// `identity_map_page` call is a no-op success that leaves `mapped`
    /// unchanged. Callers are required to tolerate this pre-init no-op.
    pub initialized: bool,

    /// Page frame numbers currently reachable through the kernel identity map
    /// (the page's PTE is present). `frame == phys_addr / PAGE_SIZE`.
    ///
    /// A page-aligned physical address `p` is reachable iff
    /// `mapped.contains(p / PAGE_SIZE)`. The set only ever grows (mappings are
    /// never torn down by the in-scope functions).
    pub mapped: Set<nat>,
}

impl IdentityMapView {
    /// Largest-valid-frame bound: a frame number is addressable iff it is
    /// strictly below this. Corresponds to the hardware `FrameNumber` range
    /// whose violation `ensure_pte` / `identity_map_page` report as
    /// `ErrorCode::BadAddress` (= physical-address-space size / PAGE_SIZE).
    pub open spec fn max_frames() -> nat {
        (::arch::mem::MAX_ADDRESS as nat) / (::arch::mem::PAGE_SIZE as nat)
    }

    /// Well-formedness invariant.
    ///
    /// Every reachable page denotes a valid physical frame. No partially
    /// installed / out-of-range frame is ever recorded as mapped, mirroring the
    /// all-or-nothing failure guarantee. Intentionally weak: PDE pre-allocation
    /// over `[0, MEMORY_SIZE)` after `init` is a structural property of the
    /// kernel page directory, not of the page-reachability set.
    pub open spec fn inv(self) -> bool {
        forall|f: nat| #[trigger] self.mapped.contains(f) ==> f < Self::max_frames()
    }

    /// Address-level reachability query: is the page covering `phys_addr`
    /// identity-mapped in the kernel address space?
    pub open spec fn maps(self, phys_addr: int) -> bool {
        self.mapped.contains((phys_addr / spec_page_size()) as nat)
    }

    /// Effect of identity-mapping the page with frame number `frame`.
    ///
    /// - Pre-init: no-op (callers must tolerate this).
    /// - Live: the frame becomes reachable. `Set::insert` is idempotent, so
    ///   re-mapping an already-mapped page is automatically a no-op success
    ///   (captures the "idempotent" caller expectation with no special case).
    pub open spec fn spec_identity_map_page(self, frame: nat) -> IdentityMapView {
        if self.initialized {
            IdentityMapView { mapped: self.mapped.insert(frame), ..self }
        } else {
            self
        }
    }
}

/// Abstract view of the kernel lazy identity map at the current program point.
///
/// The subsystem state lives in module-level `static`s (`KERNEL_PD_PADDR` /
/// `KERNEL_CR3`) plus raw page-table memory that a Verus `spec fn` cannot read
/// directly, so this is an *uninterpreted* accessor: the exec shims in
/// `identity_map.rs` pin down its value through their `ensures` clauses.
/// `initialized` mirrors `KERNEL_PD_PADDR != 0`; `mapped` mirrors the set of
/// frames whose kernel PTE is present. This mirrors the `phys_view()` accessor
/// of `mm::phys`.
pub uninterp spec fn identity_map_view() -> IdentityMapView;

} // verus!
