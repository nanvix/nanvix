// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// identity_map - Proofs
//
// Abstract laws backing the caller-facing guarantees of the lazy identity map.
// They are stated over the `IdentityMapView` transition vocabulary
// (`spec_identity_map_page`) defined in `identity_map.spec.rs`, and capture the
// key invariants from the caller analysis: idempotence, map-on-success,
// monotone growth, and invariant preservation. The exec shims in
// `identity_map.rs` are trusted boundaries (they touch global statics and raw
// page-table memory Verus cannot model); these lemmas give the proving phase
// and any future manager-level reasoning -- which can thread an authority token
// to name `old`/`new` views -- the laws relating one view to the next.
//
// Bodies are left as `admit()` during the specification phase; the proving
// phase discharges them.

verus! {

/// Idempotence: re-mapping an already-reachable page (when the mapper is live)
/// leaves the abstract state unchanged. Backs the caller expectation that
/// `identity_map_page` is a safe no-op on an already-mapped page (e.g. per page
/// in a range).
pub proof fn lemma_map_idempotent(v: IdentityMapView, frame: nat)
    requires
        v.initialized,
        v.mapped.contains(frame),
    ensures
        v.spec_identity_map_page(frame) =~= v,
{
    admit();
}

/// Map-on-success: after mapping the page covering `phys_addr` on a live mapper,
/// that address is reachable. Backs `identity_map_page`'s / `ensure_pte`'s
/// success postcondition (`maps(phys_addr)`).
pub proof fn lemma_map_on_success(v: IdentityMapView, phys_addr: int)
    requires
        v.initialized,
        phys_addr >= 0,
        spec_page_size() > 0,
    ensures
        v.spec_identity_map_page((phys_addr / spec_page_size()) as nat).maps(phys_addr),
{
    admit();
}

/// Monotone growth: mapping a page never removes any previously reachable page
/// and never flips the lifecycle flag. Backs the "mappings are never torn down"
/// invariant the range loop in `ensure_identity_mapped_range` relies on.
pub proof fn lemma_map_monotone(v: IdentityMapView, frame: nat)
    ensures
        v.mapped.subset_of(v.spec_identity_map_page(frame).mapped),
        v.spec_identity_map_page(frame).initialized == v.initialized,
{
    admit();
}

/// Invariant preservation: mapping an in-range frame keeps every recorded frame
/// valid. Backs the all-or-nothing guarantee -- a successful mapping never
/// records an out-of-range frame.
pub proof fn lemma_map_preserves_inv(v: IdentityMapView, frame: nat)
    requires
        v.inv(),
        frame < IdentityMapView::max_frames(),
    ensures
        v.spec_identity_map_page(frame).inv(),
{
    admit();
}

} // verus!
