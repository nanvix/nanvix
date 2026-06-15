# Final Comprehensive Review: phys-frame (gpt-5.3-codex)
## Checklist
- [ ] **Caller Analysis** — Covered **6/15** caller expectations; missing key wrapper guarantees (notably Err-state preservation and several success-transition facts) in `frame.rs` wrapper specs (`alloc`/`alloc_contiguous`/`alloc_range`/`book`/`share`, lines 1328-1517).
- [x] **View Design** — `FrameAllocView` abstraction remains caller-facing and consistent with `Inner::inv()` (`frame.spec.rs:3-8`, `frame.proof.rs:14-79`); no representation leakage.
- [ ] **Specification** — External API contracts are not fully complete for callers: multiple error arms are too weak for stated caller expectations; `free` wrapper is tautological (`ensures true`, `frame.rs:1410-1414`).
- [x] **Proving** — No `admit()`/`assume()` in `frame.rs`, `frame.spec.rs`, `frame.proof.rs`; module verify command exits 0.
- [ ] **Cheating Elimination** — Guardrail counts are clean for `admit/assume`, but AST consistency reports **9 exec mismatches** (`ast_consistency.py` summary: `matched=10 mismatched=9`), requiring explicit deviation justification.
- [ ] **Bug Recording** — `bugs.md` records historical/resolved items, but newly identified spec-coverage/contract-completeness gaps are not recorded there.

## Spec Quality
**FAIL** (strict review).

Evidence:
- Wrapper contracts exist for all top-level APIs, but several are weaker than caller-facing expectations:
  - `alloc`: no explicit `Err => state unchanged`; only `Err => free_frames.is_empty()` (`frame.rs:1328-1336`).
  - `alloc_contiguous`: `Err(_) => true` (`frame.rs:1356-1366`) is under-specified for callers expecting rollback semantics.
  - `book`: `Ok` only states `reserved`; does not state prior-free transition (`frame.rs:1453-1456`).
  - `alloc_range`: no explicit unchanged-on-Err (`frame.rs:1472-1477`).
  - `share`: success does not expose increment relation, only allocated-ness (`frame.rs:1492-1495`).
- `free` wrapper uses tautological postcondition (`ensures true`, `frame.rs:1410-1414`), acceptable for destructor-callability constraints but weak as an API contract.
- Inner contracts are much stronger and generally well-formed (`frame.rs:115-937`), but callers of singleton wrappers cannot directly rely on private `Inner::*` contracts.

## Caller Coverage  (Covered N/Total; Missing: ...)
**Covered 6 / 15** (using caller_analysis success/failure expectations for in-scope public wrappers).

Covered:
1. `is_covered` truth condition (`frame.rs:1432-1437`).
2. `free` destructor constraints (`opens_invariants none`, `no_unwind`) (`frame.rs:1413-1414`).
3. `free_count == phys_view().frames.free_count()` (`frame.rs:1385-1388`).
4. `share` Err condition includes not-allocated or saturated (`frame.rs:1494-1496`).
5. `refcount` Ok returns model count (`frame.rs:1511-1514`).
6. `refcount` Err => not allocated (`frame.rs:1515`).

Missing / not fully covered:
- `alloc` success transition “old free -> now allocated with refcount=1” (only `allocated_frames.contains` exposed).
- `alloc` Err state-preservation.
- `alloc_contiguous` success transition over all `count` frames and Err state-preservation.
- `alloc_range` success “all were free” and Err state-preservation.
- `book` success prior-free premise and Err state-preservation.
- `share` success increment/new-reference fact.

## Proof Completeness  (admit: N w/ locations; external_body not in TCB: N)
- `admit`: **0** (no locations).
- `assume`: **0** (no locations).
- Frame `external_body` not in TCB: **0**.

Frame `external_body` locations:
- `instance` (`frame.rs:1235`, fn line 1242)
- `init` (`frame.rs:1271`, fn line 1282)
- `alloc` (`frame.rs:1327`, fn line 1338)
- `alloc_contiguous` (`frame.rs:1355`, fn line 1368)
- `free` (`frame.rs:1409`, fn line 1416)
- `book` (`frame.rs:1448`, fn line 1458)
- `alloc_range` (`frame.rs:1467`, fn line 1479)

## TCB Compliance  (YES/NO; list any not in approved TCB)
**YES.**

All frame `external_body` functions are listed in `verus-ai-logs/tcb-allowed.md`:
- `frame.rs::instance` (line 7)
- `frame.rs::init` (lines 70/85)
- `frame.rs::alloc` (line 94)
- `frame.rs::alloc_contiguous` (line 96)
- `frame.rs::free` (line 101)
- `frame.rs::book` (line 103)
- `frame.rs::alloc_range` (line 110)

`frame.spec.rs` `assume_specification` declarations (2) are also allow-listed under the frame spec trust-boundary section (`tcb-allowed.md:159-160`).

## Guardrails Compliance  (admit:N assume:N external_body:N assume_specification:N cfg-gated-exec:N)
- `admit`: **0**
- `assume`: **0**
- `external_body`: **7** (`frame.rs`)
- `assume_specification`: **2** (`frame.spec.rs:31,38`)
- `cfg-gated-exec`: **0**

Notes:
- `verus_keep_ghost` sites in `frame.rs`: 28 total; they gate imports, logging, `debug_assert!`, and loop-spec attributes. No exec branch/body duplication detected.

## AST Consistency  (PASS/FAIL with reasoning)
**FAIL.**

Tool evidence:
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py src/kernel/src/mm/phys/frame.rs count`
  - `⚠️  9 mismatched (10 functions match)` (exit 1)
- `... summary`
  - mismatches: `Inner::alloc`, `Inner::alloc_contiguous`, `Inner::alloc_range`, `Inner::book`, `Inner::free`, `Inner::is_covered`, `Inner::refcount`, `Inner::share`, `free_count`.

Additional checks:
- `// VERUS REWRITE` comments in `frame.rs`/`frame.proof.rs`: **none**.
- `cfg(verus_keep_ghost)` review: no verus/non-verus exec divergence via cfg-gated code paths.

## Verification  (PASS/FAIL, error count)
**PASS** for required command; global caveats remain.

Required run:
- `make verify-kernel MODULE=mm::phys 2>&1 | tail -40`
- Exit code: **0**.
- Reported: `status: CHEATING_DETECTED` (global mm::phys counts: `assume=0 external_body=16 admit=7 cfg_gate=13`; admits are outside `frame` scope per output list).

Cross-module/full-kernel note:
- `make verify-kernel 2>&1 | tail -40` also exits **0**, with global `CHEATING_DETECTED` and a trigger-warning note referencing `frame.rs:1186`.

## Bug Summary  (Total recorded N; True bugs with severity)
- Recorded entries reviewed:
  - `verus-ai-logs/nanvix-phys-phys-frame/bugs.md`: **5** entries.
  - `verus-ai-logs/nanvix-phys-phys-frame/cheating-elimination/bugs.md`: **1** entry (manager ghost-token issue; out-of-frame scope).
- True bugs in frame log: **1** correctness bug (top-of-memory representability on 32-bit) — marked resolved.
- Other frame log items are resolved/accepted robustness changes or historical checks.
- New issue found in this final review: wrapper-spec caller-coverage incompleteness (not currently recorded in frame `bugs.md`).

## Issues (highest priority first)
1. **AST consistency blocker**: 9 exec mismatches in `frame.rs` vs baseline (`ast_consistency.py` fail). Requires explicit approved deviation trail per policy before sign-off.
2. **Caller-contract incompleteness**: top-level wrapper error/success guarantees do not fully cover caller_analysis expectations (`alloc`, `alloc_contiguous`, `book`, `alloc_range`, `share`).
3. **Specification weakness**: tautological/very weak wrapper postconditions (notably `free` and `alloc_contiguous` Err arm) reduce caller-proof utility.
4. **Verification hygiene**: Verus trigger warning present in current full verification logs (`frame.rs:1186`), indicating non-clean verifier output.

## Result: FAIL
