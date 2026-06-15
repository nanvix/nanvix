// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// FrameAddress — Proofs
//
// `FrameAddress` is an immutable scalar value, so there are no state-transition
// lemmas. The single proof obligation that cannot be discharged from the
// dependency contracts alone is the *bridge* between the two projections of a
// `PhysicalAddress`'s abstract value:
//
//   * `PhysicalAddress`'s own `View` (`pa@`), used by `PhysicalAddress::from_number`
//     / `into_frame_number`; and
//   * the universal address projection `spec_addr(&pa)`, used by
//     `PageAligned::from_address` (which is generic over `T: Address` and cannot
//     name a per-`T` `View`).
//
// They denote the same integer; `spec_addr` is left uninterpreted only because
// `PageAligned<T>` is generic. The bridge lemma identifies them. Its body is
// `admit()` during the specification phase; the proving phase discharges it once
// the `Address`-trait address projection is connected to the per-type `View`.

verus! {

use crate::hal::mem::spec_addr;

// Bridge: for any `PhysicalAddress`, the universal `spec_addr` projection
// coincides with its `View`. Used by `FrameAddress::from_frame_number` to carry
// `PhysicalAddress::from_number`'s `@`-based guarantee across
// `PageAligned::from_address`'s `spec_addr`-based contract.
pub proof fn lemma_phys_view_is_spec_addr(pa: PhysicalAddress)
    ensures
        spec_addr(&pa) == pa@,
{
    admit();
}

} // verus!
