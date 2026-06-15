## Turn 1: Full Proving Checklist Walkthrough — arch-x86-pde

### Progress
- Done (PASS): all 10 checklist items
- Current: full checklist sweep with tool evidence
- Remaining: none

### Verification

Scope files:
- `src/libs/arch/src/x86/mem/paging/pde.rs`
- `src/libs/arch/src/x86/mem/paging/pde.spec.rs`
- `src/libs/arch/src/x86/mem/paging/pde.proof.rs`

Commands run:
- `make verify-arch` → exit 0. Result: **48 verified, 0 errors** (see
  `verus-ai-logs/verify-arch/verus-logs/verus_2026-06-15_14-12-13.log`).
  Cargo build line `Finished dev profile` with no warnings.
- Cheating summary: `assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=4`
  (crate-wide). Detail (`cheating-detail.txt`):
  - `x86/mem/paging/mod.rs:80 invlpg: external_body` — TCB-allowed
  - `x86/mem/paging/table.rs:209 read: external_body` — TCB-allowed
  - `x86/mem/paging/table.rs:246 write: external_body` — TCB-allowed
  - `x86/mem/paging/table.proof.rs:16 lemma_entry_roundtrip: assume` — TCB-listed
  None are in pde files.
- `python3 scripts/spec_drift.py check arch-x86-pde` → exit 0, **0 contract drift**
  (baseline `5e2a624da810` = proving phase start → HEAD).
- `grep -nE "admit|external_body|assume|assume_specification"` over the three pde
  files → **no matches**.
- `grep -n "VERUS REWRITE"` over pde files → **no matches**.
- `git diff` over the paging dir → pde exec bodies unchanged vs baseline.

### Item-by-item result

1. **No specs weakened** — PASS. spec_drift.py: 0 ensures removed, 0 requires added,
   0 functions removed. Exit 0.
2. **Zero admit()** — PASS. None in pde files; cheating admit=0.
3. **Zero external_body unless TCB-allowed** — PASS. Zero in pde files. The 3
   crate-wide external_body are in `mod.rs::invlpg`, `table.rs::read`,
   `table.rs::write`, all explicitly listed in `verus-ai-logs/tcb-allowed.md`.
4. **Zero assume/assume_specification** — PASS. None in pde files. The single
   `assume` is `table.proof.rs::lemma_entry_roundtrip` (out of pde scope, TCB-listed).
5. **No cfg-gated exec code** — PASS. The only `cfg` uses in pde.rs are
   `#[cfg(verus_keep_ghost)]` on `include!("pde.spec.rs")` / `include!("pde.proof.rs")`
   (lines 9, 11) and `#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]`
   (lines 83, 307). These are the standard Verus ghost-include guards and lint
   markers — no cfg-gated branches, expressions, or match arms in exec code.
6. **Cheating audit (exact counts/locations)** — PASS. pde module:
   admit=0, external_body=0, assume=0, cfg-gated exec=0. Locations: none.
7. **Claimed Verus limitation → isolated reproducer** — PASS (N/A). No limitation
   claimed: pde has no admit/assume/external_body and bugs.md records none.
8. **Exec rewrites minimal & semantically equivalent (VERUS REWRITE)** — PASS.
   No `// VERUS REWRITE` comments; pde exec bodies are unchanged from baseline
   (git diff empty for the dir). Only spec/proof companion files added verification.
9. **Cross-module regression (verify all)** — PASS. `make verify-arch` verifies the
   entire `arch` crate: 48 verified, 0 errors. No regressions in sibling modules
   (frame, table, mod, pte).
10. **Verification + build, 0 errors / 0 warnings** — PASS. 48 verified, 0 errors;
    cargo build finished with no warnings; exit code 0.

### Fix Request
None. All checklist items PASS with tool evidence. No code changes required.
