## Turn 1: Full proving-phase checklist walkthrough (arch-x86-pte)

### Progress
- Done (all PASS): spec-drift, admit, external_body, assume/assume_specification,
  cfg-gated exec code, cheating audit, Verus-limitation reproducers, exec rewrites,
  cross-module regression, verification+build.
- Current: final verdict.
- Remaining: none.

### Scope
Module `x86::mem::paging::pte`. In-scope functions: `PageTableEntry::new`,
`PageTableEntryFlags::new`, `PageTableEntry::is_present`, `PageTableEntryFlags::is_present`.
Files: `pte.rs`, `pte.spec.rs`, `pte.proof.rs`.

### Verification (commands run + evidence)

`make verify-arch` (full crate, no MODULE) on the restored working tree:
```
note: verifying module x86::mem::paging::pte
verification results:: 48 verified, 0 errors
Exit code : 0
cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=2
```
Latest verus log: `verus_2026-06-15_14-57-21.log` — `grep -ic warning` = **0**.

Cheating detail (`verus-logs/cheating-detail.txt`):
```
x86/mem/paging/mod.rs:80        invlpg: external_body
x86/mem/paging/table.proof.rs:16 lemma_entry_roundtrip: assume (broadcast external_body axiom)
x86/mem/paging/table.rs:209     read: external_body
x86/mem/paging/table.rs:246     write: external_body
```
All four are in **other** modules (`mod`, `table`), **none** in `pte`, and each is
explicitly enumerated in `verus-ai-logs/tcb-allowed.md`.

cfg_gate=2 enumerated (custom scan reproducing `scripts/verify.sh::count_cfg_gates`):
```
pte.rs:85   #[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]  (PageTableEntryFlags::new)
pte.rs:307  #[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]  (PageTableEntry::new)
```

Per-item verdicts:

1. **No specs weakened — PASS.** Proving made **zero** source edits to the pte
   module: `git diff ae732a893(proving START) 817a79dce(PASS) -- pte*.rs` is empty.
   Current contracts are byte-identical to the specification-phase-approved ones
   (`spec_pte_flags_new`, `spec_pte_new`, `inv`, the `is_present` ensures, and
   `PageTableEntry::new`'s `result@ == spec_pte_new(..) && result.inv()`). No
   weakening is possible when nothing changed.

2. **Zero admit() — PASS.** `grep admit` in pte.rs/spec/proof = 0; crate `admit=0`.

3. **Zero external_body unless TCB-allowed — PASS.** pte module: 0 external_body.
   Crate external_body=3 = `invlpg`, `table::read`, `table::write` — each
   individually listed in `tcb-allowed.md`. No pte-scope violation.

4. **Zero assume/assume_specification — PASS.** pte module: 0. Scanner `assume=0`.
   `lemma_entry_roundtrip` is a TCB-listed broadcast `external_body` axiom (table
   module), not an in-scope assume; no `assume_specification` in pte.

5. **No cfg-gated exec code — PASS.** The two cfg_gate hits are
   `#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]` — lint
   *allow* attributes, not branches/expressions/match arms. Exec body is identical
   under any cfg. Proven **required**: removing both lines reproduces commit-HEAD
   `426e3fea1` and fails compilation with `error[E0425]: cannot find function 'new'`
   (the `verus_spec` macro needs the marker allow). They gate a verus-only lint so
   the normal build does not choke — necessary tooling attributes, not cheating.

6. **Cheating audit (exact counts + locations) — PASS.**
   - pte module: admit=0, external_body=0, assume=0, cfg-gated-exec=0.
   - crate-wide: admit=0; external_body=3 (mod.rs:80 invlpg, table.rs:209 read,
     table.rs:246 write — all TCB-listed); assume=0; cfg_gate=2 (pte.rs:85, pte.rs:307,
     required lint-allow cfg_attr). `CHEATING_DETECTED` status is driven solely by
     these TCB-approved external_bodies + the required cfg_attr — no in-scope cheat.

7. **Verus-limitation reproducers — PASS / N/A.** Proving introduced no new limitation
   claims (no source change). `bugs.md` reports "None"; no `verus-unsupported.md` for
   this module. Pre-existing trust boundaries (table read/write, invlpg) carry their
   own documented reproducers referenced from `tcb-allowed.md`.

8. **Exec rewrites minimal & equivalent — PASS / N/A.** No `// VERUS REWRITE` comments
   in pte files; no exec edits during proving.

9. **Cross-module regression — PASS.** Full-crate `make verify-arch`: **48 verified,
   0 errors** (all arch modules, not just pte).

10. **Verification + build, 0 errors/0 warnings — PASS.** verify: 48 verified, 0 errors,
    exit 0, 0 warnings in the verus log. The verus run is a full cargo+rustc compile of
    the arch crate (`Finished dev profile`), so the build dimension is covered with 0
    warnings.

### Note on repo state
A transient FAIL commit `426e3fea1` exists in history: it captured my own removal
experiment for item 5 (the harness auto-commits each `make verify-arch`). The working
tree was restored to the known-good source (identical to `817a79dce`) and re-verified,
producing PASS commit `9c2fffa28` (48 verified, 0 errors). Working tree is clean apart
from dialogue logs.

### Fix Request
None. All ten checklist items PASS with concrete tool evidence.

### Verdict
RESOLVED.
