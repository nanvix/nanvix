# Final Comprehensive Review: arch-frame-number

Consolidated from two independent sub-agent reviews:
- `final_review.claude.md` (claude-opus-4.8)
- `final_review.codex.md` (gpt-5.3-codex)

Both reviewers independently reached **PASS** with zero blockers.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` output recorded in caller_analysis.md (2 pub fns + type, 12 refs)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified — opaque validated physical frame index in `0..=MAX`
- [x] Pre-existing specs assessed — upstream `number.spec.rs`/`.proof.rs` were empty; kernel placeholders superseded

### View Design
- [x] Every field passes the substitution test — single `self@ : int` index survives a complete rewrite
- [x] All caller-observable state represented — the lone frame index
- [x] No implementation-specific fields — `usize` newtype not mirrored; abstract `int`
- [x] inv() encodes real constraints — `0 <= self@ <= spec_max()`, not trivially true
- [x] Mathematical types used — View is `int`, bound is `nat`

### Specification
- [x] Every in-scope exec function has requires/ensures (`fn_coverage.py`: 4/4 matched; 2 in-scope pub fns specced, 2 out-of-scope unit tests)
- [x] Caller coverage: each expectation has corresponding requires/ensures/inv (4/4)
- [x] View consistency: specs reference `self@` / `spec_max()` and maintain `inv()`
- [x] No tautological ensures
- [x] No subsumed ensures
- [x] Error paths have meaningful ensures (`value > spec_max() ==> result is None`)
- [x] No assume_specification for workspace-internal code (none present)
- [x] vstd searched before any assume_specification (none needed)
- [x] Specs written for the caller (directly usable in PTE/PDE proofs)
- [x] Trait obligations satisfied (no semantic trait contracts; only Debug/Clone/Copy)
- [x] Spec completeness (advisory): no unintended nondeterminism
- [x] Loop invariants: N/A (no loops)
- [x] No cheating on module's own functions: admit=0, assume=0, external_body=0, trusted=0
- [x] No specs weakened: `spec_drift.py` → 0 contract drift
- [x] Bug awareness: no fundamentally incorrect code in scope
- [x] Cross-module regression: `make verify` → all crates exit 0
- [x] Verification: `make verify-arch` exit 0; arch crate compiles clean

### Proving
- [x] No specs weakened (`spec_drift.py`: 0 drift)
- [x] Zero remaining admit()
- [x] Zero external_body in module (3 arch external_body all in `tcb-allowed.md`)
- [x] Zero assume/assume_specification
- [x] No cfg-gated exec code (only `#[cfg(verus_keep_ghost)]` spec/proof `include!`s)
- [x] Cheating audit: admit=0, external_body=0, assume=0, cfg-gated exec=0 (module)
- [x] No claimed Verus limitation (none claimed; no rewrites)
- [x] Exec rewrites minimal and equivalent — none exist (no `// VERUS REWRITE`)
- [x] Cross-module regression: `make verify` passes
- [x] Verification: `make verify-arch` 0 errors

### Cheating Elimination
- [x] Zero admit()
- [x] Zero assume()
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code
- [x] Zero external_body in module; all 3 arch external_body in `tcb-allowed.md`
- [x] AST consistency: zero mismatches (no rewrites; exec byte-for-byte unchanged except a ghost-only `proof!` block)
- [x] All exec rewrites have VERUS REWRITE comment — N/A (none)
- [x] Each surviving external_body confirmed in `tcb-allowed.md`
- [x] No specs weakened (`spec_drift.py`: 0)
- [x] Cross-module regression: `make verify` passes
- [x] Verification: `make verify-arch` 0 errors

### Bug Recording
- [x] bugs.md absent — correct, no in-scope bugs found
- [x] N/A — no bug entries
- [x] N/A
- [x] No external_body used to mask a code defect
- [x] N/A

## Spec Quality
Public API contracts are correct, complete, declarative, and readable.

- `from_raw_value`: bidirectional iff — `value as int <= spec_max() ==> Some && result@ == value`, and `value as int > spec_max() ==> None`. Boundary matches the exec body's `value > Self::MAX` exactly because `spec_max() as int == MAX as int`.
- `into_raw_value`: total, value-preserving (`result as int == self@`), and in-range (`0 <= self@ <= spec_max()`) — the in-range fact is what underwrites the caller's overflow-safe `<< FRAME_SHIFT`. Uses `use_type_invariant(self)` correctly to surface the bound.
- `View = int`, `inv = 0 <= self@ <= spec_max()` — minimal and real.

Notable strengthening vs. design doc: the **shipped** spec uses
`open spec fn spec_max() -> nat { (MAX_ADDRESS / FRAME_SIZE - 1) as nat }` with **no**
`assume_specification[FrameNumber::MAX]`, whereas `view_design.md` proposed an
`uninterp spec_max` + a trusted `assume_specification`. The shipped approach is **stronger and
trust-free**: the binding to exec `MAX` is discharged by verification, not assumed. Both reviewers
independently confirmed the `nat` cast cannot underflow (`MAX_ADDRESS = usize::MAX`,
`FRAME_SIZE = 4096` ⇒ quotient ≥ 1 ⇒ `- 1 ≥ 0`) and that it mirrors exec `MAX` with no off-by-one.

## Caller Coverage
- Covered: **4 / 4**
  - Round-trip identity — covered by `from_raw_value` (`result@ == value`) + `into_raw_value` (`result as int == self@`).
  - Out-of-range rejection — `value as int > spec_max() ==> result is None`.
  - Overflow-safe bound for `<< FRAME_SHIFT` — `inv()` + `into_raw_value` in-range ensures.
  - Totality of `into_raw_value` — total `fn` returning `usize`, no panic/Option.
- Missing: **none**

## Proof Completeness
- Remaining admit(): **0**
- Remaining external_body not in `tcb-allowed.md`: **0** (module has 0 external_body; the 3 arch-wide ones — `paging/mod.rs::invlpg`, `table.rs::read`, `table.rs::write` — are all listed)

## TCB Compliance
- All external_body listed in `tcb-allowed.md`: **YES**. No new trust boundary introduced by this module (module external_body = 0).

## Guardrails Compliance
Module (`number.rs` / `.spec.rs` / `.proof.rs`):
- admit: **0**, assume: **0**, external_body: **0**, assume_specification: **0**, cfg-gated exec: **0**

Arch crate (context, all pre-existing & TCB-approved, outside this module):
- external_body: 3 (invlpg, table read, table write) — all in `tcb-allowed.md`; admit=0, assume=0, cfg_gate=0.

## AST Consistency
- AST check: **PASS** — no `// VERUS REWRITE` comments anywhere; exec code byte-for-byte unchanged from the pre-spec baseline except the ghost-only `proof! { use_type_invariant(self); }` block in `into_raw_value`. No semantic mismatch.

## Verification
- verus: **PASS** — `make verify-arch` exit 0; `make verify` (full cross-module regression) exit 0 for all crates. (`make build` is a no-op target in this repo; arch compilation is exercised by `verify-arch`.)

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` absent — correct)
- True Bugs: **0**. No code defect in scope; no surviving unresolved verification failure; no external_body masking a defect.

## Issues (highest priority first)
1. **[Doc lag, non-blocking]** `view_design.md` still describes the *rejected* `uninterp spec_max` + `assume_specification[FrameNumber::MAX]` design. The shipped spec is strictly stronger (no trust boundary). Recommend updating the doc to match the shipped `open spec` approach for future-reader accuracy. Does not affect verification or guarantees.
2. **[Advisory, non-blocking]** `spec_max()` is an extra `pub` spec fn beyond the View/inv. Justified — it names the architectural bound used by both `inv()` and the constructor split and is reached by downstream crates via the exported type.
3. **[Note, non-blocking]** `FrameNumber::NULL` carries `ensures Self::NULL@ == 0`. The associated const is borderline-in-scope; the ensures is correct and harmless.

No blockers. Both independent reviewers concur.

## Result: **PASS**
