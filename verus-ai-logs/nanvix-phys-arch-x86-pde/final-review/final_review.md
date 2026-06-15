# Final Comprehensive Review: arch-x86-pde

> Consolidated from two independent sub-agent reviews (one per model):
> - `final_review.claude.md` (claude-opus-4.8)
> - `final_review.codex.md` (gpt-5.3-codex)
>
> Both reviewers independently investigated with tooling and reached **PASS**
> with identical cheating counts. This consolidation reconciles their findings.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified
- [x] Pre-existing specs assessed (if any exist from upstream verification)

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite)
- [x] All caller-observable state represented (no missing fields)
- [x] No implementation-specific fields (only caller-observable state)
- [x] inv() encodes real constraints (not trivially true)
- [x] Mathematical types used (int/Seq/Set/Map; exception: addresses keep usize)

### Specification
- [x] Every in-scope exec function has requires/ensures (run `fn_coverage.py`)
- [x] Caller coverage: each caller expectation has corresponding requires/ensures
- [x] View consistency: specs reference View fields and maintain inv()
- [x] No tautological ensures (e.g., `Err(_) => true`)
- [x] No subsumed ensures (derivable from inv() + other ensures)
- [x] Error paths have meaningful ensures (N/A — all 5 in-scope fns are total/infallible)
- [x] No assume_specification for workspace-internal code
- [x] vstd searched before any assume_specification (none used)
- [x] Specs written for the caller (usable directly in caller proofs)
- [x] Trait obligations satisfied (specs match trait-level semantic contracts)
- [x] Spec completeness (advisory): no unintended nondeterminism
- [x] Loop invariants: N/A — no loops in in-scope functions
- [x] No cheating on module's own functions: admit=0, assume=0, external_body=0, trusted=0
- [x] No specs weakened: `spec_drift.py` → 0 contract drift
- [x] Bug awareness: no fundamentally incorrect code; bugs.md accurate
- [x] Cross-module regression: `make verify` exits 0 (all verified modules pass)
- [x] Verification: `make verify-arch` (48 verified, 0 errors) and `make build` pass

### Proving
- [x] No specs weakened: `spec_drift.py` → no contract drift
- [x] Zero remaining admit()
- [x] Zero external_body (none in pde; in-scope rule satisfied trivially)
- [x] Zero assume/assume_specification
- [x] No cfg-gated exec code
- [x] Cheating audit: admit=0, external_body=0, assume=0, cfg-gated exec=0
- [x] Claimed Verus limitation has an isolated reproducer (frame_address)
- [x] Exec rewrites are minimal and semantically equivalent (`// VERUS REWRITE`)
- [x] Cross-module regression: `make verify` exits 0
- [x] Verification: `make verify-arch` + `make build` — 0 errors

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code (only the two `include!` spec/proof gates remain)
- [x] Zero external_body in pde (in-scope)
- [x] AST consistency: only the one semantically-equivalent frame_address rewrite
- [x] All exec rewrites have VERUS REWRITE comment and minimal reproducer
- [x] For each surviving external_body (crate-wide): all listed in tcb-allowed.md
- [x] No specs weakened: `spec_drift.py` → no weakening
- [x] Cross-module regression: `make verify` exits 0
- [x] Verification: `make verify-arch` + `make build` — 0 errors

### Bug Recording
- [x] bugs.md exists and accurately records "no bugs found"
- [x] Each claim reconciled against final code state (overflow genuinely proven)
- [x] N/A — no bug entries (no defects found); structure check not applicable
- [x] No external_body used to mask a code defect (zero external_body in pde)
- [x] N/A — no bug entries requiring provenance

## Spec Quality
The public API contracts for all five in-scope functions are correct, complete,
and readable; both reviewers concur.

- `PageDirectoryEntryFlags::new` — `ensures result@ == spec_pde_flags_new(...)`
  faithfully records all eight flag arguments into the closed `PdeFlagsView`.
- `PageDirectoryEntry::new` — `ensures result@ == spec_pde_new(flags@, frame@)
  && result.inv()`: pairs exactly the given flags and frame, and pins the
  frame-bound invariant inherited from `FrameNumber`.
- `PageDirectoryEntry::is_present` / `PageDirectoryEntryFlags::is_present` —
  `ensures result == self@.flags.present` / `result == self@.present`: pure
  read-back of the present bit, delegating correctly.
- `PageDirectoryEntry::frame_address` — `ensures result as int == self@.frame *
  FRAME_SIZE && result as int % FRAME_SIZE == 0`: the physical base address is
  the frame index times frame size, and is page-aligned (overflow-free, proven
  by `lemma_frame_address`).

Specs use mathematical types (`int`), reference the closed `View`, and maintain
a meaningful `inv()` (`0 <= self@.frame <= FrameNumber::spec_max()` — the real
constraint that makes `frame_address` total/overflow-free). No tautological or
subsumed ensures. `PageDirectoryEntryFlags::inv()` is vacuously `true`, which is
correct: a flags bundle has no cross-field constraint (every bit combination is
legal) — this is justified in the spec comments, not a missing invariant.

## Caller Coverage
- Covered: **21 / 21** (6/6 numbered caller invariants + 15/15 per-function
  expectations across the 5 in-scope functions; both reviewers agree, claude
  framed it as 6/6 + 5/5 by-function which is the same set).
- Missing: **none**.

Every expectation in `caller_analysis.md` maps to a spec: constructor fidelity
(flags + entry), presence delegation, frame alignment, purity/totality, and
encoding independence (closed View) are all expressed in the ensures/inv().

## Proof Completeness
- Remaining admit(): **0** [none — no BLOCKER]
- Remaining external_body not in tcb-allowed.md: **0** [none in pde — no BLOCKER]

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES**.
  The arch crate's only trusted items are `x86/mem/paging/mod.rs::invlpg`,
  `table.rs::read`, `table.rs::write` (external_body) and
  `table.proof.rs::lemma_entry_roundtrip` (broadcast axiom, reported as
  `assume`) — **all four are pre-approved in tcb-allowed.md**, and **none are in
  pde**. No new trust boundary introduced.

## Guardrails Compliance
(counts over the three pde files: pde.rs, pde.spec.rs, pde.proof.rs)
- admit: **0**, assume: **0**, external_body: **0**, assume_specification: **0**,
  cfg-gated exec: **0**.

Note: `pde.rs:9` and `pde.rs:11` carry `#[cfg(verus_keep_ghost)]` solely to gate
the `include!` of the spec/proof bodies — standard spec/proof inclusion, **not**
cfg-gated exec code.

## AST Consistency
- AST check: **PASS** (22 functions MATCH, 1 MISMATCH that is a pre-approved,
  semantically-equivalent deviation).

The single MISMATCH is `PageDirectoryEntry::frame_address`:
```
-  self.frame.into_raw_value() << crate::mem::FRAME_SHIFT
+  let raw: usize = self.frame.into_raw_value();
+  raw << crate::mem::FRAME_SHIFT
```
This is the pre-approved `f(complex_expr)` → `let x = complex_expr; f(x)`
deviation (intermediate value for assertions). The `let` binding is required so
`proof! { lemma_frame_address(raw); }` can run between the exec call (whose
postcondition bounds `raw`) and the overflow-bearing shift. Same value, same
operations — semantically equivalent. Documented with a `// VERUS REWRITE`
comment (pde.rs:420) and an isolated reproducer
(`cheating-elimination/repro/frame_address.rs`, present, 4436 B).

## Verification
- verus: **PASS** — `make verify-arch` → 48 verified, 0 errors, exit 0.
- build: **PASS** — `make build` → exit 0 (nothing to rebuild, cached).
- spec drift: **PASS** — `spec_drift.py` → 0 contract drift, no weakening.
- cross-module: **PASS** — `make verify` exits 0 (no regression). The kernel
  crate's pre-existing `admit=4` (all in `mm/phys/manager.proof.rs`) and
  `external_body=23` are in separate in-progress modules, **not** in pde and not
  introduced by this effort.

## Bug Summary
- Total bugs recorded: **0**
- True Bugs: **0**

`bugs.md` claim ("no bugs found") reconciled against the final code: accurate.
The `frame_address` overflow concern is genuinely proven (not assumed away);
with zero external_body in pde, no defect can be masked. No new defects were
discovered during proving or cheating-elimination.

## Issues (highest priority first)
1. (Non-blocking, nomenclature) The `frame_address` deviation comment uses the
   prefix `// VERUS REWRITE` rather than the `// VERUS DEVIATION` wording in the
   ast-consistency skill table. The content is complete (what/why/reproducer);
   this is a label nit with no semantic impact. No action required for PASS.

No blocking issues. No missing caller coverage. No cheating constructs in scope.

## Result: PASS
