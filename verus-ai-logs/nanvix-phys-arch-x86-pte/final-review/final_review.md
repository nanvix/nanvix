# Final Comprehensive Review: arch-x86-pte

Consolidated from two independent sub-agent reviews (models: `claude-opus-4.8`,
`gpt-5.3-codex`). Raw reviews: `final_review.claude.md`, `final_review.codex.md`.
Both reviewers independently re-derived every count with tools and both returned
**PASS** with zero issues.

Module: `src/libs/arch/src/x86/mem/paging/pte.rs` (+ `pte.spec.rs`, `pte.proof.rs`)
In-scope functions: `PageTableEntry::new`, `PageTableEntryFlags::new`,
`PageTableEntry::is_present`, `PageTableEntryFlags::is_present`.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` run (documented LSP false-negative for the `kernel` crate; real call sites enumerated)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified (single x86 PTE = `(flags, frame)` + flag-set sub-component)
- [x] Pre-existing specs assessed (upstream `assume_specification`/`Ex*` placeholders in `identity_map.spec.rs`, assessed as partial+weak, now removed)

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite) — documented per-field in `view_design.md`
- [x] All caller-observable state represented (8 flag bits + frame index)
- [x] No implementation-specific fields (no `PteWord`/raw layout; `view()` is `closed`)
- [x] inv() encodes real constraints (`PageTableEntry::inv = 0 <= frame <= FrameNumber::spec_max()`; `PageTableEntryFlags::inv = true` justified — no cross-field constraint, documented in Rejected Alternatives)
- [x] Mathematical types used (`bool` fields, `frame: int`; address arithmetic stays exec-side)

### Specification
- [x] Every in-scope exec function has requires/ensures (all 4 carry `#[verus_spec]` ensures)
- [x] Caller coverage: each caller expectation has corresponding requires/ensures
- [x] View consistency: specs reference View fields and maintain inv()
- [x] No tautological ensures
- [x] No subsumed ensures
- [x] Error paths have meaningful ensures (N/A — both constructors total; queries are pure `bool`)
- [x] No assume_specification for workspace-internal code (0 in module)
- [x] vstd searched before any assume_specification (none used)
- [x] Specs written for the caller (view-level equalities usable directly in caller proofs)
- [x] Trait obligations satisfied (`TableEntry` round-trip is an out-of-scope boundary obligation; in-scope `new` pins `result@`)
- [x] Spec completeness (advisory): constructor fidelity + presence delegation match caller expectations; `cow` default is intentional/required
- [x] Loop invariants: N/A (no loops in in-scope functions)
- [x] No cheating on module's own functions: `admit=0 assume=0 external_body=0 trusted=0`
- [x] No specs weakened: `spec_drift.py` → 0 contract drift (specs strictly added)
- [x] Bug awareness: `bugs.md` = "None" (reconciles with clean verification)
- [x] Cross-module regression: `make verify` exit 0 (all verified modules pass)
- [x] Verification: `make verify-arch` exit 0, `./z build -- all` PASS

### Proving
- [x] No specs weakened (`spec_drift.py` → 0 drift)
- [x] Zero remaining admit()
- [x] Zero external_body (none introduced; `tcb-allowed.md` correctly has no pte entry)
- [x] Zero assume/assume_specification
- [x] No cfg-gated exec code (only standard `cfg(verus_keep_ghost)` ghost-include + marker attrs)
- [x] Cheating audit: admit=0, external_body=0, assume=0, cfg-gated exec=0
- [x] Any claimed Verus limitation has an isolated reproducer (N/A — no rewrites/limitations claimed)
- [x] Exec rewrites minimal and semantically equivalent (no `// VERUS REWRITE` present)
- [x] Cross-module regression: `make verify` exit 0
- [x] Verification: `make verify-arch` 0 errors; `./z build -- all` PASS

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code
- [x] Zero external_body (none; nothing to list)
- [x] AST consistency: zero mismatches (`ast_consistency.py` → 23/23 MATCH, Consistent: YES)
- [x] All exec rewrites have VERUS REWRITE comment + reproducer (N/A — none)
- [x] For each surviving external_body: listed in `tcb-allowed.md` (N/A — none in pte)
- [x] No specs weakened (`spec_drift.py` → 0 drift)
- [x] Cross-module regression: `make verify` exit 0
- [x] Verification: `make verify-arch` 0 errors; `./z build -- all` PASS

### Bug Recording
- [x] bugs.md exists and states "None" (no bugs found)
- [x] Each bug is a real code defect (N/A — none)
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix (N/A — none)
- [x] No external_body used to mask a code defect (no external_body at all)
- [x] Bug entries include provenance (N/A — none)

## Spec Quality
The four external-top API contracts are correct, complete, and declarative:
- `PageTableEntryFlags::new` → `result@ == spec_pte_flags_new(...)` pins all seven
  argument bits and the `cow == false` default; total constructor (no `requires`).
- `PageTableEntry::new` → `result@ == spec_pte_new(flags@, frame@)` (constructor
  fidelity) **and** `result.inv()` (frame bound), discharged in-body from the
  `FrameNumber` type invariant via `use_type_invariant(frame)`.
- `PageTableEntry::is_present` → `result == self@.flags.present` (presence delegation).
- `PageTableEntryFlags::is_present` → `result == self@.present` (pure projection).

Views (`PteView`, `PteFlagsView`) are `closed`, expose only caller-observable state
(8 `bool` flags + `frame: int`), pass the substitution test, and use mathematical
types. `PageTableEntry::inv()` is a real non-vacuous frame bound;
`PageTableEntryFlags::inv() == true` is justified (no architectural cross-bit
constraint; all 2⁸ combinations are legal) and documented with rejected alternatives.
Both reviewers: **PASS**.

## Caller Coverage
- Covered: **4 / 4 in-scope functions** (codex enumerated the same coverage at the
  finer per-expectation granularity as **6 / 6**). Every documented caller expectation
  maps to a `requires`/`ensures` clause.
- Missing: **None**. (The `TableEntry` raw round-trip is an explicitly out-of-scope
  boundary obligation routed through non-in-scope accessors.)

## Proof Completeness
- Remaining admit(): **0** — none. (`pte.proof.rs` is empty; the only proof block is a
  sound `use_type_invariant(frame)` in `new`.)
- Remaining external_body not in tcb-allowed.md: **0** — none (pte introduces zero
  `external_body`).

## TCB Compliance
- All external_body listed in `tcb-allowed.md`: **YES (vacuously)** — the pte module
  introduces no `external_body`. The only `arch`-crate boundaries
  (`mod.rs::invlpg`, `table.rs::read`, `table.rs::write`) are out of pte scope and
  already TCB-listed. No new trust boundary added.

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **0**, assume_specification: **0**,
  cfg-gated exec: **0**.
- (The two `#[cfg(verus_keep_ghost)] include!(...)` lines and
  `#[cfg_attr(verus_keep_ghost, ...)]` marker attributes are the standard ghost
  inclusion/marker pattern, not cfg-gated exec code.)

## AST Consistency
- AST check: **PASS** — `ast_consistency.py` reports 23/23 functions MATCH,
  Consistent: YES, 0 mismatch/missing/extra. No `// VERUS REWRITE` comments exist
  (nothing to audit for semantic equivalence).

## Verification
- verus: **PASS** — `make verify-arch` exit 0 (0 errors); full cross-module
  `make verify` exit 0 (all verified modules pass, no regression);
  `./z build -- all` PASS. `spec_drift.py` → 0 contract drift. Confirmed the prior
  placeholder `assume_specification`s and `Ex*` type specs for the four in-scope
  functions were removed from `identity_map.spec.rs` and superseded by the real arch
  contracts (no weakening).

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` = "None", reconciles with final state).
- True Bugs: **0**. No unrecorded defect discovered during proving/integrity; no
  surviving unresolved verification failure to classify.

## Issues (highest priority first)
- None. Every strict dimension is clean across both independent reviews.

## Result: PASS
