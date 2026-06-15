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
// `PageAligned<T>` is generic and the generic `View` impl for `PageAligned<T>`
// cannot name a per-`T` `View` (it must project through the universal
// `spec_addr`). The identity below is therefore an **external-bottom trust
// boundary**, not a provable lemma: it is the single fact that connects the
// universal `spec_addr` projection of a `PhysicalAddress` to that type's own
// `View`, and it becomes derivable only once the `sys::mm::Address` trait `impl`
// for `PhysicalAddress` is itself verified (its raw-pointer-cast sibling methods
// currently keep it below this module's verification boundary — see
// `verus-ai-logs/verus-unsupported.md`). The specification phase reviewed the
// claim as semantically sound and deferred its discharge to the proving phase;
// the proving phase discharges it here with the governed `axiom fn` mechanism
// (no `admit`, no `external_body`), registered in `verus-ai-logs/tcb-allowed.md`.

verus! {

// Bridge: for any `PhysicalAddress`, the universal `spec_addr` projection
// coincides with its `View`. Used by `FrameAddress::from_frame_number`,
// `from_raw_value`, and `into_frame_number` to carry a `PhysicalAddress`'s
// `@`-based guarantees across `PageAligned::from_address`'s / `Deref::deref`'s
// `spec_addr`-based contracts. Removed when `sys::mm::Address` is verified.
pub axiom fn lemma_phys_view_is_spec_addr(pa: PhysicalAddress)
    ensures
        crate::hal::mem::spec_addr(&pa) == pa@,
;

} // verus!
