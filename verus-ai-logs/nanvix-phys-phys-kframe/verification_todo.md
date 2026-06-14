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
