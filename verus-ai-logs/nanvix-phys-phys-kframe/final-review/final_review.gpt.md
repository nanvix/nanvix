# Final Review (gpt-5.3-codex): phys-kframe

## Checklist — copy the master item list below; mark each [x]/[ ] with justification

### Caller Analysis
- [x] pub fns callers searched (tool-verified) — validated with `caller_analysis.md` and fresh rg on manager/kpage (`src/kernel/src/mm/phys/manager.rs:388,485`, `src/kernel/src/mm/virt/kpage.rs:58,74`).
- [x] success+failure documented — caller expectations documented in `caller_analysis.md:95-116`.
- [x] abstract resource identified — `caller_analysis.md:121-137`.
- [x] pre-existing specs assessed — `caller_analysis.md:141-177`.

### View Design
- [x] substitution test — documented in `view_design.md:164-183,259-264`.
- [x] all caller-observable state — address-only abstraction justified (`view_design.md:40-45,158-173`).
- [x] no impl-specific fields — `View::V = int`, no leaked internals (`kframe.spec.rs:3-10`, `view_design.md:43-46`).
- [x] inv() encodes real constraints — page alignment (`kframe.spec.rs:20-22`).
- [x] mathematical types (addresses keep usize) — inherited address algebra uses `int`; consistent with module (`kframe.spec.rs:5-9`, `view_design.md:236-241`).

### Specification
- [x] every in-scope exec fn has requires/ensures (fn_coverage.py) — `new` (`kframe.rs:68-79`), `base` (`kframe.rs:132-138`), `drop` has `#[verus_spec]` (`kframe.rs:197-200`) but no semantic postcondition.
- [ ] caller coverage — missing explicit error-path non-consumption for `new` and missing deallocation postcondition for `drop` (details below).
- [x] view consistency — contracts are in terms of `self@`/`inv()` (`kframe.rs:74-76,136-137`, `kframe.spec.rs:20-22`).
- [ ] no tautological ensures — `Err(_) => true` in `new` is tautological (`kframe.rs:77`).
- [ ] no subsumed ensures — `kf.inv()` likely implied by `base.inv()` + `kf@==base@`; `base`’s `result.inv()` likely implied by `self.inv()` + `result@==self@` (`kframe.rs:74-76,136-137`; `kframe.spec.rs:20-22`; `hal/.../frame.rs:58-61`).
- [ ] error paths meaningful — `new` error arm is `true` (`kframe.rs:77`), does not encode caller-required non-consumption (`caller_analysis.md:99-101`).
- [ ] no assume_specification for workspace-internal code — `pub assume_specification[KernelFrame::map_frame]` exists (`kframe.spec.rs:34`), though TCB-listed.
- [ ] vstd searched before assume_specification — no evidence found in artifacts.
- [ ] specs for caller — `drop` lacks allocator-effect postcondition despite caller dependence (`caller_analysis.md:107-108`, `kframe.rs:197-205`).
- [ ] trait obligations (Drop) — behavior exists in body (`kframe.rs:202`) but not encoded in spec contract.
- [ ] spec-completeness — incomplete for `new` Err and `drop` effect.
- [x] loop invariants (none expected) — no loops in in-scope functions.
- [x] no cheating on own functions (grep counts) — in-scope `new/base/drop`: no `admit`, `assume`, `external_body`.
- [x] no specs weakened (spec_drift.py git-diff kframe.rs --before HEAD) — drift check passed (0 drift, exit 0).
- [x] bug awareness — reconciled against `bugs.md` (see Bug Summary).
- [x] cross-module regression — `make verify-kernel MODULE=mm::phys` executed.
- [x] verification reported — included below.

### Proving
- [x] no specs weakened — spec drift pass.
- [x] zero admit() in-scope — none in `new/base/drop`.
- [x] zero external_body unless TCB-listed — none in-scope.
- [x] zero assume/assume_specification except external-bottom trust boundaries [assess map_frame] — only `map_frame` assume_spec exists and is TCB-listed.
- [x] no cfg-gated exec (logging exception) — one cfg-gated logging macro only (`kframe.rs:203`).
- [x] cheating audit counts+locations — provided in Guardrails section.
- [ ] Verus limitations have isolated reproducer — for `map_frame` trust comment, no standalone reproducer in this module’s logs.
- [ ] exec rewrites minimal+equivalent (VERUS REWRITE) — rewrite appears semantically equivalent for `new`, but AST checker still reports MISMATCH.
- [x] cross-module regression — verify command ran.
- [x] verification 0 errors — command exit 0.

### Cheating Elimination
- [x] zero admit — kframe files have none.
- [x] zero assume — kframe files have none.
- [ ] zero trusted — one `assume_specification` remains (`kframe.spec.rs:34`).
- [x] zero exec_allows_no_decreases_clause — none found.
- [x] zero cfg-gated exec (logging allowed) — only logging gate (`kframe.rs:203`).
- [x] zero external_body unless TCB-listed — none present.
- [ ] AST zero mismatches — AST consistency reports 2 mismatches + 1 extra.
- [x] rewrites have VERUS REWRITE+reproducer — rewrite annotated (`kframe.rs:92`), but mismatch still present.
- [x] each external_body confirmed in TCB — no external_body in kframe.
- [x] no specs weakened — drift pass.
- [x] cross-module regression — verify run completed.
- [x] verification 0 errors — exit 0.

### Bug Recording
- [x] bugs.md exists — `verus-ai-logs/nanvix-phys-phys-kframe/bugs.md`.
- [ ] each bug real defect — “Proving-phase note” is stale/contradictory to current code (details below).
- [x] each entry has What/Why/How Verus Helped/Severity/Suggested Fix — both entries include these fields.
- [x] no external_body masking defect — current `new` is not `external_body`; trust moved to `map_frame` assume_spec.
- [ ] provenance — second entry’s claim no longer matches HEAD, so provenance is stale.

## Spec Quality
- `KernelFrame::new` has precondition `base.inv()` and success guarantees `kf@ == base@` and `kf.inv()` (`kframe.rs:68-76`).
- `KernelFrame::new` failure spec is weak/tautological: `Err(_) => true` (`kframe.rs:77`), insufficient for caller expectation that failure does not consume/free `base` (`caller_analysis.md:99-101`).
- `KernelFrame::base` contract is clear and caller-useful (`kframe.rs:132-138`), but `result.inv()` appears derivable from `self.inv()` + `result@ == self@`.
- `KernelFrame::drop` has only `opens_invariants none no_unwind` (`kframe.rs:197-200`) and no semantic postcondition, despite caller reliance on freeing behavior (`caller_analysis.md:107-108`).

## Caller Coverage (Covered 4/6, Missing list)
Covered:
1. `new` input alignment/wf requirement (`kframe.rs:70`, expected by callers at `caller_analysis.md:96-98`).
2. `new` success identity (`kframe.rs:74`, expected by `caller_analysis.md:95-97`).
3. `base` returns same frame address (`kframe.rs:136`, expected by `caller_analysis.md:113-115`).
4. `base` alignment guarantee (`kframe.rs:137`, expected by `caller_analysis.md:115-116`).

Missing:
1. `new` failure non-consumption/no-free guarantee (`caller_analysis.md:99-101`) is not specified (`kframe.rs:77`).
2. `drop` “frees frame exactly once” expectation (`caller_analysis.md:107-108`) is not captured in `requires/ensures` (`kframe.rs:197-205`).

## Proof Completeness (admit count+locations; external_body-not-in-TCB count+locations)
- In-scope `KernelFrame::new/base/drop`:
  - `admit()`: **0** (no matches by rg).
  - `external_body`: **0** (no `#[verus_verify(external_body)]` / `#[verifier::external_body]` matches).
  - `external_body not in TCB`: **0**.

## TCB Compliance (all external_body listed? YES/NO + list)
- **YES** for kframe scope.
- Actual kframe trust boundary found:
  - `assume_specification[KernelFrame::map_frame]` at `kframe.spec.rs:34`.
- TCB allow-list includes it explicitly at `tcb-allowed.md:100-103`.
- No `external_body` declarations in kframe files.

## Guardrails Compliance (admit:N assume:N external_body:N assume_specification:N cfg-gated-exec:N; in-scope vs out-of-scope)
### KFRAME module (`kframe.rs`, `kframe.spec.rs`, `kframe.proof.rs`)
- `admit`: **0**
- `assume`: **0**
- `external_body`: **0**
- `assume_specification`: **1** (`kframe.spec.rs:34`, `KernelFrame::map_frame`)
- `cfg-gated exec`: **1** (`kframe.rs:203`, logging macro only — allowed exception)

### In-scope only (`new`, `base`, `drop`)
- `admit`: 0
- `assume`: 0
- `external_body`: 0
- `assume_specification`: 0
- `cfg-gated exec`: 1 (inside `drop`, logging-only)

### Out-of-scope siblings (from verify output)
- `frame`: 7 `external_body`
- `manager.proof`: 4 `admit`
- `manager.rs`: 2 `external_body`
- `mod.rs`: 2 `external_body`
- `upool.rs`: 3 `external_body`
- `identity_map.rs`: 3 `admit` (outside kframe, reported globally)

## AST Consistency (PASS/FAIL + VERUS REWRITE assessment)
- **FAIL**.
- `ast_consistency.py ... count`: `⚠️ 2 mismatched, 1 extra`.
- `summary`: `KernelFrame::new` = MISMATCH, `KernelFrame::drop` = MISMATCH, `KernelFrame::map_frame` = EXTRA_IN_VERUS.
- `diff --name KernelFrame::new` shows extraction of mapping block into `Self::map_frame(base)?`.
- `// VERUS REWRITE` at `kframe.rs:92` appears semantically equivalent for `new` (same mapping code now in `map_frame`, `kframe.rs:104-113`), but AST checker still reports mismatch; per checklist rule, this is a **BLOCKER**.

## Verification (PASS/FAIL + exit code + cheating summary)
- `make verify-kernel MODULE=mm::phys`: **PASS (exit 0)**.
- Verifier reported `status: CHEATING_DETECTED`.
- Module summary: `external_body=15`, `admit=4`, `cfg-gated exec=11`.
- Global summary: `assume=0`, `external_body=15`, `admit=7`, `trusted=0`, `cfg_gate=12`.
- In-scope kframe functions are not listed among admitted/external_body functions.

## Bug Summary (total recorded, true bugs, stale entries)
- Total entries in `bugs.md`: **2**.
- True current in-scope code bugs: **0** found.
- Stale/contradictory entries: **1**.
  - `bugs.md:32-53` claims `KernelFrame::new` “retains sanctioned external_body”, but current code has no `external_body` on `new`; trust is now via `assume_specification[KernelFrame::map_frame]` (`kframe.spec.rs:34`).
- Historical resolved entry: duplicate import fix (`bugs.md:6-30`) is consistent with current file state.

## Issues (highest priority first)
1. **BLOCKER — AST consistency failure**: 2 mismatches + 1 extra (`new`, `drop`, `map_frame`).
2. **BLOCKER — Missing caller-critical failure/effect contracts**:
   - `new` error arm is tautological (`Err(_) => true`).
   - `drop` lacks semantic postcondition for frame release.
3. **Policy tension — workspace-internal `assume_specification`** on `KernelFrame::map_frame` (TCB-listed, but still trusted internal boundary).
4. **Documentation integrity issue**: `bugs.md` proving-phase note is stale vs HEAD.

## Result: FAIL
Reason: explicit blockers remain (AST mismatches and missing caller-critical spec coverage for `new` failure and `drop` behavior).
Single most important blocker: **AST consistency FAIL (2 mismatches + 1 extra)**.
