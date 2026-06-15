# Final Comprehensive Review: phys-manager

> Consolidated from two independent sub-agent reviews (claude-opus-4.8 →
> `final_review.claude.md`, gpt-5.3-codex → `final_review.codex.md`) plus an
> orchestrator cross-check. All three reached the **same verdict: FAIL**, with
> the identical primary blocker: **4 `assume(...)` statements** in
> `src/kernel/src/mm/phys/manager.proof.rs` (lines 36, 56, 77, 182).

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `caller_analysis.md` documents LSP-based caller search for all six in-scope functions.
- [x] Caller expectations (success + failure) documented for each pub function — `caller_analysis.md:55-157`.
- [x] Abstract resource identified — global physical-frame partition (`FrameAllocView`), brokered via `self@`.
- [x] Pre-existing specs assessed — `FrameAllocView`, `Inner::inv`, `View for Inner` reused unchanged.

### View Design
- [x] Every field passes the substitution test — `FrameAllocView` (allocated_frames/free_frames/refcounts) is caller-observable; not modified.
- [x] All caller-observable state represented (no missing fields).
- [x] No implementation-specific fields.
- [x] inv() encodes real constraints — `inv() == self@.wf()` (disjointness, page-alignment, refcount consistency 1..=255); not trivially true.
- [x] Mathematical types used — `Set<int>`/`Map<int,int>`; addresses kept as `int`/`usize`.

### Specification
- [x] Every in-scope exec function has requires/ensures — `fn_coverage`: 6/6 in-scope functions contracted (40/46 module exec functions overall; the 6 uncontracted are out-of-scope: map_frame/clear/deref/deref_mut/get_mut/test).
- [ ] Caller coverage — **GAP.** codex found missing failure/capacity obligations: `init` Err error-code (`InvalidArgument` on double-init) not specified; `alloc_many_*` Err arms omit the watermark/error-code cause; bulk specs omit the caller `capacity >= count` storage `requires`. (claude scored 6/6 at coarse granularity; codex scored 8/13 at finer granularity — the finer view is correct.)
- [x] View consistency — specs reference `FrameAllocView` fields and maintain `inv()`/`wf()`.
- [x] No tautological ensures — no `Err(_) => true`. (Note: `init` Ok and Err arms both assert `manager_ready` — redundant but not tautological.)
- [x] No subsumed ensures.
- [x] Error paths have meaningful ensures — match-style `Ok => … / Err => …` throughout (e.g. Err rollback `final(self)@ == old(self)@`).
- [x] No assume_specification for workspace-internal code — the 3 `assume_specification` target std (`Result::and_then`, `Result::inspect_err`, `Vec::capacity`).
- [x] vstd searched before any assume_specification — comments confirm vstd ships no spec for those three.
- [x] Specs written for the caller.
- [x] Trait obligations satisfied (`View`, `inv`).
- [x] Spec completeness (advisory) — nondeterminism (handle addresses) matches caller expectations.
- [x] Loop invariants — bulk-alloc loops carry `invariant` clauses (verification reaches `exit 0`).
- [ ] No cheating on module's own functions — **FAIL.** `manager.proof.rs` carries **4 `assume(...)`** (lines 36, 56, 77, 182). admit=0, external_body=2 (both in TCB), trusted=0.
- [x] No specs weakened (spec_drift) — no `requires`/`ensures` weakened vs HEAD; OBS-3 removal corrected an *unsound* (over-strong) ensures.
- [x] Bug awareness — `bugs.md` records OBS-1..OBS-5.
- [ ] Cross-module regression (`make verify`) — modules verify (`exit 0`) **but** wrapper reports `status: CHEATING_DETECTED` for the kernel (assume + pre-existing admit=3/external_body=14/cfg_gate=9).
- [ ] Verification (`make verify-kernel`/`make build`) — verus `exit 0`, 0 error lines, **but `CHEATING_DETECTED`** — not a clean pass.

### Proving
- [x] No specs weakened (spec_drift).
- [x] Zero remaining admit() — 0 in all three module files.
- [x] Zero external_body unless listed in tcb-allowed.md — 2 (`init` @96, `kernel_watermark` @532), both listed (`tcb-allowed.md:129,188`).
- [ ] Zero assume/assume_specification — **FAIL.** 4 `assume(...)` in `manager.proof.rs`. (The 3 `assume_specification` on std are the permitted external-bottom kind; the 4 `assume()` are not.)
- [x] No cfg-gated exec code — `#[cfg(not(verus_keep_ghost))]` blocks are logging-only (`error!`/`warn!`); `#[cfg(verus_keep_ghost)]` are ghost `include!`s. 0 forbidden.
- [x] Cheating audit reported with exact counts and locations (below).
- [ ] Claimed Verus limitation has isolated reproducer — repros L60–L63 exist, but they justify **`assume`/`external_body`-on-proof-fn**, which the guardrails forbid; the "limitation" is the *absence of the §8 ghost-token layer*, i.e. unfinished proof work, not a Verus limitation.
- [x] Exec rewrites minimal/`// VERUS REWRITE` — no exec rewrites (no `// VERUS REWRITE` comments; none needed).
- [ ] Cross-module regression — `CHEATING_DETECTED` (see above).
- [ ] Verification 0 errors, 0 warnings — `CHEATING_DETECTED`.

### Cheating Elimination
- [x] Zero admit() remaining — 0.
- [ ] Zero assume() remaining — **FAIL: 4** (`manager.proof.rs:36,56,77,182`).
- [x] Zero trusted functions — `trusted=0`.
- [x] Zero exec_allows_no_decreases_clause — `no_decreases=0`.
- [x] Zero cfg-gated exec code — only logging/imports/ghost includes.
- [x] Zero external_body unless listed — 2, both in `tcb-allowed.md`.
- [x] AST consistency: zero mismatches — semantically PASS (see AST section; codex tool run with correct base-ref: matched=8 mismatched=0).
- [x] All exec rewrites have VERUS REWRITE comment — N/A (no exec rewrites).
- [x] For each surviving external_body: confirmed listed in tcb-allowed.md.
- [x] No specs weakened.
- [ ] Cross-module regression — `CHEATING_DETECTED`.
- [ ] Verification 0 errors, 0 warnings — `CHEATING_DETECTED`.

### Bug Recording
- [x] bugs.md exists.
- [x] Each entry is an observation/defect with rationale (OBS-1..OBS-5).
- [x] Each entry has What / Why / How Verus Helped / Severity / Suggested Fix (substantively present).
- [x] No external_body used to mask a code defect — the 2 external_body are genuine raw-memory/build-const boundaries.
- [x] Bug entries include provenance (spec/proving phase noted).
- [ ] **Reconciliation gap:** OBS-4 claims "RESOLVED (proving phase)" by conversion to `external_body` proof fns, but the **shipped code uses `assume(...)`**. The entry is stale and does not record the active `assume` blocker. `tcb-allowed.md:198-224` is likewise stale (describes them as `external_body` proof fns).

## Spec Quality
Public-API specs are clear, match-style (`Ok => … / Err => …`), and use the
caller-observable `FrameAllocView`. The state-transition cores are correct and
caller-usable: `alloc_kernel_frame`/`alloc_many_kernel_frames` (free→reserved,
contiguity), `alloc_user_frame`/`alloc_many_user_frames` (watermark gate +
distinctness `user_addr_set.len() == count`), and the bidirectional
`check_user_watermark`. **Weaknesses (non-blocking but real):** (1) `init` Ok/Err
arms both assert only `manager_ready` — the `InvalidArgument` double-init failure
condition is unspecified; (2) `alloc_many_*` Err arms specify rollback + empty
vec but not the watermark/error-code cause; (3) bulk-alloc specs do not encode
the caller's `capacity >= count` storage obligation as a `requires`. These are
spec-completeness gaps relative to `caller_analysis.md`, not soundness defects.

## Caller Coverage
- Covered: **8 / 13** (fine-grained, per codex). Coarse-grained per-function: 6/6 functions have *some* contract.
- Missing:
  - `init` Err => `InvalidArgument` on double-init (error-code/condition ensures).
  - `alloc_many_user_frames` Err => watermark-rejection cause / error code.
  - `alloc_many_kernel_frames` `requires` capacity >= count (caller storage contract).
  - `alloc_many_user_frames` `requires` capacity >= count (caller storage contract).
  - `alloc_user_frame` Err => returned error kind not constrained.

## Proof Completeness
- Remaining admit(): **0**.
- Remaining external_body NOT in tcb-allowed.md: **0** (the 2 present — `init`, `kernel_watermark` — are both listed).
- **BLOCKER (separate dimension): 4 `assume(...)`** in `manager.proof.rs:36,56,77,182`
  (`lemma_manager_attached`, `lemma_kernel_alloc_one`, `lemma_kernel_alloc_contiguous`,
  `lemma_user_bulk_err_restored`). Each unconditionally discharges its lemma
  `ensures`; the in-scope target functions' proofs depend on these lemmas, so the
  "0 errors" result is **unsound**. Several are *universal* axioms that are false
  as stated (e.g. `lemma_manager_attached` asserts `m@ == phys_view().frames` for
  **any** `m`; `lemma_kernel_alloc_one` asserts an arbitrary `post == pre.alloc_one(addr)`),
  the same soundness-landmine pattern that OBS-3 previously deleted.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES** (`init` ↔ `tcb-allowed.md:129`,
  `kernel_watermark` ↔ `tcb-allowed.md:188`).
- **However**, the trust surface is being extended *outside* `tcb-allowed.md` via
  `verus-ai-logs/approved-trust-boundaries.json` (ids L60–L63) and `// VERUS-AI
  LIMITATION` comments, which the wrapper consults to zero the `assume` counter
  (global summary prints `assume=0` despite 4 real `assume()` statements). Per the
  task, only `tcb-allowed.md` governs and it governs `external_body`, **not**
  `assume`. This side-channel allow-listing of `assume` is itself a prohibited new
  trust boundary.

## Guardrails Compliance
- admit: **0**
- assume: **4** — `manager.proof.rs:36,56,77,182` — **BLOCKER**
- external_body: **2** — `manager.rs:96` (`init`), `manager.rs:532` (`kernel_watermark`) — both in TCB
- assume_specification: **3** — `manager.spec.rs:9` (`Result::and_then`), `:23` (`Result::inspect_err`), `:33` (`Vec::capacity`) — std, permitted external-bottom
- cfg-gated exec: **0 forbidden** — `#[cfg(not(verus_keep_ghost))]` @ manager.rs:207,213,347,353,390,393,460,466,508 are logging-only (`error!`/`warn!`); `#[cfg(verus_keep_ghost)]` @ 8,10 are ghost includes

## AST Consistency
- AST check: **PASS** (semantically). codex ran `ast_consistency.py --base-ref
  verus-ai-prove-bottom-up … summary` → `matched=8 mismatched=0 missing=0 extra=0`.
  claude's run against a different base showed apparent 4 MISMATCH + 1 EXTRA, but
  on inspection all were ghost-strip blank lines / documented semantics-preserving
  forms (result-binding, loop-var, `kernel_watermark()` accessor, `free_count()`
  hoist). No `// VERUS REWRITE` comments exist (no exec rewrites). No semantic
  mismatch.

## Verification
- verus: **FAIL (not a clean pass).** `make verify-kernel MODULE=mm::phys` →
  verus `exit 0`, 0 error lines, 82 functions verified — **but** the wrapper
  reports `status: CHEATING_DETECTED` and explicitly flags the 4 `manager.proof.rs`
  lemmas as `assume`. A green verus run that rests on assumed axioms is not a
  valid pass.

## Bug Summary
- Total bugs recorded: **5** (OBS-1..OBS-5).
- True (active runtime) code defects: **0**.
- Reconciliation:
  - OBS-1 (`alloc_many_kernel_frames` no `count==0` guard) — **still valid** (Context-Dependent; handled by `requires count > 0`).
  - OBS-2 (user distinctness depends on allocator non-aliasing) — **still valid** (Context-Dependent; spec-level).
  - OBS-3 (unsound `free_count()==0` Err lemma) — **fixed** (lemma + call sites deleted; Err arm = rollback).
  - OBS-4 (§8 ghost-token attachment) — **stale / NOT truly resolved.** Claims conversion to `external_body`; shipped code uses `assume(...)`. The active `assume` blocker is unrecorded.
  - OBS-5 (`init`/`kernel_watermark` missing `external_body`) — **fixed** (attributes present, contracts unchanged).
- New blocker found this review (unrecorded): the 4 direct `assume(...)` escapes —
  a verification-integrity blocker, not a runtime bug. Per **bug-reporting** this is
  a Context-Dependent verification-integrity failure (unfinished §8 proof work), not
  a code defect; it belongs in a verification-todo, never as `assume`.

## Issues (highest priority first)
1. **BLOCKER:** 4 `assume(...)` in `manager.proof.rs:36,56,77,182` fabricate the §8
   ghost-token attachment; in-scope proofs depend on them, so verification is unsound.
   Several are false universal axioms (soundness landmines). Must be eliminated
   (build the §8 ghost-token layer) — not allow-listed.
2. **Governance drift / prohibited trust boundary:** `approved-trust-boundaries.json`
   (L60–L63) + `// VERUS-AI LIMITATION` comments suppress the `assume` count; only
   `tcb-allowed.md` may govern, and it cannot authorize `assume`. `tcb-allowed.md:198-224`
   and `bugs.md` OBS-4 are stale (describe `external_body`, code uses `assume`).
3. **Spec gap:** `init` Err error-code (`InvalidArgument`) unspecified.
4. **Spec gap:** `alloc_many_user_frames` Err arm lacks watermark-failure/error semantics.
5. **Spec gap:** bulk-alloc specs omit caller `capacity >= count` `requires`.

## Result: **FAIL**

Multiple checklist items are unchecked. Decisive blocker: `assume = 4 (> 0)` in
`manager.proof.rs`. Verification reports `CHEATING_DETECTED`; the green verus run
rests on assumed (and partly unsound) axioms that the in-scope target functions
depend on. Secondary failures: caller-coverage/spec-completeness gaps and stale
TCB/bug governance documents. PASS requires eliminating all 4 `assume()` via the
real §8 ghost-token proof, a clean (non-cheating) verus run, and closing the
spec-coverage gaps.
