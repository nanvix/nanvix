## Turn 1: Full proving checklist walkthrough — hal-phys-address

### Progress
- Done (all PASS, evidence below):
  1. No specs weakened
  2. Zero remaining `admit()`
  3. Zero `external_body` (none in module; TCB rule N/A)
  4. Zero `assume`/`assume_specification` beyond allowed external-bottom
  5. No cfg-gated exec code
  6. Cheating audit (counts + locations)
  7. Claimed Verus limitation → isolated reproducer
  8. Exec rewrites minimal & semantically equivalent
  9. Cross-module regression
  10. Verification + build (0 errors, 0 warnings)
- Current: none remaining
- Remaining: none

### Verification

Files reviewed:
- `src/kernel/src/hal/mem/types/address/phys.rs`
- `src/kernel/src/hal/mem/types/address/phys.spec.rs`
- `src/kernel/src/hal/mem/types/address/phys.proof.rs`

In-scope verified functions (per `verus-ai.toml`): `PhysicalAddress::from_mmio_address`,
`PhysicalAddress::from_number`, `PhysicalAddress::into_frame_number`, plus the `View`/`inv`
material. Supporting lemmas: `lemma_from_number_no_overflow`, `lemma_frame_index`.

**Item 1 — No specs weakened: PASS**
- `python3 scripts/spec_drift.py check hal-phys-address` (baseline `670499c050a3` → HEAD):
  `Contract drift: 0`, `Ensures removed: 0`, `Requires added: 0`, `✅ No contract drift`.
- `git diff 670499c0 -- phys.spec.rs` → empty (spec functions `inv`, `spec_frame_number`,
  `spec_from_number`, `spec_max_frame_number`, `spec_frame_raw_value` unchanged).
- `git diff` on `phys.rs` shows only a body change in `from_number` (hoisting
  `into_raw_value()` into a `let` before `proof!`); the three `#[verus_spec]` contracts are byte-identical.

**Item 2 — Zero `admit()`: PASS**
- `grep -c admit phys.{rs,spec.rs,proof.rs}` → `0/0/0`.
- The two former `admit()` placeholders in `phys.proof.rs` (`lemma_from_number_no_overflow`,
  `lemma_frame_index`) are now replaced by full proofs (div/mod + nonlinear_arith bound, and the
  `pow2(shift)==4096 ⇒ shift==12` + `lemma_usize_shr_is_div` chain). Confirmed in the diff.

**Item 3 — Zero `external_body`: PASS**
- `grep -nE external_body phys.{rs,spec.rs,proof.rs}` → only doc/comment mentions, no attribute.
- Module-scoped cheating check: "✅ No cheating detected in module hal::mem::types::address::phys".
- TCB-list rule therefore N/A for this module.

**Item 4 — Zero `assume`/`assume_specification` beyond allowed: PASS**
- Exactly one `assume_specification` in `phys.spec.rs:61`:
  `<::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value` (`ensures result as int == addr@`).
- `sys` is a separate workspace crate; verified that `src/libs/sys/src/sys/mm/address/virt.rs:253`
  `fn into_raw_value` carries **no** `#[verus_spec]` (unverified dependency). Per **verus-constraints**
  this is an allowed external-bottom / not-yet-verified-dependency trust boundary, and the spec file
  documents it (lines 48–66; sibling `VirtualAddress::new` already lost its placeholder because it
  gained a real spec). `assume`/`assume_spec` count from the cheating tool = 0 in-module.

**Item 5 — No cfg-gated exec code: PASS**
- Only `#[cfg(verus_keep_ghost)]` occurrences are on the two `include!` of `phys.spec.rs`/`phys.proof.rs`
  (ghost-only, standard pattern). No cfg on any exec branch, expression, or match arm.

**Item 6 — Cheating audit (exact counts/locations): PASS**
- In `phys.rs/spec.rs/proof.rs`: `admit=0`, `external_body=0`, `assume_specification=1`
  (`phys.spec.rs:61`, allowed — see item 4), cfg-gated exec=0.
- Tool: module run reports `assume=0 external_body=19 admit=14 ... cfg_gate=19` **globally** but
  "No cheating detected in module hal::mem::types::address::phys" — the 14/19/19 are pre-existing
  aggregates from other abstracted kernel modules (identical to the `cdb7b5a1c` baseline
  `admit=14 external_body=19 cfg_gate=19`), not from this module.

**Item 7 — Verus-limitation reproducer: PASS (N/A)**
- The module claims no Verus limitation: no `external_body`, no `// VERUS REWRITE`, no
  assume-based workaround for an unsupported construct. Nothing to reproduce.

**Item 8 — Exec rewrites minimal & semantically equivalent: PASS**
- Single exec change: `from_number` binds `let addr_raw = frame.into_raw_value();` before the
  `proof!{ lemma_from_number_no_overflow(frame); }`, then `addr_raw * mem::FRAME_SIZE`. This is
  semantically identical to the prior `frame.into_raw_value() * mem::FRAME_SIZE` (pure read, no
  reordered side effects). No `// VERUS REWRITE` comment needed; module verifies.

**Item 9 — Cross-module regression: PASS**
- `make verify-kernel` (whole crate, no MODULE): `Exit code : 0`. Global cheating counts unchanged
  vs baseline (no new admit/external_body/cfg introduced). All modules pass.

**Item 10 — Verification + build: PASS**
- `make verify-kernel MODULE=hal::mem::types::address::phys` → `Exit code : 0`, status CLEAN,
  "3/17 exec functions have contracts" (the 3 in-scope verified targets).
- `make verify-kernel` (all) → `Exit code : 0`.
- `./z build -- all-kernel` → `Finished dev profile ... in 10.75s`, `[OK] Build complete.`
  (0 errors, 0 warnings).

### Fix Request
None. Every checklist item is PASS with tool evidence above. No fixes required.
Creating STOP = RESOLVED.
