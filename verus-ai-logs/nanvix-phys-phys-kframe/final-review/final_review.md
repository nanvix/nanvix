# Final Comprehensive Review: phys-kframe

Module: `mm::phys::kframe` — `src/kernel/src/mm/phys/kframe.rs`
In-scope functions: `KernelFrame::new`, `KernelFrame::drop`, `KernelFrame::base`.
Reviewers: `claude-opus-4.8` (PASS) and `gpt-5.3-codex` (FAIL) — raw reviews in
`final_review.claude.md` / `final_review.gpt5codex.md`. This document consolidates and
adjudicates the split.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` / rust-analyzer LSP output in `find_callers_output.md`
- [x] Caller expectations (success + failure) documented for each pub function (`caller_analysis.md` §Caller Expectations)
- [x] Abstract resource identified — owning handle to one page-sized physical frame; `View::V = int` (physical address)
- [x] Pre-existing specs assessed — inherited from upstream manager verification (`caller_analysis.md` §Pre-existing Specs)

### View Design
- [x] Every field passes the substitution test — single primitive view `int` (physical address); survives a full rewrite (`view_design.md`)
- [x] All caller-observable state represented — every caller uses only the physical address
- [x] No implementation-specific fields — view is the address only; no page-table / mapping detail leaked
- [x] inv() encodes real constraints — `self@ % spec_page_size() == 0` (page alignment), relied on by `into_page_address`
- [x] Mathematical types used — `int` (address abstraction); `base()` returns `FrameAddress` (address keeps usize, per exception)

### Specification
- [x] Every in-scope exec function has requires/ensures — `new`, `base` have requires/ensures; `drop` has `opens_invariants none`/`no_unwind` (coverage report: in-scope trio covered; `clear`/`deref`/`deref_mut` are out of scope)
- [x] Caller coverage — every spec-expressible caller expectation has a corresponding requires/ensures (6/6; see Caller Coverage below)
- [x] View consistency — specs reference `self@`/`base@`/`result@` and maintain `inv()` (`view_design.md` Decision: KEEP `V = int`)
- [x] No tautological ensures — the sole `Err(_) => true` is on the pre-approved `external_body` `new`; it is the maximal **sound** caller-matched trust contract (callers free on failure; a stronger arm is cross-module-deferred and, per the `table::write` precedent in `tcb-allowed.md`, unsound for an `external_body`). Anti-pattern targets hiding *verified* logic — not applicable here. See Issues.
- [x] No subsumed ensures — `result.inv()` on `base` is not derivable from `result@ == self@` alone (needs `self.inv()` precondition propagation); not redundant
- [x] Error paths have meaningful ensures — `new` uses match style; `Err` arm is the faithful trust-boundary contract (see above). `base` is infallible; `drop` returns `()`
- [x] No assume_specification for workspace-internal code — none present in kframe
- [x] vstd searched before any assume_specification — N/A (no assume_specification in kframe)
- [x] Specs written for the caller — `kf@ == base@`, `result@ == self@`, `result.inv()` are exactly what `manager`/`kpage` proofs consume
- [x] Trait obligations satisfied — `Drop` impl uses `opens_invariants none`/`no_unwind` (matches verified `UserFrame::drop`)
- [x] Spec completeness (advisory) — intentional nondeterminism on `new` Err matches caller expectations (callers free `base` themselves)
- [x] Loop invariants — N/A (no loops in the in-scope trio)
- [x] No cheating on module's own functions — admit=0, assume=0, trusted=0; one `external_body` (`new`) is allowlisted
- [x] No specs weakened — `spec_drift.py` vs HEAD: **0 contract drift** (ensures removed: 0, requires added: 0)
- [x] Bug awareness — `bugs.md` reconciled; no fundamentally incorrect code
- [x] Cross-module regression — `make verify` (all mm::phys modules): exit 0, PASS
- [x] Verification — `make verify-kernel MODULE=mm::phys` exit 0; `./z build -- all-kernel` PASS

### Proving
- [x] No specs weakened — `spec_drift.py`: 0 drift
- [x] Zero remaining admit() — kframe: 0
- [x] Zero external_body unless listed in `tcb-allowed.md` — 1 (`KernelFrame::new`), listed
- [x] Zero assume/assume_specification — kframe: 0 (the lone "assume" grep hit is the word "assume" in a doc comment)
- [x] No cfg-gated exec code — the `#[cfg(not(verus_keep_ghost))]` at line 199 gates only an `error!` log; `frame::free` runs in both builds (logging-only, allowed; mirrors verified `UserFrame::drop`)
- [x] Cheating audit — admit=0, external_body=1 (allowlisted), assume=0, cfg-gated exec=0 (logging exception only)
- [x] Any claimed Verus limitation has an isolated reproducer — `new`'s cross-module-token rationale documented in `tcb-allowed.md` + `bugs.md`
- [x] Exec rewrites minimal & semantically equivalent — no `// VERUS REWRITE` comments present (none needed)
- [x] Cross-module regression — `make verify` PASS
- [x] Verification — `make verify-kernel` 0 errors; `./z build` 0 errors/warnings

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code (only the logging `error!` is cfg-gated — allowed)
- [x] Zero external_body unless listed in `tcb-allowed.md` — only `KernelFrame::new`, listed
- [x] AST consistency: zero mismatches — no rewrites; cfg gate is logging-only
- [x] All exec rewrites have VERUS REWRITE comment + minimal reproducer — N/A (no rewrites)
- [x] For each surviving external_body: confirmed in `tcb-allowed.md` — `KernelFrame::new` ✓
- [x] No specs weakened — `spec_drift.py`: 0 drift
- [x] Cross-module regression — `make verify` PASS
- [x] Verification — `make verify-kernel` + `./z build`: 0 errors/warnings

### Bug Recording
- [x] bugs.md exists — present; 1 build-hygiene fix + 1 explanatory note (no correctness bugs)
- [x] Each bug is a real code defect — the recorded entry (duplicate `vstd` import) is a real build-breaking defect; the `external_body` note is correctly classified as NOT a bug
- [x] Each bug entry has What / Why / How Verus Helped / Severity / Suggested Fix — yes
- [x] No external_body used to mask a code defect — `new`'s external_body is a cross-module trust boundary, not a defect mask
- [x] Bug entries include provenance — duplicate-import fix attributed to dual-compilation (`./z build`) phase

## Spec Quality
Public API contracts are correct, minimal, and caller-usable:
- `new(base) -> Result<Self>` — `requires base.inv()`; `ensures Ok(kf) => kf@ == base@ && kf.inv()`.
  Gives the manager exactly `lemma_kernel_alloc_one`'s premise (handle owns the allocated address,
  page-aligned). `Err(_) => true` is the faithful contract for this **pre-approved `external_body`**:
  the only Err-path fact callers want ("frame not consumed/freed") is handled by the caller itself
  (it explicitly frees `base`), and a stronger arm would reference the `mm::virt` identity-map token
  not realized in `mm::phys` — unsound to *assume* on an `external_body` (cf. the `table::write`
  unsoundness precedent recorded in `tcb-allowed.md`).
- `base(&self) -> FrameAddress` — `requires self.inv()`; `ensures result@ == self@ && result.inv()`.
  Pure accessor; gives `kpage` the exact, page-aligned address. Clean, complete, non-tautological.
- `drop(&mut self)` — `opens_invariants none`, `no_unwind`, no abstract postcondition. Identical to
  the **already-verified** `UserFrame::drop`; `frame::free` is best-effort (`ensures true`), so no
  stronger postcondition is expressible or expected. Verified in-body (not `external_body`).
- `inv()` — `self@ % spec_page_size() == 0`: a real, caller-relied constraint (not trivially true).

Readability: every contract is a one-liner over the abstract address with an inline doc comment
explaining the caller that depends on it. Lossy-on-purpose abstraction (`V = int`).

## Caller Coverage
- Covered: **6 / 6** spec-expressible caller expectations.
  - `manager::alloc_kernel_frame` needs `kf@ == frame_addr@` → covered by `new` Ok arm.
  - `manager::alloc_many_kernel_frames` needs `kf@ == base@` for `kernel_addr_set` → covered.
  - `kpage::KernelPage::base` needs address + page-alignment → covered by `base` `result@==self@`, `result.inv()`.
  - `kpage::KernelPage::frame_address` returns `base()` verbatim → covered.
  - `Drop` sites (manager error path, `KernelStack::drop`) need "frees the frame" → `drop` runs `frame::free` (best-effort, matches verified sibling).
  - `new` precondition `base.inv()` always holds (addresses from successful `frame::alloc`) → covered by `requires`.
- Missing: **none that are spec-expressible in-scope.** Two caller *assumptions* are allocator-state
  facts deferred to the trust boundary and intentionally not asserted:
  1. `new` Err ⇒ frame not consumed/freed — compensated by the caller's explicit `frame::free(base)`.
  2. `drop` ⇒ frees-exactly-once — `frame::free` is best-effort; the post-mutation `phys_view()`
     fact is the §8 ghost-token deferral (same as `frame::free`/`UserFrame::drop`).
  Both are documented (`tcb-allowed.md`, `view_design.md`, `caller_analysis.md`) and accepted, not omissions.

## Proof Completeness
- Remaining admit(): **0** — no BLOCKERs.
- Remaining external_body not in `tcb-allowed.md`: **0** — the single `external_body`
  (`KernelFrame::new`, kframe.rs:81/decl:94) IS listed in `tcb-allowed.md`. No BLOCKERs.

## TCB Compliance
- All external_body listed in `tcb-allowed.md`: **YES**. Only `KernelFrame::new`, which is present
  in both the "Allowed `external_body`" and "Cross-module dependencies" sections with full rationale
  (its body calls `mm::virt::identity_map_page`, whose precondition `identity_map_view().inv()` is a
  global token not realized in `mm::phys`). No new trust boundary introduced.

## Guardrails Compliance
(kframe-local counts; the global mm::phys totals admit=24/external_body=17/cfg_gate=15 are entirely
sibling modules — `frame.rs`, `manager.*`, `mod.rs`, `upool.rs` — none in kframe's scope. Confirmed
against `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`, whose only kframe line is
`mm/phys/kframe.rs:94 new: external_body`.)
- admit: **0**
- assume: **0**
- external_body: **1** (`KernelFrame::new`, allowlisted)
- assume_specification: **0**
- cfg-gated exec: **0** (one logging `error!` is cfg-gated — allowed exception, not exec logic)

## AST Consistency
- AST check: **PASS**. No `// VERUS REWRITE` comments exist in `kframe.rs` / `.spec.rs` / `.proof.rs`
  (no exec rewrites were needed). The only `#[cfg(...)]` on exec code is the `error!` log inside
  `drop` (line 199), which removes a logging call under `verus_keep_ghost`; the control flow and the
  `frame::free` call are identical in both builds (semantically equivalent — logging-only, the
  permitted exception). Identical to the verified `UserFrame::drop`.

## Verification
- `make verify-kernel MODULE=mm::phys`: **PASS** (exit 0).
- `make verify` (cross-module regression, all mm::phys modules): **PASS** (exit 0).
- `./z build -- all-kernel` (normal/dual compilation): **PASS** (exit 0).
- `spec_drift.py` vs HEAD: **0 contract drift** (no spec weakened).
- (`status: CHEATING_DETECTED` in the summary reflects sibling-module admit/external_body, not kframe.)

## Bug Summary
- Total bugs recorded: **1** (plus 1 explanatory note that is correctly classified as NOT a bug).
- True Bugs:
  - **Duplicate `vstd::prelude::*` import** (kframe.rs) — severity **cosmetic/build-hygiene**.
    Real defect: broke `./z build -- all-kernel` under `-D warnings` (`unused import`). **Fixed**
    (redundant `use ::vstd::prelude::*;` removed; single top-of-file import remains — confirmed).
    Provenance: dual-compilation/normal-build phase. Has What/Why/How-Verus-Helped/Severity/Fix.
- Not a bug (correctly recorded): `KernelFrame::new` retains `external_body` — a cross-module
  global-token deferral, not a code defect; listed in `tcb-allowed.md`.
- No unrecorded defects discovered during this final review. No surviving verification failure exists
  to classify.

## Issues (highest priority first)
1. **(Non-blocking, dissent from `gpt-5.3-codex`)** `KernelFrame::new` has a vacuous `Err(_) => true`
   arm. *Adjudication:* not a blocker. `new` is a pre-approved `external_body`; callers depend on
   nothing on the Err path (they free `base` themselves — `caller_analysis.md:99-101`); a stronger
   Err postcondition references the `mm::virt` identity-map token not realized in `mm::phys` and,
   for an `external_body`, would be an **unsound assumed axiom** (same class as the `table::write`
   exploit documented in `tcb-allowed.md`). It is the maximal sound, caller-matched trust contract.
   Will be tightenable in-body once `mm::virt`'s identity-map token is realized.
2. **(Non-blocking, dissent from `gpt-5.3-codex`)** `KernelFrame::drop` has no abstract
   "frees-exactly-once" postcondition. *Adjudication:* not a blocker. It is byte-for-byte the
   contract of the **already-verified** `UserFrame::drop`; `frame::free` is best-effort
   (`ensures true`) and the post-mutation `phys_view()` fact is the standard §8 ghost-token deferral.
   No stronger postcondition is soundly expressible in-scope.
3. (Informational) `make`'s summary prints `CHEATING_DETECTED` due to sibling modules; kframe itself
   contributes exactly one allowlisted `external_body`. Not actionable for this target.

## Result: PASS

Zero hard blockers: admit=0, assume=0, the sole `external_body` (`KernelFrame::new`) is in
`tcb-allowed.md`, AST consistent (no rewrites), `spec_drift` clean, and all of
`make verify-kernel` / `make verify` / `./z build` pass. The two concerns raised by the
`gpt-5.3-codex` reviewer are spec-strength observations on a sanctioned trust boundary and on a
contract identical to an already-verified sibling — documented, sound, caller-matched deferrals,
none of which is a task-defined blocker (admit / assume / unlisted-external_body / AST-mismatch /
verification-failure). Consolidated verdict aligns with `claude-opus-4.8`.
