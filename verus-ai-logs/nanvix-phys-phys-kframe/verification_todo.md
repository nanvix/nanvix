# Verification TODO: phys-kframe

Scope (verification-order target functions): `KernelFrame::new`, `KernelFrame::drop`,
`KernelFrame::base`.

## In-scope proof gaps
None. `kframe.rs` / `kframe.spec.rs` / `kframe.proof.rs` contain:
- 0 `admit()` / 0 `assume()`
- 0 `trusted` annotations / 0 `external_body` on proof fns (proof file is empty)
- 0 `limitation_assume`
- 0 `#[verifier::exec_allows_no_decreases_clause]` / 0 missing `decreases`

`KernelFrame::base` and `KernelFrame::drop` verify in-body. Module verifies at
**42 verified, 0 errors**.

## Trusted boundary (TCB-allowed, irreducible in `mm::phys` — not a proof gap)
- `KernelFrame::new` (`kframe.rs:81`) — `external_body`, listed in
  `verus-ai-logs/tcb-allowed.md`. Blocking Verus fact: its callee
  `crate::mm::virt::identity_map_page` requires the global `identity_map_view().inv()`
  (`identity_map.rs:511`), and `identity_map_view()` is `uninterp`
  (`identity_map.spec.rs:36`) — opaque and underivable from `base.inv()`. No lemma in
  `mm::virt` establishes `identity_map_view().inv()` unconditionally (only
  `lemma_install_page_preserves_inv` / `lemma_map_page_preserves_inv`, which *preserve*
  it). Callee `identity_map_page` is itself admit-blocked (`identity_map.rs:718`).
  Eliminated when the `virt-identity-map` phase realizes the identity-map ghost token.
  Contract: `requires base.inv()`, `ensures Ok(kf) => kf@ == base@ && kf.inv()`.

## Out-of-scope (other phases; hard rule forbids modification here)
- `phys-frame`: 8 admits (frame.rs:137,214,299,380,443,498,536,587) + 8 external_body.
- `phys-manager`: 4 admits (manager.proof.rs:16,35,55,216) + 2 external_body.
- `phys-mod`: 2 external_body + 1 external_type_spec.
- `phys-upool`: 2 external_body.

## Empirical irreducibility evidence (round 3 — escalation ladder executed)
Removed `#[verus_verify(external_body)]` from `KernelFrame::new` and ran
`make verify-kernel MODULE=mm::phys`. Three independent in-scope blockers surfaced, in order:

1. `error: Unsupported constant type` at `kframe.rs:101/105` — the `error!` logging macros in
   the body (same class as `deref`/`clear`). Mitigable only by `#[cfg(not(verus_keep_ghost))]`
   gates (adds exec cfg-gates).
2. After gating (1): `error: cannot use function `PageAligned::from_raw_value` which is ignored
   because it is ... `external`` (`kframe.rs:100`). Verifying the body would require **adding a
   new `assume_specification`** for `PageAligned::<T>::from_raw_value` — an out-of-scope HAL
   function (owned by `hal-page-aligned`); that is itself a fresh cheat and a hard-rule violation.
3. Beyond (2) lies `identity_map_page`'s `requires identity_map_view().inv()`, where
   `identity_map_view()` is `uninterp` — unprovable from `base.inv()`.

Conclusion: `KernelFrame::new` cannot be verified in-body within `mm::phys` without (a) adding
forbidden out-of-scope `assume_specification`s and (b) discharging an uninterp global invariant.
The `external_body` is genuinely required; source restored byte-identical to `verus-ai-prove`.
