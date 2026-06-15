# Final Comprehensive Review: arch-frame-number

**Module:** `arch::x86::mem::paging::frame::number`
**Date:** 2026-06-15 · **Branch:** `verus-ai-prove`
**Reviewers:** 2 independent sub-agents — `claude-opus-4.8` (PASS) and `gpt-5.3-codex` (FAIL only on out-of-scope crate-level cheating).
Raw reviews: `final_review.claude.md`, `final_review.codex.md`.
In-scope items ONLY: type `FrameNumber` (`view`/`inv`/`spec_max`), `FrameNumber::from_raw_value`, `FrameNumber::into_raw_value`. `NULL`/`MAX` consts and unit tests are out of scope (confirmed untouched — AST MATCH, NO_DIFF vs base).

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` output in caller_analysis.md (from_raw_value: 2 ext callers; into_raw_value: 4 ext callers; type: 12 refs)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified — validated physical page-frame index in `0..=MAX`
- [x] Pre-existing specs assessed — upstream `number.spec.rs`/`.proof.rs` were empty; kernel placeholder `assume_specification`s superseded by native specs

### View Design
- [x] Every field passes the substitution test — sole field `self@ : int` survives reimplementation
- [x] All caller-observable state represented — the single frame index
- [x] No implementation-specific fields — `usize` newtype hidden behind `closed view`
- [x] inv() encodes real constraints — `0 <= self@ <= spec_max()` (non-trivial range bound)
- [x] Mathematical types used — View is `int`; bound `spec_max()` is `nat`

### Specification
- [x] Every in-scope exec function has requires/ensures — both pub fns annotated (`fn_coverage`/verify-arch coverage confirms)
- [x] Caller coverage — 7/7 caller expectations map to requires/ensures (both reviewers)
- [x] View consistency — specs reference `self@`/`spec_max()` and maintain `inv()`
- [x] No tautological ensures — both arms of `from_raw_value` are definite (`Some`/`None`)
- [x] No subsumed ensures — `into_raw_value`'s bound clause is a *required* surfacing of `inv()` (type invariant of a consumed `self` is not auto-available to callers), not redundant
- [x] Error paths have meaningful ensures — `value > spec_max() ==> result is None`
- [x] No assume_specification for workspace-internal code — 0 present in-scope
- [x] vstd searched before any assume_specification — n/a (none used)
- [x] Specs written for the caller — round-trip + overflow-safe bound directly usable by PTE/PDE
- [x] Trait obligations satisfied — none (only `Debug/Clone/Copy` derived)
- [x] Spec completeness (advisory) — total, deterministic; no nondeterminism
- [x] Loop invariants — n/a (no loops)
- [x] No cheating on module's own functions — admit/assume/external_body/trusted = 0 in-scope
- [x] No specs weakened — `spec_drift.py`: ✅ No contract drift detected
- [x] Bug awareness — no defect found; bugs.md correctly absent
- [x] Cross-module regression — `make verify-arch` exit 0 (see note on crate-level cheating)
- [x] Verification — `make verify-arch` exit 0, 0 errors; `make build` no-op (build folds into verify compile)

### Proving
- [x] No specs weakened — spec_drift clean
- [x] Zero remaining admit() — 0 in-scope
- [x] Zero external_body unless listed in tcb-allowed.md — 0 in-scope external_body
- [x] Zero assume/assume_specification — 0 in-scope
- [x] No cfg-gated exec code — only `#[cfg(verus_keep_ghost)]` ghost `include!` guards (allowed)
- [x] Cheating audit — admit=0, external_body=0, assume=0, cfg-gated exec=0 in-scope
- [x] Any claimed Verus limitation has isolated reproducer — n/a (no rewrites/limitations claimed)
- [x] Exec rewrites minimal & equivalent — none present (no `// VERUS REWRITE`)
- [x] Cross-module regression — verify-arch exit 0
- [x] Verification — verify-arch 0 errors; build clean

### Cheating Elimination
- [x] Zero admit() remaining (in-scope)
- [x] Zero assume() remaining (in-scope)
- [x] Zero trusted functions (in-scope)
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code (only ghost include! guards)
- [x] Zero external_body unless listed in tcb-allowed.md — 0 in-scope
- [x] AST consistency — `ast_consistency.py`: Consistent ✅ (matched=4, mismatched=0); struct `FrameNumber` MATCH
- [x] All exec rewrites have VERUS REWRITE comment + reproducer — n/a (none)
- [x] Each surviving external_body listed in tcb-allowed.md — n/a (none in-scope)
- [x] No specs weakened — spec_drift clean
- [x] Cross-module regression — verify-arch exit 0
- [x] Verification — 0 errors

### Bug Recording
- [x] bugs.md exists if bugs were found — none found, file correctly absent
- [x] Each bug is a real code defect — n/a (no bugs)
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix — n/a
- [x] No external_body used to mask a code defect — none in-scope
- [x] Bug entries include provenance — n/a

## Spec Quality
Public API specs are correct, complete, and declarative.
- `from_raw_value`: bidirectional, exhaustive partition on `value <= spec_max()` → `Some` (value-preserving, `(result->Some_0)@ == value as int`) / `None`. No tautology, no one-sided error path.
- `into_raw_value`: total value-preserving projection (`result as int == self@`) plus the in-range bound `0 <= self@ <= spec_max()` — the overflow-safety guarantee every PTE/PDE caller relies on for `result << FRAME_SHIFT`. Surfacing the invariant in `ensures` is required (not subsumed), since a consumed `self`'s type invariant is not auto-available to callers. The ghost `use_type_invariant(self)` is the correct idiom to discharge it.
- View `closed view(&self) -> int { self.0 as int }` + `#[verifier::type_invariant] inv()` are minimal and caller-facing.
- **Design divergence (non-blocking):** the shipped `pub open spec fn spec_max()` (interpreted) is *sounder/stronger* than view_design.md's stale `uninterp spec_max` + `assume_specification[FrameNumber::MAX]` plan — it discharges the `MAX` binding by proof instead of adding a trust boundary (and `uninterp` would itself have tripped the guardrails). view_design.md is stale and should be updated.

## Caller Coverage
- Covered: **7 / 7**
- Missing: none.
  (1) `from_raw_value` Some-iff-in-range; (2) clean `None` propagation/no truncation; (3) round-trip identity; (4) `into_raw_value` exact stored value; (5) bound for overflow-safe `<< FRAME_SHIFT`; (6) `into_raw_value` totality; (7) `FrameNumber` always-valid token via type invariant.

## Proof Completeness
- Remaining admit(): **0** in-scope (proof file is empty `verus! { }`).
- Remaining external_body not in tcb-allowed.md: **0** in-scope.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES (vacuous)** — the in-scope module introduces no `external_body`.

## Guardrails Compliance (in-scope module)
- admit: **0**, assume: **0**, external_body: **0**, assume_specification: **0**, cfg-gated exec: **0**
  (The only `cfg` in `number.rs` lines 9/11 gate the ghost `include!("number.spec.rs"/".proof.rs")`, which is allowed and not exec code.)

## AST Consistency
- AST check: **PASS** — matched=4, mismatched=0, missing=0, extra=0; struct `FrameNumber` MATCH. No `// VERUS REWRITE` comments to audit. Only Verus-side addition is the ghost `proof! { use_type_invariant(self); }`.

## Verification
- verus (`make verify-arch`): **PASS** — exit 0, 0 verifier errors, build compiles (`make build` is a no-op; compilation is performed by the verify step).
- **Out-of-scope note (not a blocker for this module):** the crate-level wrapper reports `CHEATING_DETECTED` with `admit=1 external_body=3 cfg_gate=4`. Every one of these is OUTSIDE the in-scope module (`cheating-detail.txt`):
  - `x86/mem/paging/mod.rs:80 invlpg: external_body`
  - `x86/mem/paging/table.proof.rs:8 lemma_entry_roundtrip: admit`
  - `x86/mem/paging/table.rs:209 read: external_body` (listed in tcb-allowed.md)
  - `x86/mem/paging/table.rs:246 write: external_body` (listed in tcb-allowed.md)
  These belong to the separately-tracked `paging-table`/paging modules and are explicitly out of scope per the hard rule "do not touch unlisted functions". The `arch-frame-number` module itself contributes **zero** cheating. Codex's `RESULT: FAIL` is attributable solely to this crate-level aggregate, not to any in-scope defect.

## Bug Summary
- Total bugs recorded: **0** (bugs.md correctly absent)
- True Bugs: **0** — independent review of `view`/`inv`/`from_raw_value`/`into_raw_value` found no logic error, safety violation, or incorrect behavior.

## Issues (highest priority first)
1. **(Non-blocking, doc hygiene)** `view_design.md` is stale: it documents an `uninterp spec_max` + `assume_specification[FrameNumber::MAX]` design that was superseded by the sounder shipped `open spec_max()`. Update the doc to match the final, stronger design.
2. **(Informational)** Crate-level `make verify-arch` reports `CHEATING_DETECTED` from out-of-scope `paging/table` + `invlpg`. Not attributable to this module; tracked by those modules' own reviews. The `table.proof.rs` admit and `invlpg` external_body are NOT in tcb-allowed.md and must be resolved by their owning module efforts (flagged here for cross-module awareness only).

## Result: PASS

All checklist items for the in-scope `arch-frame-number` module are satisfied: 7/7 caller coverage, zero in-scope cheating across every dimension (admit/assume/external_body/assume_specification/cfg-gated-exec), AST consistent, no spec drift, no bugs, and `make verify-arch` at 0 errors. The lone `FAIL` from the codex reviewer reflects a crate-wide aggregate of pre-existing, out-of-scope cheating in other paging modules, not any deficiency in this module. Verdict for **arch-frame-number: PASS**.
