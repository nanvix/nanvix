# Final Comprehensive Review: hal-platform-microvm

Consolidated from two independent sub-agent reviews (raw reports retained
alongside this file):

- `final_review.claude.md` — model `claude-opus-4.8`
- `final_review.gpt-5.3-codex.md` — model `gpt-5.3-codex`

Plus orchestrator-run tooling (`make verify-kernel`, `make verify`,
`ast_consistency.py`, `spec_drift.py`, `fn_coverage.py`). Both reviewers and the
orchestrator independently agree on every item below.

In scope: the single free function `gva_to_gpa(gva: usize) -> usize` in
`src/kernel/src/hal/platform/microvm/mod.rs` (the MicroVM identity GVA→GPA
translation). All other module items are out of scope and were left untouched.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` + repo-wide search; sole caller `mm/phys/mod.rs:114`
- [x] Caller expectations (success + failure) documented for each pub function — success documented; failure is N/A (total/infallible, no `Result`)
- [x] Abstract resource identified — the platform GVA→GPA translation map (identity on MicroVM)
- [x] Pre-existing specs assessed (if any exist from upstream verification) — none existed (empty `mod.spec.rs`); assessed in caller_analysis.md

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite) — no struct; substitution-test table rejects all candidate fields
- [x] All caller-observable state represented (no missing fields) — pure stateless function; the only observable is the result, named by `spec_gva_to_gpa`
- [x] No implementation-specific fields (only caller-observable state) — no fields at all
- [x] inv() encodes real constraints (not trivially true) — N/A (no instance/state); map properties pinned to the exec contract, not free-standing
- [x] Mathematical types used (int/Seq/Set/Map; exception: addresses keep usize) — `int` domain used (advisory note below re: usize)
- [x] No assume_specification for workspace-internal code — none present

### Specification
- [x] Every in-scope exec function has requires/ensures (`fn_coverage.py`) — `gva_to_gpa` carries `ensures`; coverage 28/28 matched
- [x] Caller coverage: each caller expectation has corresponding requires/ensures — totality, determinism, frame correspondence, identity all captured/derivable from `result == gva`
- [x] View consistency: specs reference View fields and maintain inv() — ensures references `spec_gva_to_gpa` (the View map)
- [x] No tautological ensures (e.g., `Err(_) => true`) — `result as int == spec_gva_to_gpa(gva as int)` is falsifiable
- [x] No subsumed ensures (derivable from inv() + other ensures) — single non-redundant clause
- [x] Error paths have meaningful ensures — N/A (infallible `usize` return, no error path)
- [x] No assume_specification for workspace-internal code — none
- [x] vstd searched before any assume_specification — N/A (none introduced)
- [x] Specs written for the caller (usable directly in caller proofs) — `result == gva` drops directly into `book_mmio_regions`
- [x] Trait obligations satisfied — N/A (free function, no trait)
- [x] Spec completeness (advisory): intentional determinism matches caller expectations
- [x] Loop invariants: every loop has an `invariant` clause — N/A (no loops in `gva_to_gpa`)
- [x] No cheating on module's own functions — admit=0, assume=0, external_body=0, trusted=0 in microvm files
- [x] No specs weakened: `spec_drift.py` — 0 functions changed, 0 ensures removed, 0 requires added
- [x] Bug awareness: no fundamentally incorrect code — identity function, no defect
- [x] Cross-module regression: `make verify` — exit 0 (all crates + kernel)
- [x] Verification: `make verify-kernel` (exit 0) and build (compiles clean)

### Proving
- [x] No specs weakened — `spec_drift.py` clean
- [x] Zero remaining admit() — 0 in microvm files
- [x] Zero external_body unless listed in tcb-allowed.md — 0 external_body in microvm
- [x] Zero assume/assume_specification — 0 in microvm
- [x] No cfg-gated exec code (in-scope) — `gva_to_gpa` has only `#[verus_spec]` + `#[inline(always)]`; no cfg
- [x] Cheating audit: counts reported (see Guardrails) — all zero in scope
- [x] Any claimed Verus limitation has an isolated reproducer — N/A (no limitations claimed; no rewrites)
- [x] Exec rewrites minimal and semantically equivalent — 0 `// VERUS REWRITE` comments; body byte-identical to HEAD
- [x] Cross-module regression: `make verify` — exit 0
- [x] Verification: `make verify-kernel` / build — exit 0

### Cheating Elimination
- [x] Zero admit() remaining (in scope)
- [x] Zero assume() remaining (in scope)
- [x] Zero trusted functions (in scope)
- [x] Zero exec_allows_no_decreases_clause (in scope)
- [x] Zero cfg-gated exec code (in-scope function) — only the standard `cfg(verus_keep_ghost)` spec/proof includes module-wide
- [x] Zero external_body unless listed in tcb-allowed.md — 0 in microvm (vacuously compliant)
- [x] AST consistency: zero mismatches — `ast_consistency.py` 28/28 functions, 0 mismatch, exit 0
- [x] All exec rewrites have VERUS REWRITE comment and minimal reproducer — N/A (0 rewrites)
- [x] For each surviving external_body: confirmed listed in tcb-allowed.md — none in scope
- [x] No specs weakened — `spec_drift.py` clean
- [x] Cross-module regression: `make verify` — exit 0
- [x] Verification: `make verify-kernel` / build — exit 0

### Bug Recording
- [x] bugs.md exists if bugs were found — no bugs found, no file needed (correct: identity function)
- [x] Each bug entry is a real code defect — N/A (no bugs)
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix — N/A
- [x] No external_body used to mask a code defect — none in scope
- [x] Bug entries include provenance — N/A

## Spec Quality

The external-top contract is correct, minimal, declarative, and caller-usable:

```rust
pub open spec fn spec_gva_to_gpa(gva: int) -> int { gva }   // mod.spec.rs:14

#[verus_spec(result => ensures result as int == spec_gva_to_gpa(gva as int))]
pub fn gva_to_gpa(gva: usize) -> usize { gva }              // mod.rs:425-432
```

- **Correct**: `result == gva` is exactly the MicroVM platform contract
  (guest runs identity-mapped; VMM maps GVA == GPA).
- **Non-tautological**: the ensures is falsifiable — it rejects any non-identity
  reimplementation (e.g. an offset remap), so it has real proof power.
- **Non-subsumed / minimal**: a single clause; determinism, injectivity, and
  per-frame stepping are corollaries of `result == gva` and correctly omitted as
  separate clauses.
- **`open` justified**: the caller (`book_mmio_regions`) must derive frame
  correspondence, which on MicroVM *is* `result == gva`; the identity is a
  platform contract, not a hidden implementation choice, so exposing the body is
  correct rather than a leak.
- **Indirection via `spec_gva_to_gpa`**: names the platform map (WHAT) rather
  than restating the body (HOW), giving future non-identity platforms a single
  hook to redefine.

Advisory (non-blocking, both reviewers): `spec_gva_to_gpa` uses `int` for an
address where the spec-design "addresses keep usize" guidance would prefer
`usize`. Harmless for the identity map (no arithmetic ⇒ no overflow/negativity);
the checklist item "mathematical types used" is satisfied.

## Caller Coverage
- Covered: 1 / 1 caller expectations (all properties captured or derivable)
- Missing: none. Totality/infallibility (no `requires`, direct `usize` return),
  determinism (function of `gva` alone), frame correspondence + identity
  (`result == gva`) all follow from the single ensures. The sole caller
  `book_mmio_regions` (`mm/phys/mod.rs:114`) can discharge "the booked frame
  backs the MMIO GVA" directly.

## Proof Completeness
- Remaining admit(): **0** in microvm module files (BLOCKER count = 0)
- Remaining external_body not in tcb-allowed.md: **0** in microvm module files
  (BLOCKER count = 0)

(Kernel-wide `make verify-kernel` aggregates `admit=12`, `external_body=19` from
`mm/phys` and `mm/virt` — all out of scope for this effort; every one of those
external_body is recorded in `tcb-allowed.md`, and those admits belong to other
modules' own verification pipelines. Confirmed via
`verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`: zero entries under
`hal/platform/microvm`.)

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES** — the microvm module
  introduces zero external_body, so the requirement is satisfied vacuously. No
  new trust boundary was introduced.

## Guardrails Compliance
Scoped to microvm module files (`mod.rs`, `mod.spec.rs`, `mod.proof.rs`):
- admit: **0**, assume: **0** (the 6 textual "assumes" are prose in doc
  comments, not escapes), external_body: **0**, assume_specification: **0**,
  cfg-gated exec on `gva_to_gpa`: **0**.

## AST Consistency
- AST check: **PASS** — `ast_consistency.py` reports 28/28 functions MATCH,
  0 mismatched, 0 missing, 0 extra, exit 0. Zero `// VERUS REWRITE` comments;
  `gva_to_gpa` body is byte-identical to the pre-verification source.

## Verification
- verus: **PASS** — `make verify-kernel` exit 0 (0 errors); `make verify`
  (cross-module regression, all crates + kernel) exit 0; build compiles clean.

## Bug Summary
- Total bugs recorded: 0 (no `bugs.md`, which is correct)
- True Bugs: 0 — `gva_to_gpa` is the identity function with no logic, safety, or
  behavioral defect. No bugs were discovered during proving or cheating
  elimination.

## Issues (highest priority first)
- **Blockers: none.**
- (Advisory, non-blocking) `spec_gva_to_gpa` domain is `int`; spec-design
  prefers `usize` for addresses. No action required for the identity map (no
  arithmetic); revisit only if a future MicroVM translation introduces address
  arithmetic.
- (Informational) The referenced skill docs (spec-design, verus-constraints,
  etc.) were not found under `.github/skills` in this checkout; review applied
  the criteria from memory/instructions. Does not affect the verdict.

## Result: PASS
