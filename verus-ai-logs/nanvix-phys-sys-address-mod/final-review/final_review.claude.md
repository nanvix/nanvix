# Final Verification Review — `sys-address-mod`

- **Model**: claude-opus-4.8
- **Date**: 2026-06-15
- **Module**: `src/libs/sys/src/sys/mm/address/mod.rs` (`pub trait Address`)
- **In-scope functions**: `from_raw_value`, `into_raw_value`, `is_aligned` (trait
  method *declarations* carrying `#[verus_spec]` external-top contracts; no exec
  bodies in this module — proof file is correctly empty).
- **Verdict**: **PASS** (independent re-run; no blockers)

This is an independent re-investigation. All commands below were executed by me
from the repo root; I did not rely on the submitter's summary.

---

## Spec Quality (per-function assessment, spec-design skill)

Spec helper (mod.spec.rs:8): `spec_addr_is_aligned(v, align) := v %
spec_align_value(align) == 0`. `spec_align_value` is the existing, already-trusted
spec companion of `Alignment` (alignment.rs:155). The helper is referenced by an
in-scope contract → no floating spec.

### `from_raw_value(raw_addr: usize) -> Result<Self, Error>`  (mod.rs:54–61)
```
ensures match result {
    Ok(a)  => a@ == raw_addr as int,
    Err(e) => e.code == ErrorCode::BadAddress,
}
```
- **Ok arm** — round-trip `a@ == raw_addr as int`. Non-tautological, exact, the
  fact every caller round-trip (`into_raw_value`, `PageAligned`/`PhysicalAddress`
  re-wrap) depends on. ✅
- **Err arm** — pins the error *code* to `BadAddress`. This is what the blanket
  `?`-propagation in `PageAligned`/`PageTableAligned` and the kernel tests rely on
  (`Err == BadAddress`). ✅
- **Intentional, justified gap**: the contract does **not** state a *triggering
  condition* for failure (no `requires`, no bidirectional `Err ⇔ raw > max`). This
  is correct, not a defect — `PhysicalAddress::from_raw_value` (kernel phys.rs)
  validates via sparse `is_valid_physical_address` and may reject `raw <= max_addr`,
  so `Err ⇔ raw > spec_max_addr` would be an *untruthful* uniform contract; and
  surfacing `spec_max_addr` would force changes to out-of-scope `max_addr` impls.
  view_design.md §"Specification-phase update" (lines 194–230) documents this. The
  spec-design rule ("dynamic info → keep the `Err` arm, do not turn it into a
  range `requires`") is satisfied. Consequence (noted, not a blocker): the trait
  spec alone does not guarantee that an in-range raw value *succeeds*; that
  positive fact is supplied per-implementor (e.g. `VirtualAddress::from_raw_value`
  is total, virt.rs:181–183). Sound for a trait-level contract.

### `into_raw_value(self) -> usize`  (mod.rs:63–67)
```
ensures result as int == self@
```
Exact, total, lossless projection. Not tautological, no error path (total `usize`
return). Matches `MemoryRegion::new` / round-trip caller needs. ✅

### `is_aligned(&self, align: Alignment) -> Result<bool, Error>`  (mod.rs:135–140)
```
ensures result matches Ok(aligned) && aligned == spec_addr_is_aligned(self@, align)
```
- Uses `matches Ok(aligned)`, which additionally pins that the method **never
  returns `Err`** for any `Alignment` — a *stronger* contract than the bare type
  admits, matching the reality "current impls never error on valid Alignment"
  (caller_analysis lines 108–109). ✅
- The boolean equals the declarative predicate `self@ % spec_align_value == 0`.
  Non-tautological; equivalent to the pre-refactor inline form (see AST/Drift). ✅
- Discharged by `VirtualAddress::is_aligned` (virt.rs:236–238, `Ok(self.is_aligned(..))`),
  part of the 6 verified.

No tautological or subsumed ensures found across the three contracts.

---

## Caller Coverage

Cross-checked every expectation in `caller_analysis.md` (§Caller Expectations,
lines 88–109) and `view_design.md` against the actual contracts. Framed as the
distinct caller-observable properties:

| # | Property (source) | Contract | Status |
|---|-------------------|----------|--------|
| P1 | `from_raw_value` Ok ⇒ `a@ == raw` (round-trip) — caller_analysis:91 | from_raw_value Ok arm | **Covered** |
| P2 | `from_raw_value` Err ⇒ `BadAddress` (used by `?`-propagation) — caller_analysis:92, view_design:218 | from_raw_value Err arm | **Covered** |
| P3 | out-of-range *input* ⇒ Err (negative-direction trigger; tests test.rs:106–177) | — | **Intentionally excluded** (non-uniform/dynamic: PhysicalAddress sparse validity; justified view_design:194–212). Supplied per-implementor, not by trait. |
| P4 | `into_raw_value` lossless `result as int == self@` — caller_analysis:98 | into_raw_value ensures | **Covered** |
| P5 | `is_aligned` Ok(b) ∧ `b == self@%align==0` — caller_analysis:105 | is_aligned ensures | **Covered** |
| P6 | `is_aligned` never spurious `Err` on valid alignment — caller_analysis:108 | is_aligned `matches Ok` | **Covered (stronger)** |

Out-of-band (not a function of the 3 in-scope methods): Ord/Eq agreement with `@`
is provided by the `View<V=int>` + `Ord`/`Eq` supertraits, not by these contracts —
correctly out of scope.

**Covered: 5/6** caller properties pinned by the in-scope contracts. The single
uncovered item (P3) is a deliberate, soundly-justified exclusion (would be an
untruthful uniform contract), not a missing property. No *genuinely* missing
property.

---

## Proof Completeness

- **admit count**: 0 (none in module; whole-crate scan = 0).
- **external_body not in TCB**: 0 — the module introduces **zero** `external_body`.
- `mod.proof.rs` is `verus! { } // verus!` (empty) — correct: the module contains
  only trait method *declarations* with no exec bodies to prove. The proof
  obligations are discharged by the `VirtualAddress` impls in the same crate
  (virt.rs:181/236/253), counted among `6 verified, 0 errors`.

---

## TCB Compliance

- **Compliant: YES.** Zero `external_body` in scope. `tcb-allowed.md` contains no
  entry for `src/libs/sys/src/sys/mm/address/*`, and none is needed.
- No new TCB entries required by this module.

---

## Guardrails Compliance (exact, address module dir)

`grep -rnE 'admit|assume\b|assume_specification|external_body|trusted|exec_allows_no_decreases_clause|#\[cfg(' src/libs/sys/src/sys/mm/address`:

```
admit:0  assume:0  external_body:0  assume_specification:0  cfg-gated-exec:0
```

`make verify-sys` cheating summary (fresh, uncached run 2026-06-15_04-43-24):
`assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0` → **CLEAN**.

cfg attributes found are **benign**, not cheating:
- `mod.rs:9,11` and `virt.rs:9,11` — `#[cfg(verus_keep_ghost)]` guarding the
  `include!("mod.spec.rs")` / `include!("mod.proof.rs")` lines (standard spec/proof
  inclusion idiom).
- `virt.rs:39,296` — `#[cfg(target_pointer_width = "32")]` on a `static_assert!`
  and a `From<VirtualAddress> for u32` impl. These are in the **out-of-scope**
  sibling `virt.rs`, are platform conditionals on real exec items (not
  spec-gating), and are unrelated to the in-scope trait contracts.
- The previously-flagged sole `cfg_gate` item (`alignment.rs:151`, a redundant
  `#[cfg(verus_keep_ghost)]` on a bare `verus!` spec block) was eliminated in the
  cheating-elimination phase; the current crate scan reports `cfg_gate=0`.

---

## AST Consistency

- **PASS.** `ast_consistency.py src/.../address/mod.rs` →
  `✅ All exec functions consistent` / `Consistent: YES` (0/0 functions — a
  trait-declaration-only file has no exec bodies to diff).
- `grep -rn 'VERUS REWRITE' src/.../address/` → **none**. No exec rewrites exist,
  so no semantic-equivalence concern.

---

## Verification

- **PASS.** Fresh, non-cached run (touched `mod.rs`+`mod.spec.rs` to force
  recompilation): `make verify-sys` →
  `verification results:: 6 verified, 0 errors`, **Exit code 0**, status CLEAN.
- Log: `verus-ai-logs/verify-sys/verus-logs/verus_2026-06-15_04-43-24.log`.
- **Error count: 0.**

### Spec Drift
`spec_drift.py git-diff src/.../address/mod.rs --before HEAD` →
`Functions with changes: 0`, `Contract drift: 0`, **"✅ No contract drift
detected."** (working tree == committed HEAD).
Additionally, I diffed against the pre-spec baseline `192f966ee` (caller-analysis
START): the change is **strengthening only** — `from_raw_value` gained its
`ensures`; `is_aligned` was refactored from the inline
`self@ % spec_align_value(align) == 0` to the *definitionally-equal* helper
`spec_addr_is_aligned(self@, align)` (no semantic change); `into_raw_value`
unchanged; one redundant duplicate `use ::vstd::prelude::*;` removed (the
canonical `use vstd::prelude::*;` at mod.rs:8 remains). **No weakening.**

---

## Bug Summary

- Recorded bugs: `specification/bugs.md` = "No fundamentally incorrect code
  found … Status: clean." (The prompt-cited `…/bugs.md` at module root does not
  exist; only the phase file does — non-blocking.)
- **True bugs found in this review: 0.** Per bug-reporting skill, the P3 negative-
  path non-coverage is a *deliberate spec scoping decision* (verus/representation
  limitation across implementors), not a code defect. No unrecorded bugs.

---

## Issues (highest priority first)

1. **(Informational, non-blocking)** `from_raw_value`'s contract does not
   guarantee that a valid/in-range input succeeds (no positive-direction or
   `requires`). This is the intentional, documented consequence of supporting
   sparse `PhysicalAddress` validation (view_design.md:194–212). Callers needing
   "in-range ⇒ Ok" get it from the concrete implementor, not the trait. Acceptable
   and correct for a uniform trait contract; flagged only for transparency.
2. **(Cosmetic)** The prompt-referenced `nanvix-phys-sys-address-mod/bugs.md` is
   actually at `…/specification/bugs.md`. Content is clean; no action needed.

No correctness, security, or guardrail issues.

---

## Result: **PASS**

All checklist items verified with concrete evidence; zero blockers:
admit=0, assume=0, external_body(not-in-TCB)=0, AST mismatches=0, verify-sys
errors=0, no unchecked items, no spec weakening. The three in-scope contracts are
sound, non-tautological, and cover all *uniform* caller expectations; the one
excluded property is justified and non-uniform.
