# Final Comprehensive Review: hal-platform-microvm

Consolidated from two independent sub-agent reviews:
- `final_review.claude.md` (claude-opus-4.8)
- `final_review.codex.md` (gpt-5.3-codex)

Both reviewers performed tool-based, independent investigation and **both
returned PASS** with identical module-scoped counts. In-scope target:
`gva_to_gpa` in `src/kernel/src/hal/platform/microvm/mod.rs`; every other module
function is out of scope.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — script false-negative explained; sole real caller `mm/phys/mod.rs:114` via re-export `crate::hal::platform`
- [x] Caller expectations (success + failure) documented for each pub function — totality/infallibility, determinism, frame correspondence, identity; failure path N/A
- [x] Abstract resource identified — the platform GVA→GPA translation map (identity on MicroVM)
- [x] Pre-existing specs assessed (none existed upstream; spec file was empty)

### View Design
- [x] Every field passes the substitution test (stateless pure function → no struct View; all candidate fields rejected by substitution test)
- [x] All caller-observable state represented (pure map via `spec_gva_to_gpa`)
- [x] No implementation-specific fields (no offset/Map/Platform fields)
- [x] inv() encodes real constraints (N/A — no instance; map properties documented and subsumed by `result == gva`)
- [x] Mathematical types used (`int`; address-as-`usize` exception noted as soft, non-blocking)

### Specification
- [x] Every in-scope exec function has requires/ensures (`gva_to_gpa`, `mod.rs:425-428`; coverage 1/1 in-scope)
- [x] Caller coverage: every caller expectation has a corresponding ensures
- [x] View consistency: spec references `spec_gva_to_gpa` exactly as `view_design.md` prescribes
- [x] No tautological ensures
- [x] No subsumed ensures (determinism/injectivity/frame-stepping documented as derivable, not emitted)
- [x] Error paths have meaningful ensures (N/A — infallible, no error path)
- [x] No assume_specification for workspace-internal code (none used)
- [x] vstd searched before any assume_specification (N/A — none used)
- [x] Specs written for the caller (`result == gva` drops directly into `book_mmio_regions` proof)
- [x] Trait obligations satisfied (none — free function)
- [x] Spec completeness (advisory) assessed — total/deterministic, matches caller expectations
- [x] Loop invariants (N/A — no loops in scope)
- [x] No cheating on module's own functions — admit/assume/external_body/trusted = 0
- [x] No specs weakened (`spec_drift.py` clean)
- [x] Bug awareness — code reconciled, identity is correct; no bugs.md needed
- [x] Cross-module regression — module verify run CLEAN; crate-wide cheating from out-of-scope modules only
- [x] Verification: `make verify-kernel MODULE=hal::platform::microvm` → 0 errors

### Proving
- [x] No specs weakened (`spec_drift.py`: no contract drift)
- [x] Zero remaining admit()
- [x] Zero external_body unless listed in `tcb-allowed.md` — 0 external_body in module
- [x] Zero assume/assume_specification
- [x] No cfg-gated exec code (only standard `#[cfg(verus_keep_ghost)] include!` at `mod.rs:9,11`)
- [x] Cheating audit done with exact counts/locations
- [x] Claimed Verus limitation reproducer (N/A — no limitations claimed; no rewrites)
- [x] Exec rewrites minimal/equivalent (none present; no `// VERUS REWRITE`)
- [x] Cross-module regression evidence present (status CLEAN)
- [x] Verification: 0 errors

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code (only ghost includes)
- [x] Zero external_body (none to confirm against `tcb-allowed.md`)
- [x] AST consistency: zero mismatches
- [x] All exec rewrites have VERUS REWRITE comment + reproducer (N/A — none)
- [x] Each surviving external_body confirmed in `tcb-allowed.md` (N/A — none survive)
- [x] No specs weakened (`spec_drift.py` clean)
- [x] Cross-module regression — module verify CLEAN
- [x] Verification: 0 errors

### Bug Recording
- [x] bugs.md exists if bugs were found (none found → file correctly absent)
- [x] Each bug is a real code defect (N/A — none)
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix (N/A — none)
- [x] No external_body used to mask a code defect (none used)
- [x] Bug entries include provenance (N/A — none)

## Spec Quality
The external-top API contract is correct, complete, minimal, and understandable.

```rust
#[verus_spec(result => ensures result as int == spec_gva_to_gpa(gva as int))]
pub fn gva_to_gpa(gva: usize) -> usize { gva }

pub open spec fn spec_gva_to_gpa(gva: int) -> int { gva }   // identity
```

- Bound to exec code in-place (signature unchanged); only `return gva` satisfies it — bugs (`gva+1`, `0`, any remap) are rejected.
- Declarative (names the platform GVA→GPA map via a single hook), not operational.
- `open` is correct: the caller must derive frame correspondence, which on MicroVM *is* `result == gva`; `closed` would defeat verification.
- No `requires` faithfully records totality/infallibility (caller's unguarded loop).
- Determinism, injectivity, and frame-stepping are correctly documented as corollaries of `result == gva`, not emitted as redundant clauses.

Both reviewers: high-quality spec, no defects.

## Caller Coverage
- Covered: **4 / 4** caller expectations (totality/infallibility, determinism, frame correspondence, identity); injectivity and post-failure correctly handled as subsumed / N/A.
- Missing: **none**.

## Proof Completeness
- Remaining admit(): **0** (module-scoped scan of `microvm/**/*.rs`).
- Remaining external_body not in `tcb-allowed.md`: **0** (no module external_body at all).
- `mod.proof.rs` is `verus! { }` (no proof debt).

## TCB Compliance
- All external_body listed in `tcb-allowed.md`: **YES (vacuously)** — the module contains 0 `external_body`, and `tcb-allowed.md` lists no `microvm` function. No new trust boundary introduced. The crate-wide `external_body=11` in the `make` summary originates entirely from out-of-scope modules (`mm/phys/*`, `arch/x86/mem/paging/*`, `bump_allocator`), all already governed by `tcb-allowed.md`.

## Guardrails Compliance
Module-scoped (`src/kernel/src/hal/platform/microvm/`):
- admit: **0**, assume: **0**, external_body: **0**, assume_specification: **0**, cfg-gated exec: **0**

(Crate-wide totals, out-of-scope, informational: assume=0, external_body=11, admit=27, trusted=0, no_decreases=0, cfg_gate=14.)

## AST Consistency
- AST check: **PASS** — `ast_consistency.py` reports `matched=28 mismatched=0 missing=0 extra=0` (plus 3 structs match); `gva_to_gpa` = MATCH. No `// VERUS REWRITE` comments exist (exec code byte-for-byte semantically unchanged).

## Verification
- verus: **PASS** — `make verify-kernel MODULE=hal::platform::microvm` → `1 verified, 0 errors`, status CLEAN. The single verified function is the in-scope `gva_to_gpa`; coverage `1/31` is expected (30 out-of-scope functions untouched).

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` absent — no defects found).
- True Bugs: **none**. The identity implementation matches the documented MicroVM identity-map contract; no undocumented bugs surfaced during proving/integrity.

## Issues (highest priority first)
1. (Minor / non-blocking, both reviewers) `spec_gva_to_gpa` and the ensures use `int` rather than `usize`. spec-design expresses a soft preference for `usize` on address-typed values; here it is harmless — identity cannot overflow and `result as int == gva as int` cleanly implies `result == gva`. No action required.

No correctness, soundness, coverage, cheating, drift, TCB, or AST issues.

## Result: PASS
Both independent reviews (claude-opus-4.8 and gpt-5.3-codex) concur. The single
in-scope function `gva_to_gpa` is fully verified (0 errors) with a correct,
complete, minimal external-top contract; all module-scoped cheating dimensions
(admit / assume / external_body / assume_specification / cfg-gated exec) are
**0**; TCB-compliant; AST-consistent; no spec drift; no bugs. Every checklist
item passes.
