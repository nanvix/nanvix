# Final Comprehensive Review: phys-manager

**Module:** `kernel::mm::phys` • **Target file:** `src/kernel/src/mm/phys/manager.rs`
**Branch:** `verus-ai/phys-manager` • **Date:** 2026-06-15
**Method:** two independent sub-agent reviews (one per allowed model) reconciled below.
- `final_review.claude.md` — model `claude-opus-4.8`
- `final_review.codex.md` — model `gpt-5.3-codex`

Both reviewers independently reached **FAIL**: all *mechanical* gates pass (verification,
admit/assume, TCB, AST, spec-drift), but the realized contracts miss several **spec-design /
verus-constraints quality criteria** (tautological error arms, a vacuous `init` contract, a missing
distinctness guarantee on the user bulk path, and a banned `uninterp` watermark spec fn).

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified (global frame-reservation state via `phys_view()`/`FrameAllocView`)
- [x] Pre-existing specs assessed (do-not-modify `FrameAllocView`/`Inner` views from upstream)

### View Design
- [x] Every field passes the substitution test (uses do-not-modify `FrameAllocView`)
- [x] All caller-observable state represented (no missing fields)
- [x] No implementation-specific fields (only caller-observable state)
- [x] inv() encodes real constraints (not trivially true)
- [x] Mathematical types used (int/Seq/Set/Map; addresses keep usize)

### Specification
- [x] Every in-scope exec function has requires/ensures (`fn_coverage.py` → 7/7 matched)
- [ ] Caller coverage: each caller expectation has corresponding requires/ensures — **FAIL** (success paths strong; multiple failure-path + lifecycle expectations missing — see Caller Coverage)
- [x] View consistency: specs reference `FrameAllocView` fields and maintain `inv()`
- [ ] No tautological ensures — **FAIL**: `Err(_) => true` on `alloc_user_frame` (`:264`), `check_user_watermark` (`:303`), `alloc_kernel_frame` (`:349`) (anti-pattern #8)
- [ ] No subsumed ensures — **FAIL**: `init` ensures (`:104–105`) merely restate its `requires` (`:101–102`); contract is a vacuous no-op
- [ ] Error paths have meaningful ensures — **FAIL**: three `Err(_) => true` arms (anti-pattern #5, One-Sided Error); bulk Err arms give only `len==0`, not "no-leak"
- [x] No assume_specification for workspace-internal code (0 in module)
- [x] vstd searched before any assume_specification (none used)
- [x] Specs written for the caller (success-path facts are caller-usable)
- [x] Trait obligations satisfied (no trait impls in scope beyond derives)
- [ ] Spec completeness (advisory) — **GAPS**: missing distinctness/no-double-alloc (H1) and fresh-ownership/refcount=1 (M2)
- [x] Loop invariants — N/A (all six bodies are `external_body`; no verified loops in scope)
- [x] No cheating on module's own functions: `admit=0`, `assume=0`, `external_body=6` (all pre-approved), `trusted=0`
- [x] No specs weakened: `spec_drift.py ... --before HEAD` → no drift; vs pre-verus base only additions (0 ensures removed)
- [x] Bug awareness: no fundamentally incorrect code; `bugs.md` "None" is accurate
- [x] Cross-module regression: module verify (`MODULE=mm::phys`) exit 0; `kernel::all` passing at branch HEAD (git log)
- [ ] Verification: `make verify-kernel` PASS (0 errors); `make build` not separately run this review — **partial**

### Proving
- [x] No specs weakened (`spec_drift.py` clean)
- [x] Zero remaining admit()
- [x] Zero external_body unless listed in `tcb-allowed.md` — all 6 are listed
- [x] Zero assume/assume_specification
- [x] No cfg-gated exec code (only `include!` ghost-gating + `allow` attrs)
- [x] Cheating audit: counts + locations reported (see Guardrails)
- [x] Claimed Verus limitations isolated (static-mut + `error!`/`warn!` macro limits, documented)
- [x] Exec rewrites minimal/equivalent — N/A, **zero** `// VERUS REWRITE` in module
- [x] Cross-module regression (module verify PASS; kernel::all PASS at HEAD)
- [ ] Verification: `make verify-kernel` 0 errors **and** `make build` 0 warnings — build not re-run — **partial**

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code
- [x] Zero external_body unless listed in `tcb-allowed.md` (all 6 listed)
- [x] AST consistency: zero mismatches (`ast_consistency.py` → matched=7 struct=1, 0 mismatched)
- [x] All exec rewrites have VERUS REWRITE comment + reproducer — N/A (none)
- [x] Each surviving external_body confirmed in `tcb-allowed.md`
- [x] No specs weakened (`spec_drift.py` clean)
- [x] Cross-module regression (module verify PASS)
- [ ] Verification: `make verify-kernel` + `make build` — 0 errors, 0 warnings — build not re-run — **partial**

### Bug Recording
- [x] bugs.md exists; correctly records "None"
- [x] Each (n/a) bug would be a real defect — none found
- [x] Bug entry format — n/a (no bugs)
- [x] No external_body used to mask a code defect (shims justified by genuine Verus limits)
- [x] Bug entries include provenance — n/a

## Spec Quality
The six methods are `#[verus_verify(external_body)]` shims with `#[verus_spec]` contracts over the
do-not-modify `phys_view()`/`FrameAllocView`. Because the carrier is the global `phys_view()` with **no
`old(phys_view())`**, contracts are monotone post-state facts (a pre-approved architectural choice).
**Success-path facts are good and caller-usable** (allocated-membership + page alignment for single
allocs; exact `count` + per-frame membership + watermark for bulk user; exact `count` + contiguity for
bulk kernel — `kernel_frames_contiguous`). The supporting lemmas (`lemma_watermark_monotone`,
`lemma_contiguous_run_distinct`) are fully discharged (no admit/assume).

**Quality deficiencies (why both reviewers FAIL it):**
- **Tautological error arms** `Err(_) => true` on `alloc_user_frame`, `check_user_watermark`,
  `alloc_kernel_frame` (anti-patterns #5/#8). Callers expect "nothing allocated / allocator untouched";
  not captured. (For value-less single-frame `Err` with no `old(phys_view())`, this is partly inherent
  to the carrier, but the guarantee is still absent.)
- **`init` contract is vacuous** — ensures (`:104–105`) restate requires (`:101–102`); the singleton
  lifecycle / double-init `Err` semantics callers rely on (`caller_analysis.md` L53–61) are unmodeled.
- **Missing distinctness on the user bulk path** — `alloc_many_user_frames` Ok arm (`:188–193`) has no
  no-double-allocation clause; the spec is satisfiable by an impl returning `count` aliases of one frame
  (kernel bulk path is fine via contiguity).
- **`spec_kernel_watermark()` is `uninterp`** (`manager.spec.rs:35`) — explicitly **banned** by
  verus-constraints (line 113) and spec-design anti-pattern #12; it is a standalone spec fn, not a
  `View::view()` of an external_body type, so the exception does not apply.
- **Missing fresh-ownership** (`refcount==1`, `addr ∈ old free_frames`) on all alloc success arms.
- **Bulk Err arms** capture only `final(frames)@.len()==0`, not allocator no-leak/rollback.

## Caller Coverage
Reviewers differ in strictness on the count; consensus: **success paths well covered, failure +
lifecycle paths weak.**
- Claude: **6/14 fully covered**, 4 partial, 4 missing.
- Codex: **3/14 fully covered**, 6 partial, 5 missing.
- Consolidated **Covered: 6 / 14** (best case), **Total: 14**.
- Missing / partial (union):
  - `init` success lifecycle guarantee (singleton established for later `get_mut`) — **missing**
  - `init` error condition (already-initialized → InvalidArgument) — **missing**
  - `alloc_user_frame` failure "no allocation / no leak" — **missing** (`Err(_)=>true`)
  - `alloc_kernel_frame` failure "no leak" incl. wrap-failure path — **missing** (`Err(_)=>true`)
  - `check_user_watermark` error classification (overflow vs breach) + converse `Err => !policy` — **missing**
  - `alloc_user_frame` / `alloc_kernel_frame` fresh/exclusive ownership (refcount=1) — **partial**
  - `alloc_many_*` capacity-check / InvalidArgument failure behavior — **partial**
  - `alloc_many_*` failure all-or-nothing **no-leak** (beyond `len==0`) — **partial**
  - `alloc_many_user_frames` success **distinctness** — **missing** (admits aliasing impl)

## Proof Completeness
- Remaining admit(): **0** — none. (no BLOCKER)
- Remaining external_body not in `tcb-allowed.md`: **0** — all 6 (`manager.rs:107,198,267,306,352,409`)
  are listed in the pre-approved TCB. (no BLOCKER)

## TCB Compliance
- All external_body listed in `tcb-allowed.md`: **YES**. The six `PhysMemoryManager` methods are
  enumerated in the "Allowed `external_body` — `PhysMemoryManager`" section. No unapproved trust
  boundary introduced.

## Guardrails Compliance
(counts over `manager.rs`, `manager.spec.rs`, `manager.proof.rs`)
- admit: **0**, assume: **0**, external_body: **6** (all pre-approved; `:98,177,249,292,336,388`),
  assume_specification: **0**, cfg-gated exec: **0** (only `#[cfg(verus_keep_ghost)] include!` at
  `:9,11` and `cfg_attr(... allow ...)` at `:97,291` — not exec-logic forks).

## AST Consistency
- AST check: **PASS** — `ast_consistency.py` → matched=7 fns + 1 struct, mismatched=0, missing=0,
  extra=0. Zero `// VERUS REWRITE` comments in the module (no exec rewrites to audit).

## Verification
- verus: **PASS** — `make verify-kernel MODULE=mm::phys` exit 0, **0 errors** (canonical run + both
  sub-agents independently confirmed). `kernel::all` reported passing at branch HEAD (git log).

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` = "None", reconciled accurate).
- True Bugs: **0**. Cleanup/rollback paths are leak-free; the six shims are justified by genuine Verus
  front-end limitations (no `static mut` model; `error!`/`warn!` macros unsupported), not by masking a
  defect. No new code defects discovered during this review — all new findings are
  spec-completeness/quality issues, correctly **not** logged as bugs.

## Issues (highest priority first)
1. **[BLOCKER — verus-constraints]** `spec_kernel_watermark()` is `uninterp` (`manager.spec.rs:35`).
   `uninterp spec fn` is banned (verus-constraints L113; spec-design #12). Must be given a concrete
   definition (e.g. tied to `config::kernel::KERNEL_WATERMARK`) — it is not a `View::view()` exception.
2. **[Quality]** Tautological `Err(_) => true` on `alloc_user_frame` (`:264`), `check_user_watermark`
   (`:303`), `alloc_kernel_frame` (`:349`) — anti-patterns #5/#8; add meaningful failure guarantees.
3. **[Quality]** `init` contract vacuous (`:101–106`) — restates precondition; does not model the
   singleton lifecycle / double-init `Err` callers depend on.
4. **[Quality]** `alloc_many_user_frames` Ok arm (`:188–193`) lacks distinctness/no-double-allocation —
   spec admits an aliasing implementation.
5. **[Quality]** Missing fresh-ownership (`refcount==1`, drawn from old free set) on all alloc success
   arms; bulk Err arms capture only `len==0`, not allocator no-leak.
6. **[Coverage]** Failure-path + lifecycle caller expectations largely uncovered (see Caller Coverage).

## Result: **FAIL**

All mechanical/cheating gates pass (verification 0 errors; admit=0; assume=0; external_body=6 all in the
pre-approved TCB; AST consistent; no spec weakening; no bugs), so there are **no hard cheating/TCB/AST
blockers**. However, the strict bar requires **every** checklist item checked, and multiple
specification-quality and caller-coverage items are unmet — most critically the **banned `uninterp`
watermark spec fn** (a direct verus-constraints violation), the **tautological `Err(_) => true`** error
arms, the **vacuous `init` contract**, and the **missing distinctness** guarantee on the user bulk path.
Per "PASS only if ALL checklist items are checked," the verdict is **FAIL**.
