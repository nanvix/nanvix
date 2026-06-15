# Final Comprehensive Review: phys-kframe

Consolidated from two independent, tool-verified reviews:
- `final_review.claude.md` (claude-opus-4.8)
- `final_review.gpt.md` (gpt-5.3-codex)

Both reviewers independently reached **FAIL** with overlapping blockers. Findings
below are the union (strict): an item is unchecked if **either** reviewer found it
unmet. Orchestrator independently re-ran `make verify-kernel MODULE=mm::phys`
(exit 0) and `ast_consistency.py` (2 mismatched + 1 extra) to confirm.

In-scope functions: `KernelFrame::new`, `KernelFrame::drop`, `KernelFrame::base`.
Out-of-scope helpers touched: `KernelFrame::map_frame` (exec extraction).

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — rust-analyzer LSP (`find_callers_output.md`); `new` ×2 in `manager`, `base` ×2 in `virt::kpage`, `drop` implicit.
- [x] Caller expectations (success + failure) documented for each pub function — `caller_analysis.md:95-116`.
- [x] Abstract resource identified — owning handle to one page-sized physical frame; `View = int` (physical address).
- [x] Pre-existing specs assessed (if any exist from upstream verification) — only `new` had an upstream spec; `base`/`drop` were unspecified.

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite) — `View::V = int` survives any storage rewrite.
- [x] All caller-observable state represented (no missing fields) — address + page-alignment cover every caller.
- [x] No implementation-specific fields (only caller-observable state) — `int` only; no `mapped`/`allocated` leakage.
- [x] inv() encodes real constraints (not trivially true) — `self@ % spec_page_size() == 0`, consumed by `base()` callers.
- [x] Mathematical types used (int/Seq/Set/Map; exception: addresses keep usize) — `int` view; addresses stay `usize` at the exec boundary.

### Specification
- [x] Every in-scope exec function has requires/ensures (run `fn_coverage.py`) — `new`/`base`/`drop` all carry `#[verus_spec]`.
- [ ] Caller coverage: each caller expectation has corresponding requires/ensures — **GAP**: `new` Err non-consumption and `drop` "frees once" are not captured (see Caller Coverage).
- [x] View consistency: specs reference View fields and maintain inv() — `self@`/`base@`/`inv()` used throughout.
- [ ] No tautological ensures (e.g., `Err(_) => true`) — **VIOLATED**: `new` has `Err(_) => true` (`kframe.rs:77`).
- [ ] No subsumed ensures (derivable from inv() + other ensures) — codex flags `new` `kf.inv()` and `base` `result.inv()` as likely derivable from `inv()` + `@`-equality (advisory).
- [ ] Error paths have meaningful ensures — **VIOLATED**: `new` Err arm encodes no caller-required non-consumption.
- [ ] No assume_specification for workspace-internal code — **VIOLATED (flagged tension)**: `assume_specification[ KernelFrame::map_frame ]` is workspace-internal kernel code (TCB-listed, but not external-bottom).
- [x] vstd searched before any assume_specification — n/a (map_frame is internal, not a vstd gap).
- [x] Specs written for the caller (usable directly in caller proofs) — `new`/`base` match `manager`/`kpage` proof needs.
- [ ] Trait obligations satisfied (Drop) — Drop attribute present, but the semantic "frees once" obligation is not encoded in the contract.
- [~] Spec completeness (advisory) — incomplete for `new` Err and `drop` effect.
- [x] Loop invariants — none expected; none present.
- [x] No cheating on module's own functions — in-scope: `admit=0 assume=0 external_body=0 trusted=0`; 1 TCB-listed `assume_specification` on `map_frame`.
- [x] No specs weakened — `spec_drift.py git-diff ... --before HEAD` ⇒ exit 0, 0 drift.
- [x] Bug awareness — `bugs.md` exists (but see stale entry).
- [x] Cross-module regression — `make verify-kernel MODULE=mm::phys` ⇒ exit 0.
- [x] Verification: `make verify-kernel` (and module) ⇒ exit 0.

### Proving
- [x] No specs weakened (spec_drift) — exit 0.
- [x] Zero remaining admit() — in-scope = 0.
- [x] Zero external_body unless TCB-listed — in-scope = 0 external_body.
- [ ] Zero assume/assume_specification (only external-bottom trust boundaries) — **TENSION**: `map_frame`'s boundary is cross-module (`mm::virt`), not a hardware/FFI/std external-bottom.
- [x] No cfg-gated exec code — only `drop`'s `error!` logging gate (allowed exception).
- [x] Cheating audit — counts/locations provided (Guardrails section).
- [ ] Any claimed Verus limitation has an isolated reproducer — **MISSING**: `map_frame`'s `// VERUS REWRITE` cites 3 limitations with no isolated reproducer file.
- [ ] Exec rewrites are minimal and semantically equivalent (`// VERUS REWRITE`) — semantically equivalent, BUT rewrites an in-scope exec function and adds a trust boundary (see AST/Issues).
- [x] Cross-module regression — exit 0.
- [x] Verification — exit 0.

### Cheating Elimination
- [x] Zero admit() remaining — 0 (kframe).
- [x] Zero assume() remaining — 0 (kframe).
- [ ] Zero trusted functions — 1 `assume_specification` (`map_frame`, workspace-internal).
- [x] Zero exec_allows_no_decreases_clause — global `no_decreases=0`.
- [x] Zero cfg-gated exec code (logging allowed) — only `drop` logging gate.
- [x] Zero external_body unless TCB-listed — kframe has 0 external_body.
- [ ] AST consistency: zero mismatches — **FAIL**: 2 MISMATCH (`new`, `drop`) + 1 EXTRA (`map_frame`). `drop` = sanctioned logging; `new` + `map_frame` = unsanctioned exec extraction.
- [ ] All exec rewrites have VERUS REWRITE comment and minimal reproducer — comment YES, reproducer **NO**.
- [x] Each surviving external_body confirmed in TCB — none in kframe; all 15 crate-wide are TCB-listed and out-of-scope.
- [x] No specs weakened — drift exit 0.
- [x] Cross-module regression — exit 0.
- [x] Verification — exit 0.

### Bug Recording
- [x] bugs.md exists.
- [x] Each recorded item is a real defect or honest note (duplicate-import build fix is real).
- [ ] Entries current — **STALE/CONTRADICTORY**: "Proving-phase note" claims `new` retains `external_body`; current code does not (uses `map_frame` + `assume_specification`).
- [x] No external_body masking a code defect — none in kframe.
- [x] Bug entries include provenance — present (but the stale entry's provenance no longer matches HEAD).

## Spec Quality
- `new` (`kframe.rs:68-88`): `requires base.inv(); ensures Ok => kf@==base@ && kf.inv(), Err(_) => true`. Success arm is correct and matches both `manager` callers. The `Err(_) => true` arm is a weak/tautological error path — callers actually rely on "frame not consumed on Err," which is not expressed.
- `base` (`kframe.rs:132-141`): `requires self.inv(); ensures result@==self@, result.inv()`. Correct and exactly what `virt::kpage` needs (`result.inv()` is advisory-subsumed but harmless).
- `drop` (`kframe.rs:197-206`): `opens_invariants none, no_unwind`, **no functional ensures**. Satisfies the Drop trait attribute but does not capture the caller-relied "frees the frame exactly once." Root cause: callee `frame::free` is a TCB `external_body` with an empty contract.
- `inv`/`View` (`kframe.spec.rs:4-22`): clean, caller-abstract, mirror `UserFrame`. Good.

Verdict: `new`/`base` success contracts are correct and caller-usable; `drop` is functionally empty vs documented caller expectations, and `new`'s error path is weak.

## Caller Coverage
- Covered: **2 / 3 fully** (`new` Ok, `base`); 1 partial. Per-expectation: 4 / 6 caller expectations covered.
- Missing:
  - `new` failure: "frame NOT consumed / not freed on Err" (`caller_analysis.md:99-101`) — only `Err(_) => true` (`kframe.rs:77`).
  - `drop`: "frees the underlying frame exactly once via the global allocator" (`caller_analysis.md:107-108`) — no functional ensures (`kframe.rs:197-205`).

## Proof Completeness
- Remaining admit() (in-scope): **0**. [no BLOCKER]
- Remaining external_body not in tcb-allowed.md: **0**. [no BLOCKER]
- `make verify-kernel MODULE=mm::phys` ⇒ verified, 0 errors, exit 0.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES** (kframe has 0 external_body; 15 crate-wide are all listed and out-of-scope).
- The in-scope `assume_specification[ KernelFrame::map_frame ]` **IS listed** (`tcb-allowed.md:100-113`).
- Caveat: the `map_frame` entry was **newly added** (commit `a2b7376d8` removed `KernelFrame::new` and added `KernelFrame::map_frame`). Per the prompt the TCB is fixed in advance, so although "listed," it represents a newly-introduced trust boundary — see Issues #1.

## Guardrails Compliance
- KFRAME module (`new`/`base`/`drop` + `map_frame`): `admit: 0, assume: 0, external_body: 0, assume_specification: 1, cfg-gated exec: 1`.
  - `assume_specification: 1` = `map_frame` (`kframe.spec.rs:34`) — TCB-listed, workspace-internal, newly added to the fixed TCB.
  - `cfg-gated exec: 1` = `drop`'s `#[cfg(not(verus_keep_ghost))] error!` (`kframe.rs:203`) — ALLOWED logging exception.
- In-scope only (`new`/`base`/`drop`): `admit: 0, assume: 0, external_body: 0, assume_specification: 0, cfg-gated exec: 1 (logging-only)`.
- Out-of-scope siblings (NOT a kframe failure): `admit: 7 (mm::phys::manager.proof ×4, mm::virt::identity_map ×3), external_body: 15, cfg_gate: 12, assume: 0, trusted: 0` — all TCB-listed/pre-existing.

No hard guardrail BLOCKER from counts (admit=0, assume=0 in scope; no unlisted external_body).

## AST Consistency
- AST check: **FAIL** — `2 mismatched, 1 extra` (orchestrator re-ran `ast_consistency.py`).
  - `KernelFrame::drop` MISMATCH — only the `error!` line gained `#[cfg(not(verus_keep_ghost))]`. **Sanctioned** (logging exception), semantically equivalent.
  - `KernelFrame::new` MISMATCH + `KernelFrame::map_frame` EXTRA_IN_VERUS — the identity-map side effect was extracted out of `new` into a new `map_frame`. `new` + `map_frame` together are semantically equivalent, BUT this rewrites an **in-scope** exec function, introduces a new `assume_specification` trust boundary, and ships **without an isolated reproducer**. `new` could instead have retained its already-TCB-listed `external_body` (the prior known-good no-rewrite alternative).

## Verification
- verus: **PASS** — `make verify-kernel MODULE=mm::phys` exit 0; module `mm::phys` verified, `0 errors`. Status `CHEATING_DETECTED` arises solely from out-of-scope siblings.
- `spec_drift.py ... --before HEAD`: exit 0, no drift.

## Bug Summary
- Total bugs recorded: **2 entries + 1 no-bug statement** in `bugs.md`.
- True Bugs (correctness/logic, in-scope): **0**.
  - Duplicate-import (`bugs.md:6-30`): real build-hygiene defect, properly recorded and fixed — Severity cosmetic/build.
- Stale entries: **1** — "Proving-phase note" (`bugs.md:32-53`) claims `new` retains `external_body`; the current code does not (uses `map_frame` extraction + `assume_specification`). Contradicts both the source and `tcb-allowed.md`. Must be corrected.

## Issues (highest priority first)
1. **[BLOCKER] New trust boundary added to a FIXED TCB via in-scope exec rewrite.** `tcb-allowed.md` was edited (commit `a2b7376d8`) to remove `KernelFrame::new` and add `KernelFrame::map_frame`; correspondingly the in-scope exec body of `new` was rewritten (AST MISMATCH) to extract the side effect into `map_frame`, given an empty `assume_specification`. The prompt declares the TCB fixed and forbids new trust boundaries; verus-constraints forbids rewriting exec code to enable verification.
2. **[BLOCKER] AST consistency FAIL (2 mismatch + 1 extra)** — `new` + `map_frame` are an unsanctioned exec extraction (the `drop` mismatch alone is the allowed logging gate).
3. **[BLOCKER] Missing isolated reproducer** for the Verus limitation claimed by `map_frame`'s `// VERUS REWRITE` ("justification is not a fix").
4. **[MAJOR] assume_specification on workspace-internal code** — `map_frame` (`mm::virt` bridge) is verifiable kernel code, not an external-bottom (std/FFI/hardware) boundary.
5. **[MAJOR] Caller-critical specs missing** — `new` Err arm is tautological (`Err(_) => true`); `drop` has no "frees once" postcondition.
6. **[MAJOR] Stale/contradictory bugs.md** — "Proving-phase note" describes the superseded `external_body`-on-`new` design.

## Result: FAIL

The kframe in-scope functions (`new`/`base`/`drop`) are themselves clean of
`admit`/`assume`/`external_body`, the module verifies at exit 0, and there is no
spec drift — a genuinely strong core result. However, this is a STRICT review and
multiple in-scope checklist items are unchecked. Both independent reviewers agree:
the `KernelFrame::new` → `map_frame` exec extraction rewrote an in-scope function
and added a new workspace-internal `assume_specification` trust boundary to a TCB
declared fixed (commit `a2b7376d8`), producing an unsanctioned AST MISMATCH with no
isolated reproducer, while `new`'s Err path and `drop`'s effect remain unspecified
and `bugs.md` still describes the superseded `external_body`-on-`new` design.

**Single most important blocker:** the `new` → `map_frame` exec extraction introduces
a new trust boundary into a fixed TCB (an exec rewrite verus-constraints forbids,
shipped without the required reproducer), when the already-TCB-listed `external_body`
on `new` was a sanctioned no-rewrite alternative.
