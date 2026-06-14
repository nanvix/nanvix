# Final Comprehensive Review: phys-manager

> Consolidated from two independent sub-agent reviews:
> - `final_review.claude.md` (claude-opus-4.8)
> - `final_review.gpt53codex.md` (gpt-5.3-codex)
>
> Both reviewers independently reached **FAIL** with identical guardrail counts
> (`admit()=4`, `external_body=2`). Verifier exits 0 (0 errors) but the central
> gate reports `CHEATING_DETECTED`; the cached exit-0 is vacuous because the 4
> `admit()`s discharge their proof obligations trivially.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` / rust-analyzer LSP, recorded in `find_callers_output.md`
- [x] Caller expectations (success + failure) documented for each pub function — `caller_analysis.md` §Caller Expectations
- [x] Abstract resource identified — singleton broker over the physical-frame partition (`FrameAllocView`)
- [x] Pre-existing specs assessed — `caller_analysis.md` §Pre-existing Specs (upstream `mm::phys` liveness vocabulary)

### View Design
- [x] Every field passes the substitution test
- [x] All caller-observable state represented (allocated/free sets + refcounts)
- [x] No implementation-specific fields
- [x] inv() encodes real constraints (`self@.wf()` — disjointness, alignment, refcount bounds)
- [x] Mathematical types used (`Set<int>`, `Map`, `nat`; addresses keep `usize`/`int`)

### Specification
- [x] Every in-scope exec function has requires/ensures (6/6 targets; `fn_coverage` 40/45, the 5 uncovered are out-of-scope `clear`/`deref`/`deref_mut`/`get_mut`/`test`)
- [x] Caller coverage: each caller expectation has corresponding requires/ensures (6/6; see Caller Coverage below)
- [x] View consistency: specs reference `FrameAllocView` fields and maintain `inv()`
- [x] No tautological ensures
- [x] No subsumed ensures
- [x] Error paths have meaningful ensures (`Err => final(self)@ == old(self)@`, vector emptied, watermark negation)
- [x] No assume_specification for workspace-internal code (the 3 are `core`/`alloc` std-lib only)
- [x] vstd searched before assume_specification
- [x] Specs written for the caller
- [x] Trait obligations satisfied (none relevant; Drop semantics handled by handle types)
- [x] Spec completeness (advisory)
- [x] Loop invariants present on both bulk loops
- [ ] **No cheating on module's own functions** — `admit=4`, `external_body=2` (one masks the in-scope target `init`)
- [x] No specs weakened — `spec_drift.py` reports 0 contract drift
- [x] Bug awareness — `bugs.md` present; OBS-1/2/3 + BUILD-1 reconciled
- [x] Cross-module regression — kernel crate verifies (exit 0)
- [x] Verification: `make verify-kernel MODULE=mm::phys` exit 0, 0 errors — **but** `CHEATING_DETECTED`

### Proving
- [x] No specs weakened (`spec_drift.py` clean)
- [ ] **Zero remaining admit()** — 4 remain (BLOCKER)
- [ ] **Zero external_body unless listed** — `init` masking is the concern (see TCB Compliance)
- [x] Zero assume/assume_specification beyond std-lib boundaries
- [x] No cfg-gated exec code that changes semantics (all cfg gates are logging / `use` imports)
- [x] Cheating audit performed (counts below)
- [ ] Isolated reproducer for each claimed Verus limitation — the 4 admits cite §8 ghost-token / Drop limits in `bugs.md` but **no `verification-todo.md` / isolated reproducer file** exists for the stuck proofs
- [ ] Exec rewrites minimal + `// VERUS REWRITE` comment — `check_user_watermark` exec was refactored (constant→`kernel_watermark()` accessor, `free_count()` hoist) with **no `// VERUS REWRITE` marker**
- [x] Cross-module regression (kernel verifies)
- [ ] Verification 0 errors **0 warnings** — 0 errors but `CHEATING_DETECTED`

### Cheating Elimination
- [ ] **Zero admit() remaining** — 4 remain (BLOCKER)
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code beyond logging/imports
- [ ] **Zero external_body unless listed** — 2 present; `init` rationale is stale (see below)
- [ ] AST consistency: zero mismatches — undocumented exec refactor in `check_user_watermark`
- [ ] All exec rewrites have VERUS REWRITE comment + minimal reproducer — missing
- [ ] For each surviving external_body confirm listed — both are textually listed, but `init`'s listing rationale no longer holds
- [x] No specs weakened (`spec_drift.py` clean)
- [x] Cross-module regression (kernel verifies)
- [ ] Verification 0 errors 0 warnings — `CHEATING_DETECTED`

### Bug Recording
- [x] bugs.md exists
- [x] Each entry is a real defect or an explicitly record-only observation (OBS-1/2/3, BUILD-1)
- [x] Entries have What / Why / How Verus Helped / Severity / Suggested Fix structure
- [x] No external_body used to mask a code defect (the 2 external_body are storage/atomics + build-time constant boundaries, not defect masks)
- [x] Bug entries include provenance (spec phase turn 1/2, proving phase, review turn 1)

## Spec Quality
Public-API specs for all 6 targets are **strong, correct, and readable**:
- `alloc_user_frame` / `alloc_many_user_frames`: watermark gate (`user_alloc_ok`), exact
  `count`, **distinctness** (`user_addr_set(...).len() == count`, closing the double-free
  hazard from OBS-2), `book_all` transition, all-or-nothing Err (vector emptied,
  `final(self)@ == old(self)@`).
- `alloc_kernel_frame` / `alloc_many_kernel_frames`: free→reserved transition, **contiguity**
  (`kernel_frames_contiguous`), watermark correctly *bypassed*, no-leak Err arm.
- `check_user_watermark`: tight `Ok/Err` partition on `free_count() >= count + watermark`.
- `init`: weakest spec — both Ok and Err arms ensure `manager_ready`, so the Err arm is
  near-degenerate; acceptable given the singleton/atomics boundary but the thinnest contract.

No tautological or subsumed ensures. `spec_kernel_watermark()` is `uninterp` — a justified
external-bottom boundary (build-time constant from a non-Verus crate).

## Caller Coverage
- Covered: **6 / 6** target functions. Every expectation in `caller_analysis.md`
  (watermark gate, contiguity, all-or-nothing, no-leak, init-before-use) maps to a
  `requires`/`ensures`.
- Missing: none material. Minor weakness — `init`'s Err arm is degenerate (both arms assert
  `manager_ready`), and the bulk-API `capacity >= count` / non-empty-vector `InvalidArgument`
  path is encoded as a `requires old(frames)@.len()==0` precondition rather than an Err arm
  (consistent with callers pre-checking).

## Proof Completeness
- Remaining `admit()`: **4** — each a **BLOCKER**:
  - `manager.proof.rs:16` `lemma_manager_attached` (`m@ == phys_view().frames`)
  - `manager.proof.rs:35` `lemma_kernel_alloc_one` (`post == pre.alloc_one(addr)`)
  - `manager.proof.rs:55` `lemma_kernel_alloc_contiguous` (`post == pre.book_all(...)`)
  - `manager.proof.rs:216` `lemma_user_bulk_err_restored` (`m@ == pre` after `clear()`/Drop)
- Remaining `external_body` not in `tcb-allowed.md`: **0** textually (both are listed), but see
  TCB Compliance — `init`'s listed rationale is **stale**.

## TCB Compliance
- `manager.rs:524` `kernel_watermark` — listed YES; rationale **valid** (build-time constant in
  a non-Verus `config` crate; external-bottom boundary; `ensures ret as nat == spec_kernel_watermark()`).
- `manager.rs:96` `init` — listed YES (under "Cross-module dependencies … eliminated when their
  module is verified", rationale *"no specs yet; opaque callee"*). **Rationale is now stale/invalid:**
  `init` is an **in-scope target of the module currently under verification** and now carries a
  real `#[verus_spec]` ensures. `verus-constraints` forbids `external_body` on the current
  module's own functions. The atomics/`MaybeUninit` body is a genuine raw-memory boundary, so an
  entry may be defensible — but the existing justification does not match the current state and
  must be re-evaluated, not silently inherited. **Flagged, not auto-accepted.**

## Guardrails Compliance
- admit: **4** (BLOCKER) — `manager.proof.rs:16,35,55,216`
- assume: **0**
- external_body: **2** — `manager.rs:96` (init), `manager.rs:524` (kernel_watermark)
- assume_specification: **3** — `manager.spec.rs:9` (`Result::and_then`), `:23` (`Result::inspect_err`),
  `:33` (`Vec::capacity`) — all `core`/`alloc` std-lib boundaries (allowed)
- cfg-gated exec: **0 semantic** — all `#[cfg(not(verus_keep_ghost))]` sites are `error!`/`warn!`
  logging and `#[cfg(verus_keep_ghost)]` are `use` imports (permitted category)
- (uninterp: 1 — `spec_kernel_watermark`, justified external-bottom)

## AST Consistency
- AST check: **FAIL (advisory)**. `check_user_watermark` exec was refactored (build-time constant
  read replaced by the `kernel_watermark()` accessor; `frame::free_count()` hoisted before the
  overflow check) and a new `kernel_watermark` exec fn was introduced, **without a
  `// VERUS REWRITE` comment or isolated reproducer**. The change is documented in `bugs.md`
  (behavior-preserving) but does not satisfy the ast-consistency skill's documentation
  requirement. No `// VERUS REWRITE` markers exist anywhere in `manager.rs`.

## Verification
- verus: **PASS (vacuous) / CHEATING_DETECTED**. `make verify-kernel MODULE=mm::phys` exit 0,
  0 verification errors, 42 verified — but the gate reports
  `assume=0 external_body=18 admit=24 cfg_gate=15` crate-wide; the 4 manager `admit()`s discharge
  their obligations trivially, so exit-0 does not imply the proofs hold. `pipeline_state.json`
  records proving = `dialogue-BLOCKED` and cheating-elimination = `FAIL-cheating`.

## Bug Summary
- Total bugs recorded: **4 entries** (OBS-1, OBS-2, OBS-3, BUILD-1) — none are surviving true code defects.
- True Bugs: **0**.
  - OBS-1 (`alloc_many_kernel_frames` `count==0`) — resolved via `requires count > 0` (caller obligation). Severity: low.
  - OBS-2 (distinctness / double-free hazard) — resolved; spec asserts `user_addr_set(...).len()==count`, proven inductively. Severity: high (correctly closed).
  - OBS-3 (`alloc_kernel_frame` Err liveness) — resolved; unsound `lemma_kernel_alloc_err_empty` deleted, Err arm corrected to the strongest sound statement. Severity: high (soundness landmine, removed).
  - BUILD-1 (`unused variable: i` under `-D warnings`) — resolved (`i`→`_idx`). Severity: low.
- Reconciliation: all four are correctly resolved/record-only against the final code. No
  unrecorded bug surfaced during proving. The 4 surviving `admit()`s are **not** code bugs — they
  are deferred ghost-token/Drop trust boundaries; they are verification debt, correctly
  documented in `bugs.md` §Remaining, and are the reason this review FAILs.

## Issues (highest priority first)
1. **[BLOCKER] 4 `admit()` in `manager.proof.rs` (16, 35, 55, 216).** Core facts —
   manager↔global-partition attachment, kernel single/contiguous alloc transitions, and
   user-bulk Drop-restoration — are *assumed*, not proven. Any `admit() > 0` is an automatic FAIL.
2. **[BLOCKER-adjacent] `init` `external_body` with stale TCB rationale.** An in-scope target of
   the module under verification is marked `external_body`; the `tcb-allowed.md` justification
   ("no specs yet; opaque callee") no longer matches reality. Must be re-justified against the
   atomics/raw-memory boundary or eliminated, not inherited.
3. **[Process] No `verification-todo.md` / isolated reproducer** for the 4 stuck proofs, and the
   `check_user_watermark` exec refactor lacks a `// VERUS REWRITE` marker (AST-consistency gap).
4. **[Minor] `init` Err arm is degenerate** (both arms ensure `manager_ready`).

## Result: FAIL

**Rationale:** Both independent reviewers agree. The verifier's exit-0 is vacuous —
4 `admit()`s (`manager.proof.rs:16,35,55,216`) leave the manager's central transition,
attachment, and Drop-restoration guarantees unproven. Per the strict gate, `admit > 0` is an
automatic BLOCKER. Secondary blockers: an in-scope target (`init`) is hidden behind
`external_body` on a now-stale TCB rationale, and the AST-consistency / reproducer documentation
requirements are unmet. Verification debt is honestly recorded in `bugs.md`, but the effort is
**not complete** and does not pass final review.
