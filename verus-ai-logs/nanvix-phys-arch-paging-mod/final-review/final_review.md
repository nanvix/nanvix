# Final Comprehensive Review: arch-paging-mod

Consolidated from two independent reviews:
- `final_review.claude.md` (claude-opus-4.8)
- `final_review.codex.md` (gpt-5.3-codex)

Both reviewers ran live tools (`make verify-arch`, `ast_consistency.py`, greps) and
reached **PASS** with no blockers. The consolidating agent additionally ran
`make verify` (cross-module), `./z build`, and `spec_drift.py`. In-scope target:
`src/libs/arch/src/x86/mem/paging/mod.rs::invlpg` (only function in scope).

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` run recorded in caller_analysis.md; LSP false-negative reconciled with repo-wide search (identity_map.rs:668, page_table.rs:210/329/385/433/498, page_directory.rs:170).
- [x] Caller expectations (success + failure) documented for each pub function — caller_analysis.md §Caller Expectations; `invlpg` is infallible (`-> ()`, no error path).
- [x] Abstract resource identified — the CPU TLB (hardware state outside Verus' memory model); caller_analysis.md §Abstract Resource.
- [x] Pre-existing specs assessed — inherited `assume_specification[::arch::mem::paging::invlpg]` (empty contract) assessed; now superseded/removed (identity_map.spec.rs:151-155).

### View Design
- [x] Every field passes the substitution test — N/A by design: no View (standalone fn over external hardware); view_design.md documents the degenerate/empty View as the intended outcome.
- [x] All caller-observable state represented — TLB effect is unobservable in Rust-visible state; nothing to represent.
- [x] No implementation-specific fields — none (no View).
- [x] inv() encodes real constraints — N/A (no View / no struct).
- [x] Mathematical types used — N/A; the sole parameter `vaddr: usize` is an address (allowed usize exception).

### Specification
- [x] Every in-scope exec function has requires/ensures — `invlpg` carries an external-body trust-boundary contract (faithful empty contract); verify-arch coverage report accounts for it.
- [x] Caller coverage — all caller expectations satisfied by the side-effect-only/infallible contract (see Caller Coverage below).
- [x] View consistency — no View by design; spec preserves all caller-side invariants (touches no Rust-visible state).
- [x] No tautological ensures — no `Err(_) => true`; function is infallible with no ensures (empty, not tautological).
- [x] No subsumed ensures — none present.
- [x] Error paths have meaningful ensures — N/A; no error path.
- [x] No assume_specification for workspace-internal code — the upstream one was removed; module owns the contract via external_body. assume_specification count = 0.
- [x] vstd searched before any assume_specification — N/A (none used).
- [x] Specs written for the caller — empty contract is directly usable; callers rely only on the (un-modeled) hardware effect.
- [x] Trait obligations satisfied — none; `invlpg` is a free `unsafe fn` (caller_analysis.md §Trait Obligations).
- [x] Spec completeness (advisory) — empty contract matches caller expectations (TLB side effect intentionally outside the model).
- [x] Loop invariants — N/A; no loops.
- [x] No cheating on module's own functions — admit=0, assume=0, trusted=0; the single external_body (`invlpg`) is the pre-approved TCB hardware boundary.
- [x] No specs weakened — `spec_drift.py`: 0 contract drift, 0 ensures removed, 0 requires added.
- [x] Bug awareness — no fundamentally incorrect code; no bug to record.
- [x] Cross-module regression — `make verify` exit 0 (arch + kernel).
- [x] Verification — `make verify-arch` exit 0 (47 verified, 0 errors); `./z build` exit 0.

### Proving
- [x] No specs weakened — `spec_drift.py` clean (0 drift).
- [x] Zero remaining admit() — grep over paging subtree + verify-arch report: admit=0.
- [x] Zero external_body unless listed in tcb-allowed.md — only `invlpg`; listed at tcb-allowed.md:52.
- [x] Zero assume/assume_specification — assume=0, assume_specification=0.
- [x] No cfg-gated exec code — the two `#[cfg(verus_keep_ghost)] include!(...)` lines (mod.rs:8,10) are spec/proof include directives, not exec.
- [x] Cheating audit — admit=0, external_body=1 (in scope, TCB-listed), assume=0, cfg-gated exec=0.
- [x] Any claimed Verus limitation has an isolated reproducer — verus-unsupported.md §1 has a minimal inline-asm reproducer with the exact Verus error.
- [x] Exec rewrites minimal & equivalent — no `// VERUS REWRITE` comments in scope (none needed).
- [x] Cross-module regression — `make verify` exit 0.
- [x] Verification — `make verify-arch` exit 0; build exit 0; 0 errors.

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause (no_decreases=0)
- [x] Zero cfg-gated exec code (only spec/proof includes present)
- [x] Zero external_body unless listed in tcb-allowed.md — `invlpg` listed (tcb-allowed.md:52)
- [x] AST consistency: zero mismatches — `ast_consistency.py`: invlpg MATCH (matched=1, mismatched=0)
- [x] All exec rewrites have VERUS REWRITE comment + reproducer — none required (no rewrites)
- [x] For each surviving external_body: confirmed in tcb-allowed.md — `invlpg` confirmed
- [x] No specs weakened — `spec_drift.py` clean
- [x] Cross-module regression — `make verify` exit 0
- [x] Verification — `make verify-arch` exit 0; build exit 0

### Bug Recording
- [x] bugs.md exists if bugs were found — no bugs found, so no file needed (per template rule)
- [x] Each bug is a real code defect — N/A (no bugs)
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix — N/A
- [x] No external_body used to mask a code defect — `invlpg`'s external_body is a genuine inline-asm hardware boundary, not masking a defect
- [x] Bug entries include provenance — N/A

## Spec Quality
The faithful contract for `invlpg` is **empty** (no `requires`, no `ensures`) on an
`#[verus_verify(external_body)]` function (mod.rs:79), and both reviewers independently
confirm this is correct rather than a shortcut:
- The body is a single `core::arch::asm!("invlpg ({0})", …)` (mod.rs:81-85). Inline-asm
  expressions are genuinely unsupported by Verus (verus-unsupported.md §1, with minimal
  reproducer) — an external-bottom hardware boundary, not an avoidable proof gap.
- The TLB is unobservable hardware state (no `PointsTo`, no value read back, no error path,
  returns `()`), so there is no abstract state to specify; any non-trivial postcondition
  would be an over-faithful/abstraction-leak anti-pattern (view_design.md rejected
  alternatives).
- No `requires`: the instruction accepts any operand and is a no-op when no matching TLB
  entry exists; the ring-0 obligation is the `unsafe` caller's responsibility.
- The 18-line trust-boundary comment (mod.rs:69-77) documents the boundary and rationale.
- **Upstream cross-check:** the inherited `pub assume_specification[::arch::mem::paging::invlpg]`
  at identity_map.spec.rs:151 has been **removed** and replaced by a comment
  (identity_map.spec.rs:151-155): superseded by this module's own external_body contract —
  the correct bottom-up outcome, with **no duplicated trust boundary**.

Verdict: spec is correct, complete, and understandable.

## Caller Coverage
- Covered: **1 / 1** functions (`invlpg`); expectation items 5/5 (codex enumeration).
- Missing: **none**. Every call site (identity_map.rs:668, page_table.rs:210/329/385/433/498,
  page_directory.rs:170) uses `invlpg` identically as a fire-and-forget TLB flush and ignores
  any result; no caller needs a property the empty contract omits.

## Proof Completeness
- Remaining admit(): **0** (none in mod.rs/mod.spec.rs/mod.proof.rs or the paging subtree).
- Remaining external_body not in tcb-allowed.md: **0**. In-scope `external_body` = 1
  (`invlpg`, mod.rs:79), which is listed in tcb-allowed.md:52.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES**.
  `src/libs/arch/src/x86/mem/paging/mod.rs::invlpg` → tcb-allowed.md:52 (dedicated section
  "external_body introduced while speccing arch::x86::mem::paging (mod.rs)"). No new trust
  boundary introduced or justified by this review.

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **1** (in-scope `invlpg`, TCB-listed;
  crate-wide verify-arch reports 3, the other two being `table::read`/`table::write`, both
  TCB-listed and out of scope), assume_specification: **0**, cfg-gated exec: **0**.
  Note: verify-arch's `cfg_gate=4` originates entirely from the `#[cfg(verus_keep_ghost)]
  include!(...)` spec/proof directives (mod.rs:8,10 and table.rs:9,11), which are include
  directives, not exec-code branches.

## AST Consistency
- AST check: **PASS** — `ast_consistency.py … summary`: `invlpg` MATCH (matched=1,
  mismatched=0, missing=0, extra=0). No `// VERUS REWRITE` comments in scope.

## Verification
- verus: **PASS** — `make verify-arch` exit 0; fresh run "47 verified, 0 errors";
  cheating line `assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=4`.
- Cross-module `make verify`: **PASS** (exit 0). Kernel's pre-existing `admit=36`/`external_body=12`
  are in unverified kernel modules outside this bottom-up scope and are not regressions
  (spec_drift: 0 changes to mod.rs).
- `./z build`: **PASS** (exit 0, "Build complete").
- `spec_drift.py`: **clean** (0 contract drift).

## Bug Summary
- Total bugs recorded: **0** (bugs.md absent — correct, no bugs found).
- True Bugs: **0**. The historic `admit()` in `table.proof.rs::lemma_entry_roundtrip` was a
  different module's dead spec-phase axiom (generic over `E`, never `broadcast use`d, no
  caller); correctly **removed** (not swapped for assume/external_body) with the real proof
  deferred to the `table` proving phase — a legitimate deferral, not a surviving failure and
  not a defect in `invlpg`.

## Issues (highest priority first)
1. (Informational, non-blocking) Reference docs slightly stale: caller_analysis.md and
   tcb-allowed.md still describe the upstream `assume_specification[…invlpg]` as live at
   identity_map.spec.rs:151, but it has been removed (now a comment). The actual state is the
   correct, stronger one (module owns the contract). No action required.

No BLOCKERS: admit=0, assume=0, the only in-scope external_body is TCB-allowlisted, AST MATCH,
verus exit 0, build exit 0, no specs weakened.

## Result: PASS
