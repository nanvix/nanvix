# Final Comprehensive Review: phys-upool

> Consolidated from two independent sub-agent reviews (one per model):
> `final_review.claude.md` (claude-opus-4.8) and `final_review.gpt.md` (gpt-5.3-codex).
> Both agree on every objective measurement (admit/assume/external_body counts, TCB
> listing, AST consistency, verification result, spec-drift). They diverge **only** on
> the final verdict: claude → PASS (gaps are documented deferrals), codex → FAIL (gaps
> are caller-coverage blockers). This consolidation applies the stated **strict** rule
> ("PASS only if ALL checklist items are checked") and adjudicates **FAIL** on
> spec-completeness / caller-coverage — *not* on any cheating, TCB, or verification gate
> (all of which pass cleanly).

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — rust-analyzer LSP (`find_callers_lsp.out`)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified (reference-counted user physical frames)
- [x] Pre-existing specs assessed (`Upool::alloc` full contract noted)

### View Design
- [x] Every field passes the substitution test (`UserFrame@ = int` address; `Upool@ = FrameAllocView`)
- [x] All caller-observable state represented (`FrameAllocView` carries `free_frames`/`allocated_frames`/`refcounts`)
- [x] No implementation-specific fields (`UserFrame@` exposes only the address)
- [x] inv() encodes real constraints (`UserFrame::inv` = page-aligned, not trivially true)
- [x] Mathematical types used (int/Seq/Set/Map; address keeps usize)

### Specification
- [x] Every in-scope exec function has requires/ensures (`fn_coverage`: all 8 in-scope carry contracts)
- [ ] **Caller coverage: each caller expectation has corresponding requires/ensures** — `share` refcount **+1** and `drop` **release** semantics (both flagged "would break callers" in `caller_analysis.md`) have **no** ensures → UNCHECKED
- [x] View consistency: specs reference View fields (`phys_view().frames.*`) and maintain `inv()`
- [x] No tautological ensures (no `Err(_) => true`)
- [ ] **No subsumed ensures** — `result.inv()` (new/address/leak) and `uf.inv()` (share) are derivable from equality + precondition → UNCHECKED (Low)
- [x] Error paths have meaningful ensures (`share`/`refcount`/`alloc` Err arms are meaningful; `drop` has no Result)
- [x] No assume_specification for workspace-internal code (none present)
- [x] vstd searched before any assume_specification (n/a — none used)
- [ ] **Specs written for the caller (usable directly in caller proofs)** — fork/CoW callers cannot derive the refcount-increment / release facts they depend on → UNCHECKED
- [ ] **Trait obligations satisfied** — `Drop` contract does not capture the "release exactly one reference" semantics callers rely on → UNCHECKED
- [~] Spec completeness (advisory) — gaps do **not** match caller expectations (callers expect +1/release); advisory, but unfavorable
- [x] Loop invariants — no loops in scope (vacuous)
- [x] No cheating on module's own functions (admit=0, assume=0; the 2 `external_body` are the opaque dependency facade, not own logic)
- [x] No specs weakened (`spec_drift.py` → 0 drift)
- [x] Bug awareness (`bugs.md` accurate; no code defect)
- [x] Cross-module regression (`make verify` → exit 0)
- [x] Verification (`make verify-kernel` → 42 verified, 0 errors; `make build` → up-to-date)

### Proving
- [x] No specs weakened (spec drift = 0)
- [x] Zero remaining admit()
- [x] Zero external_body unless listed in tcb-allowed.md (2, both listed)
- [x] Zero assume/assume_specification
- [x] No cfg-gated exec code (exec cfg-gate = 0; the lone cfg guards `error!` logging — allowed)
- [x] Cheating audit: counts reported (below)
- [x] Any claimed Verus limitation has an isolated reproducer (n/a — no rewrites)
- [~] Exec rewrites minimal/equivalent; `// VERUS REWRITE` comments — 0 rewrites; one pre-approved cfg-gating of `drop`'s `error!` log lacks a documenting comment (Minor)
- [x] Cross-module regression (`make verify` → exit 0)
- [x] Verification 0 errors

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code (only the allowed `error!` logging gate)
- [x] Zero external_body unless listed in tcb-allowed.md (2, both listed)
- [x] AST consistency: zero mismatches vs HEAD (8 fns + 2 structs match). Vs the pre-verification baseline, the only diff is the pre-approved cfg-gating of `drop`'s logging line (official Verus cheating-checker counts upool cfg_gate = 0)
- [ ] **All exec rewrites have VERUS REWRITE comment and minimal reproducer** — the `drop` logging cfg-gate is undocumented → UNCHECKED (Minor)
- [x] For each surviving external_body: confirmed listed in tcb-allowed.md (`Upool::new`, `Upool::alloc`)
- [x] No specs weakened (drift = 0)
- [x] Cross-module regression (`make verify` → exit 0)
- [x] Verification 0 errors

### Bug Recording
- [x] bugs.md exists (records "no code bugs")
- [x] Each recorded bug is a real code defect (n/a — none recorded; the spec gaps are correctly classified as a verification limitation, not a defect)
- [x] Each bug entry has required fields (n/a — none)
- [x] No external_body used to mask a code defect (the 2 external_body model the opaque facade/transition, not a defect)
- [x] Bug entries include provenance (n/a — none)

## Spec Quality
Internally consistent, non-tautological, and a **faithful forward** of the frame-layer
dependency contracts — but materially **incomplete** on the module's defining property
(reference counting):

- `UserFrame::new` / `address` / `leak` — address round-trip captured
  (`result@ == self@`/`addr@`). `result.inv()` clauses are subsumed (derivable). The
  no-global-effect facts (`leak`'s suppress-Drop / no-double-free in particular) are not
  in the spec; `leak`'s correctness rests only on the (AST-unchanged) exec `ManuallyDrop`.
- `UserFrame::share` — proves same-frame aliasing (`uf@ == self@`) and that the frame is
  allocated; Err arm is meaningful (`!contains || refcount >= 255`). **Does not** prove
  the refcount **+1** transition — the operation's raison d'être — nor "parent unchanged
  on Err."
- `UserFrame::refcount` — strong and complete for the value read (both arms).
- `UserFrame::drop` — `opens_invariants none` / `no_unwind` only; **no functional
  postcondition** (the RAII "release one reference" guarantee is absent).
- `Upool::new` — `result@.wf()`, complete for its single boot caller.
- `Upool::alloc` — strong, complete two-arm contract (`alloc_one` + empty-pool Err).

**Spec-design "sufficient to reject bugs" test fails for `share`/`drop`:** a `share` that
returns `Ok` without incrementing the refcount, and a `drop` that does nothing (or
double-frees), both satisfy the current contracts. These are exactly the premature-free /
double-free breaks `caller_analysis.md` says "would break callers."

**Root cause (single, documented):** `phys_view()` is a parameter-free `uninterp spec fn`
(global accessor), so `old`/`new` global-partition transitions are inexpressible; the
frame-layer wrappers (`frame::share`/`free`) themselves carry only snapshot/`true`
postconditions. The stronger transitions designed in `view_design.md` §4 were deferred to
a proving-phase ghost token that was **never realized** (`proof.rs` is empty). This is a
documented intentional deferral — not a silent weakening (drift = 0) and not cheating
(admit/assume = 0) — but the deferral was not discharged, so the final artifact remains
incomplete on caller coverage.

## Caller Coverage
- Covered: **5 / 8 functions fully** — `UserFrame::new`, `UserFrame::address`,
  `UserFrame::refcount`, `Upool::new`, `Upool::alloc` (plus the `View for UserFrame`
  address abstraction).
- Partial: **2 / 8** — `UserFrame::leak` (address ✓, suppress-Drop/no-free ✗),
  `UserFrame::share` (same-frame alias ✓, refcount **+1** ✗, parent-unchanged-on-Err ✗).
- Missing: **1 / 8** — `UserFrame::drop` (release-one-reference / last-ref reclaim:
  **no** functional postcondition).
- Missing caller expectations (per `caller_analysis.md`):
  - `share` (Ok): explicit refcount **+1** transition (lines 64–74; "would cause
    premature free").
  - `share` (Err): explicit no-new-ref / parent-frame-unchanged.
  - `drop`: explicit release-exactly-one-reference (lines 18–24, 113).
  - `leak`: explicit suppress-Drop / no-decrement (lines 57–59; double-free risk).
  - `new`/`address`/`refcount`: explicit no-side-effect (`phys_view()` unchanged).

## Proof Completeness
- Remaining admit(): **0** — none in `upool.rs`, `upool.spec.rs`, or `upool.proof.rs`.
- Remaining external_body not in tcb-allowed.md: **0** — the 2 `external_body`
  (`Upool::new` @ upool.rs:246, `Upool::alloc` @ upool.rs:272) are **both listed** in
  `verus-ai-logs/tcb-allowed.md`. (Three further textual `external_body` hits at lines
  57/240/266 are prose comments, not attributes.)

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES** (`Upool::new`, `Upool::alloc`).
  No new trust boundary introduced. Note: `tcb-allowed.md` files these as "verified/
  eliminated when `upool` is verified"; that elimination was **not** achieved (it depends
  on the unrealized ghost-token machinery) — informational, not a blocker, since both
  remain explicitly listed.

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **2** (both TCB-listed),
  assume_specification: **0**, cfg-gated exec: **0** (one `#[cfg(not(verus_keep_ghost))]`
  guards an `error!` log in `drop` — the allowed logging exception; official Verus
  cheating-checker reports upool cfg_gate = 0).

## AST Consistency
- AST check: **PASS** vs HEAD (8 functions + 2 structs match; 0 `// VERUS REWRITE`).
  The only diff vs the pre-verification baseline is the pre-approved cfg-gating of
  `drop`'s logging statement (semantically equivalent; lacks a documenting comment — Minor).

## Verification
- verus: **PASS** — `make verify-kernel MODULE=mm::phys` → exit 0, **42 verified,
  0 errors**. `make verify` (cross-module) → exit 0. `make build` → up-to-date.
  Module-level "CHEATING_DETECTED" reflects out-of-scope `frame.rs`/`manager.rs`/`mod.rs`
  TCB (admit=24, external_body=17, cfg=15) — upool contributes only the 2 TCB-listed
  `external_body`, 0 admit, 0 assume.

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` accurate and reconciled against final code).
- True Bugs: **0**. The exec code is correct (`share` calls `frame::share`, `drop` calls
  `frame::free`); the `share`/`drop` spec gaps are a **verification/spec-completeness
  limitation**, not a code defect — correctly excluded from `bugs.md` per the
  bug-reporting skill. No new bug surfaced during this review.

## Issues (highest priority first)
1. **[BLOCKER — caller coverage] `UserFrame::drop` has no functional postcondition.**
   The RAII "release exactly one reference" guarantee (basis of automatic error-path
   cleanup) is unspecified. A no-op or double-freeing `drop` satisfies the contract.
2. **[BLOCKER — caller coverage] `UserFrame::share` omits the refcount-increment
   transition.** Proves the frame stays allocated but not `F' == F.add_ref(self@)`, nor
   parent-unchanged-on-Err. A `share` that never increments satisfies the spec, defeating
   the premature-free protection fork/CoW callers depend on.
3. **[BLOCKER — caller coverage] `leak` suppress-Drop guarantee unspecified.** No-decrement
   is enforced only by exec `ManuallyDrop`, not by the contract.
4. **[Low] Subsumed `inv()` ensures** in `new`/`address`/`leak`/`share` (derivable from
   equality + precondition).
5. **[Minor] Undocumented pre-approved deviation** — the `drop` logging cfg-gate lacks a
   `// VERUS REWRITE`/deviation comment.
6. **[Informational] TCB "eliminate when module verified" not achieved** for
   `Upool::new`/`Upool::alloc` (sound and listed; pending the deferred ghost token).

Root cause for #1–#3 is singular and documented (`view_design.md` §8, `bugs.md`): the
parameter-free `phys_view()` global and the snapshot-only frame-layer contracts make the
transitions inexpressible at this layer, and the planned proving-phase ghost token was
never realized. They are faithful forwards / intentional deferrals — **zero** of the
defined cheating/TCB blocker categories — but the strict checklist has no carve-out for
documented incompleteness, and the deferral remains undischarged.

## Result: FAIL

**Rationale.** All hard gates are clean — 0 admit, 0 assume, 2 TCB-listed `external_body`,
0 unlisted trust boundaries, AST-consistent, 0 verification errors, 0 spec drift, no code
bugs. This is **not** a cheating, TCB, or verification failure. However, under the strict
criterion ("PASS only if ALL checklist items are checked"), the **Caller coverage**,
**No subsumed ensures**, **Specs written for the caller**, **Trait obligations**, and
**All exec rewrites documented** items cannot be checked: the core reference-counting
guarantees callers depend on — `share` increments the refcount, `drop` releases exactly
one reference, `leak` suppresses the release — are absent from the specs (the
spec-design "sufficient to reject bugs" test fails for `share`/`drop`). These deferrals
were designed and documented but never discharged. Result: **FAIL**, pending realization
of the frame free-function layer contracts and the singleton ghost token that would let
the refcount-transition postconditions be expressed.
