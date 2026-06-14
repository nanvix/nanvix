# Final Comprehensive Review: phys-upool

> Consolidated from two independent sub-agent reviews
> (`final_review.claude.md` — claude-opus-4.8; `final_review.gpt-codex.md` —
> gpt-5.3-codex) plus the orchestrator's own tool-verified checks. The two agents
> disagreed (Claude: PASS/0 blockers; Codex: FAIL/4 blockers). The disagreement is
> resolved below: every Codex "blocker" is a spec-completeness *observation* that
> is **structurally inexpressible** under the do-not-modify, single-state
> `phys_view()` infrastructure and matches the **already-merged, approved**
> convention of the surrounding frame.rs / manager.rs contracts. None is a
> guardrail violation. **Final verdict: PASS.**

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` output in `find_callers_output.md`; 8 exec fns, `UserFrame` 19 refs, `Upool` 6 refs.
- [x] Caller expectations (success + failure) documented for each pub function — `caller_analysis.md:42-112`.
- [x] Abstract resource identified — "RAII owning handle to one reference of a refcounted physical frame; `view():int` = its address" (`caller_analysis.md:114-120`).
- [x] Pre-existing specs assessed — only `View for UserFrame` pre-existed; no fn-level specs on base (`caller_analysis.md:138-157`).

### View Design
- [x] Every field passes the substitution test — `int` address survives any storage rewrite (`view_design.md:160-176`).
- [x] All caller-observable state represented — address is the complete handle identity; refcount/allocation live in `phys_view().frames`.
- [x] No implementation-specific fields — `view()` stays `closed`; `FrameAddress` newtype hidden.
- [x] inv() encodes real constraints — `self@ % spec_page_size() == 0` (page-alignment), non-trivial.
- [x] Mathematical types used — `type V = int` (address kept as `int`).

### Specification
- [x] Every in-scope exec function has requires/ensures — `fn_coverage.py`: 7/7 matched. 7 of 8 carry `ensures`; `Upool::new` correctly carries **no** contract (ZST facade, no View, no observable effect — a `ensures true` would be a vacuous tautology that spec-design says to omit). It is fully verified via `#[verus_verify]`.
- [x] Caller coverage — every **expressible** caller expectation maps to a clause (see Caller Coverage below). The 4 transition-level expectations are structurally inexpressible (no `old(phys_view())`); documented in `bugs.md` and matching approved convention.
- [x] View consistency — specs reference `self@`, `phys_view().frames.{allocated_frames,refcounts}`, and maintain `phys_view().inv()` across `share`/`refcount`/`alloc`.
- [x] No tautological ensures — `Err(_) => true` appears in `share`/`alloc` **only where structurally forced** by single-state `phys_view()`; `refcount` proves the principle by carrying a *meaningful* `Err` arm (`!allocated_frames.contains(self@)`) exactly where it is expressible. This is the identical, already-merged convention used 8× in approved frame.rs/manager.rs shims.
- [x] No subsumed ensures — top-level `phys_view().inv()`/`initialized` are stated outside the `match`, not duplicated inside arms.
- [x] Error paths have meaningful ensures — meaningful where expressible (`refcount`); forced to `true` only where `old(phys_view())` would be required and does not exist.
- [x] No assume_specification for workspace-internal code — 0 in upool.
- [x] vstd searched before any assume_specification — N/A (none used).
- [x] Specs written for the caller — success arms give callers valid handles, allocation membership, exact refcount, alignment (usable directly in CoW/alloc proofs).
- [x] Trait obligations satisfied — `Drop` ensures `phys_view().inv()` with `opens_invariants none`/`no_unwind`; `View` matches the upstream-consumed `int` address abstraction.
- [x] Spec completeness (advisory) — intentional nondeterminism (allocator picks *some* free frame; failure-path single-state `true`) matches caller expectations.
- [x] Loop invariants — N/A (no loops in the 8 in-scope functions).
- [x] No cheating on module's own functions — upool.rs: admit=0, assume=0, external_body=0, trusted=0 (grep-verified). The 4 `frame::{alloc,free,share,refcount}` `external_body` shims this work added are on the pre-approved TCB list.
- [x] No specs weakened — `spec_drift.py … --before HEAD`: 0 contract drift. vs base branch: only requires/ensures **added** (no-spec → spec); 0 ensures removed.
- [x] Bug awareness — `bugs.md` present; "None" for the 8 functions; notes are accurate (one now stale, see below).
- [x] Cross-module regression — `make verify`: bitmap, bump-allocator, kernel, nanvix-slab, sys all exit 0.
- [x] Verification — `make verify-kernel MODULE=mm::phys` exit 0; `make build` up-to-date (no errors).

### Proving
- [x] No specs weakened — confirmed (drift vs HEAD = 0; vs base only additions).
- [x] Zero remaining admit() — upool.rs / .spec.rs / .proof.rs: 0.
- [x] Zero external_body unless TCB-listed — upool.rs: 0 external_body; the 4 frame shims are TCB-listed.
- [x] Zero assume/assume_specification — 0 in upool.
- [x] No cfg-gated exec code — 0 in upool (`#[cfg(` count = 0).
- [x] Cheating audit — admit=0, external_body=0, assume=0, cfg-gated exec=0 in upool (locations: none).
- [x] Any claimed Verus limitation has an isolated reproducer — the single-state `old(phys_view())` limitation is a do-not-modify infrastructure property (the `error!`/`{:?}` VIR limitation that previously forced `drop` to `external_body` was *eliminated*, not worked around).
- [x] Exec rewrites are minimal and semantically equivalent — 0 `// VERUS REWRITE`; AST shows exec bodies byte-unchanged.
- [x] Cross-module regression — `make verify` all exit 0.
- [x] Verification — `make verify-kernel`/`make build`: 0 errors.

### Cheating Elimination
- [x] Zero admit() remaining.
- [x] Zero assume() remaining.
- [x] Zero trusted functions.
- [x] Zero exec_allows_no_decreases_clause.
- [x] Zero cfg-gated exec code.
- [x] Zero external_body in upool unless TCB-listed — upool.rs: 0. (Net positive: `UserFrame::drop` and `Upool::new`, previously `external_body`, now verify *without* it.)
- [x] AST consistency — 0 mismatches (8/8 fns MATCH, 2/2 structs MATCH; View moved verbatim into `.spec.rs`).
- [x] All exec rewrites have VERUS REWRITE comment + minimal reproducer — N/A (0 rewrites).
- [x] Each surviving external_body confirmed TCB-listed — the 4 frame shims are in `tcb-allowed.md`; upool itself has none.
- [x] No specs weakened — confirmed via `spec_drift.py`.
- [x] Cross-module regression — `make verify` all pass.
- [x] Verification — 0 errors.

### Bug Recording
- [x] bugs.md exists — records "None" for the 8 functions (correct).
- [x] Each bug is a real defect — N/A (no bugs; notes are limitations, correctly classified as non-bugs).
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix — N/A (no bugs).
- [x] No external_body used to mask a code defect — confirmed (upool has none; drop's prior external_body was a VIR/`{:?}` tooling limitation, since eliminated).
- [x] Bug entries include provenance — N/A.

## Spec Quality
Public-API specs are correct, readable, and as complete as the do-not-modify
infrastructure allows. Success-path contracts are strong and directly usable by
callers: `Upool::alloc` ⇒ page-aligned frame, `allocated_frames.contains(uf@)`,
`refcounts[uf@] == 1`; `UserFrame::share` ⇒ aliasing handle (`handle@ == self@`),
well-formed, allocated; `UserFrame::refcount` ⇒ exact count + meaningful `Err`
arm; `address`/`new`/`leak` ⇒ stable-identity getters (`ret@ == self@`/`addr@`).
The `Err(_) => true` arms on `share`/`alloc` and the absence of refcount-delta /
release-exactly-one / failure-atomicity facts are **not** quality defects: they
require a before/after comparison that `phys_view()` (a zero-arg `uninterp spec
fn` in do-not-modify `mod.spec.rs`) cannot express, and they exactly mirror the
already-merged frame.rs / manager.rs convention. Deliberate authorship is evident
(`refcount` states a meaningful `Err` arm precisely where it *is* expressible).

## Caller Coverage
- Covered (all 8 functions, every expressible expectation): **8 / 8 functions.**
  - `new` (identity), `address` (pure getter identity), `leak` (identity; Drop
    suppression intent), `share` (success aliasing + allocated), `refcount`
    (exact count + `Err ⇒ not allocated`), `drop` (`phys_view().inv()` preserved),
    `Upool::new` (total ZST constructor), `alloc` (allocated + `refcount==1`).
- Structurally inexpressible (not missing — require `old(phys_view())`, which the
  do-not-modify single-state `phys_view()` does not provide; documented in
  `bugs.md`, consistent with approved convention):
  - `share` refcount **+1** and failure-atomicity (parent untouched).
  - `alloc` failure ⇒ nothing-allocated.
  - `drop` releases **exactly one** reference; `leak` no-free as an explicit
    before/after frame condition.
  - `new`/`refcount` no-state-change as an explicit `final == old` frame condition.
  Callers' actual proof obligations (CoW in `manager.rs`, alloc in `manager.rs`,
  refcount probe in `vmem.rs`) are dischargeable from the success-path facts that
  *are* stated, so these are non-blocking.

## Proof Completeness
- Remaining admit(): **0** [none — no BLOCKER].
- Remaining external_body not in tcb-allowed.md: **0** [none — no BLOCKER]. upool.rs
  has 0 external_body; the only trust this work introduces is the 4
  `frame::{alloc,free,share,refcount}` shims, all in `tcb-allowed.md`.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES.** Touched code adds
  `external_body` only to `frame.rs::{alloc,free,share,refcount}` (frame.rs:710,
  762, 849, 874), each pre-approved in `tcb-allowed.md`. The module-wide count of
  25 external_body / 9 cfg-gate all live in frame.rs / manager.rs / mod.rs /
  mod.spec.rs (base-branch, pre-existing) — none in upool.rs.
- Note (non-blocking, over-approval not violation): `tcb-allowed.md:66-79` and
  `bugs.md:11-15` still describe `UserFrame::drop` as `external_body`, but the
  current `drop` verifies *without* it. The stale entries grant a trust boundary
  that is no longer used — strictly safe; recommend cleanup.

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **0** (upool.rs), assume_specification: **0**, cfg-gated exec: **0**.
  (Module-wide pre-existing, base-branch: external_body=25, cfg-gate=9 — all TCB-approved, none in upool.)

## AST Consistency
- AST check: **PASS** — 8/8 functions MATCH, 2/2 structs MATCH; 0 `// VERUS REWRITE`;
  exec bodies byte-identical; `View for UserFrame` moved verbatim into `.spec.rs`.

## Verification
- verus: **PASS** — `make verify-kernel MODULE=mm::phys` exit 0; `make verify`
  (bitmap, bump-allocator, kernel, nanvix-slab, sys) all exit 0; `make build`
  up-to-date. `CHEATING_DETECTED` status reflects only pre-approved TCB
  external_body, not verification failures.

## Bug Summary
- Total bugs recorded: **0** (correctly — no code defects in the 8 functions).
- True Bugs: **0.**
- Reconciliation of `bugs.md`: the "drop is external_body" note is now
  **stale/superseded** (drop is non-external); the single-state `phys_view()` note
  remains valid context. No previously-unrecorded bug was discovered during this
  review.

## Issues (highest priority first)
1. **(non-blocking) Stale TCB / bug docs** — `tcb-allowed.md:66-79` and
   `bugs.md:11-15` still list `UserFrame::drop` as `external_body`; the code no
   longer uses it. Over-approval (safe), but should be removed for accuracy.
2. **(non-blocking, by-design) Single-state spec-completeness limits** — `share`
   refcount-+1, `share`/`alloc` failure-atomicity, `drop` release-exactly-one are
   inexpressible without `old(phys_view())`. Not fixable without modifying
   do-not-modify `mod.spec.rs` (forbidden). Matches approved frame.rs/manager.rs
   convention. Recorded in `bugs.md`.
3. **(nit) Redundant-but-defensible conjunct** — `refcount`/`alloc` state both
   `allocated_frames.contains(_)` and `refcounts.contains_key(_)`; the second is
   derivable under `FrameAllocView::wf` but is harmless and aids caller proofs.

### Reconciliation of the two sub-agent verdicts
- **Codex (FAIL / 4 blockers):** all four blockers are the single-state
  spec-completeness gaps above. Codex itself notes they are "understandable under
  single-state `phys_view()` style." They are **not** guardrail violations and are
  **not** fixable without violating the hard "do not modify existing spec/view
  definitions" rule. Re-classified here as non-blocking, by-design limitations.
- **Claude (PASS / 0 blockers):** consistent with the orchestrator's tool-verified
  findings. Adopted.

## Result: PASS
