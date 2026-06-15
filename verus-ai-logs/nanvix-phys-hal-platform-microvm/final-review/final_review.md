# Final Comprehensive Review: hal-platform-microvm

> Consolidated from two independent sub-agent reviews:
> - `final_review.claude.md` (claude-opus-4.8)
> - `final_review.gpt-5.3-codex.md` (gpt-5.3-codex)
>
> Both agents reached **PASS** independently, with identical findings and the
> same single non-blocking advisory note (`nat` vs `usize` address modeling).
> In-scope target (only function in scope): `gva_to_gpa` in
> `src/kernel/src/hal/platform/microvm/mod.rs`.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` output recorded; `gva_to_gpa` has one call site (`mm/phys/mod.rs:128`, `book_mmio_regions`), grep-confirmed.
- [x] Caller expectations (success + failure) documented for each pub function — success path fully documented; failure path is N/A (returns `usize`, not `Result`), explicitly stated.
- [x] Abstract resource identified — MicroVM guest-virtual → guest-physical address translation (identity map).
- [x] Pre-existing specs assessed (if any exist from upstream verification) — none existed; no upstream bias to reconcile.

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite) — zero-field unit View `MicrovmTranslationView`; the map + properties survive any MicroVM-correct rewrite.
- [x] All caller-observable state represented (no missing fields) — pure stateless function; no observable state exists to model.
- [x] No implementation-specific fields (only caller-observable state) — no fields; rejected alternatives (translation table, memory bounds, page-table flag) documented.
- [x] inv() encodes real constraints (not trivially true) — facet is stateless, so `inv()==true` is the honest invariant; well-formedness (totality/determinism/injectivity) is structural to the spec fn, not a field invariant. Justified in view_design.md.
- [x] Mathematical types used (int/Seq/Set/Map; exception: addresses keep usize) — `nat` used for addresses (a mathematical type). See advisory note below; `usize` is the *preferred* convention but `nat` is sound and the binding caller fact `result == gva` is in `usize`.

### Specification
- [x] Every in-scope exec function has requires/ensures (run `fn_coverage.py`) — in-scope coverage 1/1 (harness: "1/31" = only `gva_to_gpa` in scope; other 30 out of scope per hard rules).
- [x] Caller coverage: each caller expectation has corresponding requires/ensures — 5/5 (see Caller Coverage).
- [x] View consistency: specs reference View fields and maintain inv() — `ensures` references `MicrovmTranslationView::spec_gva_to_gpa`; `inv()==true` trivially maintained.
- [x] No tautological ensures (e.g., `Err(_) => true`) — both ensures clauses are substantive equalities.
- [x] No subsumed ensures (derivable from inv() + other ensures) — `result == gva` (usize, caller-usable) and `result as nat == spec_gva_to_gpa(gva as nat)` (View vocabulary) serve distinct purposes; neither is subsumed.
- [x] Error paths have meaningful ensures — N/A: total function, no error path.
- [x] No assume_specification for workspace-internal code — none present.
- [x] vstd searched before any assume_specification — N/A; no assume_specification used.
- [x] Specs written for the caller (usable directly in caller proofs) — `result == gva` is directly usable by `book_mmio_regions`.
- [x] Trait obligations satisfied — none; free function, implements no trait.
- [x] Spec completeness (advisory) — total, deterministic, identity; matches caller expectations exactly. No unintended nondeterminism.
- [x] Loop invariants: every loop has an `invariant` clause — N/A; `gva_to_gpa` and its lemma contain no loops.
- [x] No cheating on module's own functions — in-scope grep: admit=0, assume=0, external_body=0, trusted=0.
- [x] No specs weakened — `spec_drift.py git-diff ... --before HEAD`: 0 contract drift (additive-only diff).
- [x] Bug awareness — no fundamentally incorrect code; no bugs to record.
- [x] Cross-module regression — `make verify-kernel` CLEAN (exit 0); crate-wide cheating counts are pre-existing/out-of-scope (approved TCB).
- [x] Verification — `make verify-kernel MODULE=hal::platform::microvm`: CLEAN, exit 0. Kernel crate compiles under the verification cargo build (`make build` is not a real target in this repo; the verify-kernel cargo build is the authoritative compile and succeeded).

### Proving
- [x] No specs weakened — spec_drift clean (0 drift).
- [x] Zero remaining admit() — 0 in scope (proof lemma body discharges directly from the `open` identity definition; the only "admit" string is in a descriptive comment).
- [x] Zero external_body unless listed in tcb-allowed.md — 0 external_body in scope; `gva_to_gpa` introduces no trust boundary.
- [x] Zero assume/assume_specification — 0 in scope.
- [x] No cfg-gated exec code — 0 in scope (only standard `#[cfg(verus_keep_ghost)] include!` of spec/proof files; `use vstd::prelude::*` un-gated).
- [x] Cheating audit — in-scope: admit=0, external_body=0, assume=0, cfg-gated exec=0.
- [x] Any claimed Verus limitation has an isolated reproducer — none claimed; no rewrites needed.
- [x] Exec rewrites are minimal and semantically equivalent — none performed; exec body unchanged (`gva`). No `// VERUS REWRITE` comments.
- [x] Cross-module regression — `make verify-kernel` CLEAN.
- [x] Verification — `make verify-kernel`: 0 errors, status CLEAN.

### Cheating Elimination
- [x] Zero admit() remaining (in scope).
- [x] Zero assume() remaining (in scope).
- [x] Zero trusted functions (in scope).
- [x] Zero exec_allows_no_decreases_clause.
- [x] Zero cfg-gated exec code (only `include!`/import).
- [x] Zero external_body unless listed in tcb-allowed.md — 0 in scope.
- [x] AST consistency: zero mismatches — `ast_consistency.py`: all 28 functions MATCH.
- [x] All exec rewrites have VERUS REWRITE comment and minimal reproducer — N/A; no rewrites.
- [x] For each surviving external_body: confirm listed in tcb-allowed.md — none in scope.
- [x] No specs weakened — spec_drift clean.
- [x] Cross-module regression — `make verify-kernel` CLEAN.
- [x] Verification — 0 errors.

### Bug Recording
- [x] bugs.md exists if bugs were found — no bugs found; `bugs.md` correctly absent.
- [x] Each bug is a real code defect — N/A (no bugs).
- [x] Each bug entry has required fields — N/A (no bugs).
- [x] No external_body used to mask a code defect — none used.
- [x] Bug entries include provenance — N/A (no bugs).

## Spec Quality
The public API contract on `gva_to_gpa` is **correct, complete, and readable**.
Two `ensures` clauses:
- `result == gva` — the directly-usable identity fact the caller's MMIO frame
  walk relies on (`usize`).
- `result as nat == (MicrovmTranslationView {}).spec_gva_to_gpa(gva as nat)` —
  ties the result into the View vocabulary for downstream proofs.

The unit View (`inv()==true`) is **honest**: `gva_to_gpa` is a pure, stateless,
total free function, so there is no caller-observable state to model. The
algebraic properties callers depend on (totality, determinism, injectivity) are
intrinsic to the total spec fn `spec_gva_to_gpa` and exposed via the
`injective()` predicate + `lemma_translation_injective`. The view_design.md
documents and rejects six over-modeling alternatives.

**Advisory (non-blocking):** addresses are modeled as `nat`, whereas spec-design
and view-design *prefer* `usize` for addresses. This is a minor stylistic
deviation only — `nat` is a sound mathematical type, the `gva as nat` lift is
lossless, and the binding caller fact (`result == gva`) is in `usize`. No
soundness or usability impact; no action required.

## Caller Coverage
- Covered: **5 / 5**
  - Totality / never-panics → total exec fn with unconditional `ensures`.
  - Purity / determinism → spec fn is a function of its argument only.
  - Identity (`result == gva`) → first `ensures` clause.
  - Injectivity (distinct frames) → `injective()` + `lemma_translation_injective`.
  - Valid address encoding → identity over `usize` keeps output in the input's
    representable range (enforced at caller boundary via `from_mmio_address`);
    correctly not over-modeled.
- Missing: **none.** Coverage-of-RAM is explicitly the caller's responsibility
  (`frame::is_covered`) and correctly left out of scope.

## Proof Completeness
- Remaining admit(): **0** (no BLOCKERs).
- Remaining external_body not in tcb-allowed.md: **0** (no BLOCKERs). The
  in-scope code introduces no `external_body` at all.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES** — in-scope code adds zero
  `external_body`, so no new trust boundary is introduced. The crate-wide
  `external_body=25` reported by the harness are pre-existing/out-of-scope sites
  (the pre-approved TCB in `mm/phys`, `bump_allocator`, etc.), none in the
  microvm module.

## Guardrails Compliance
In-scope (microvm `mod.rs` `gva_to_gpa` + `mod.spec.rs` + `mod.proof.rs`):
- admit: **0**, assume: **0**, external_body: **0**, assume_specification: **0**, cfg-gated exec: **0**

(Reference — crate-wide harness counts, pre-existing & out-of-scope: external_body=25, cfg_gate=7, admit=0, assume=0, trusted=0, no_decreases=0.)

## AST Consistency
- AST check: **PASS** — `ast_consistency.py` reports MATCH for all 28 functions;
  no `// VERUS REWRITE` comments; exec body unchanged.

## Verification
- verus: **PASS** — `make verify-kernel MODULE=hal::platform::microvm`: status
  CLEAN, exit 0. `spec_drift.py`: 0 contract drift. Diff vs base
  `verus-ai/hal-memory-region` is additive-only (no deletions of guarantees).

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` correctly absent).
- True Bugs: **0**. `gva_to_gpa` is a trivially-correct identity whose contract
  is proven directly; no caller expectation is violated. No bugs were discovered
  during any phase (caller analysis → view → specification → proving →
  cheating-elimination → polish) and none surfaced in this final review.

## Issues (highest priority first)
1. *(Minor / informational, non-blocking)* Spec models addresses as `nat` rather
   than the spec-design-preferred `usize`. Acceptable: the binding caller fact
   (`result == gva`) is in `usize` and the `gva as nat` lift is exact. No action
   required.

No correctness, completeness, caller-coverage, TCB, AST, guardrails, or
verification issues.

## Result: PASS
