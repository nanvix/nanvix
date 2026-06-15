# Final Comprehensive Review: phys-upool

> Consolidated from two independent sub-agent reviews (one per model):
> - `final_review.claude.md` (claude-opus-4.8)
> - `final_review.gpt-5.3-codex.md` (gpt-5.3-codex)
>
> Both reviewers independently reached **Result: FAIL** under the strict
> "all checklist items must pass" rubric, with the **single** failing axis being
> caller-coverage / spec-sufficiency for the reference-count discipline of
> `UserFrame::share`, `UserFrame::drop`, and `UserFrame::leak`. Both also agree
> there are **NO hard blockers** (admit=0, assume=0, every `external_body` is
> TCB-approved, verus passes, AST has no semantic mismatch, no spec drift, no code
> bugs). The two reviews differ only in how they *count* caller coverage
> (claude: 5/8 fully + 3/8 partial; codex: 8/15 expectations) and in labeling the
> coverage gap "P0 blocker" (codex) vs "completeness gap, not a blocker" (claude).
> Per the task's explicit blocker definition (admit>0, assume>0, unapproved
> `external_body`), the gap is **not** a hard blocker — but it is an unchecked
> checklist item, so the strict result is **FAIL**.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.out` (rust-analyzer LSP, intra-crate); 8 exec fns + 2 types.
- [x] Caller expectations (success + failure) documented for each pub function — `caller_analysis.md` per-function success/error expectations.
- [x] Abstract resource identified — reference-counted owning handles (`UserFrame`) + thin pool facade (`Upool`).
- [x] Pre-existing specs assessed — `Upool::alloc` (full contract) and the two `external_body` facades assessed.

### View Design
- [x] Every field passes the substitution test — `UserFrame@ = int` (addr), `Upool@ = FrameAllocView`.
- [x] All caller-observable state represented — handle exposes only the frame address (what every caller reasons about).
- [x] No implementation-specific fields — no handle internals leak into the view.
- [x] inv() encodes real constraints — `self@ % spec_page_size() == 0` (page-alignment), non-trivial.
- [x] Mathematical types used — `int` / `FrameAllocView` (addresses keep `usize` via `FrameAddress`).

### Specification
- [x] Every in-scope exec function has requires/ensures — all 8 carry `#[verus_spec]`.
- [ ] **Caller coverage** — `share` (+1 increment), `drop` (release), `leak` (no-release) refcount-discipline expectations are **not realized** as ensures (documented §8 deferral). **FAILS this item.**
- [x] View consistency — specs reference `self@` / `phys_view().frames` / `FrameAllocView` and maintain `inv()`.
- [x] No tautological ensures — no `Err(_) => true` among expressed facts.
- [x] No subsumed ensures — `result.inv()` in `new`/`address`/`leak` is borderline-derivable (codex flagged); kept as a minor advisory, not a failure.
- [x] Error paths have meaningful ensures — `refcount`/`alloc`/`share` Err arms state real failure causes (not trivial); `share` Err omits frame-unchanged (deferred — captured under Caller coverage).
- [x] No assume_specification for workspace-internal code — 0.
- [x] vstd searched before any assume_specification — N/A (0 `assume_specification`).
- [ ] **Specs written for the caller (usable directly in caller proofs)** — `drop` has **no functional ensures**, so a caller cannot prove the release effect; `leak`/`share` discipline likewise not provable. **FAILS this item** (same root cause as Caller coverage).
- [x] Trait obligations satisfied — `View for UserFrame` correct; `View for Upool` uninterp (TCB consequence). Note: `Drop`'s *semantic* release contract is deferred (tied to the coverage gap above).
- [ ] **Spec completeness (advisory)** — `share`/`drop`/`leak` contracts are not "sufficient to reject bugs" (a non-incrementing share, a leaking/double-freeing drop, or a freeing leak all satisfy the current specs). Intentional, sound deferral — but **unchecked** under strict reading.
- [x] Loop invariants — no loops in any in-scope function.
- [x] No cheating on module's own functions — `admit=0`, `assume=0`, `trusted=0`; only 2 TCB `external_body`.
- [x] No specs weakened — `spec_drift.py git-diff … --before HEAD` ⇒ exit 0, no contract drift; do-not-touch defs unmodified.
- [x] Bug awareness — `bugs.md` present; no incorrect code found.
- [x] Cross-module regression — `make verify` (all crates + kernel) ⇒ exit 0, 0 verification errors.
- [x] Verification — `make verify-kernel MODULE=mm::phys` exit 0, 0 errors; `make verify` exit 0; kernel compiles under the verus build (`make build` is a no-op alias).

### Proving
- [x] No specs weakened — `spec_drift.py` clean (exit 0).
- [x] Zero remaining admit() — 0 in upool (`upool.rs`/`.spec.rs`/`.proof.rs`).
- [x] Zero external_body unless listed in `tcb-allowed.md` — exactly 2 (`Upool::new`, `Upool::alloc`), both listed.
- [x] Zero assume/assume_specification — 0.
- [x] No cfg-gated exec code — the only `cfg(not(verus_keep_ghost))` (upool.rs:207) gates a logging macro (`error!`) — sanctioned; 0 semantic exec gating.
- [x] Cheating audit — admit=0, external_body=2 (TCB), assume=0, cfg-gated exec=0 semantic. Locations below.
- [x] Any claimed Verus limitation has an isolated reproducer — the 0-arg `uninterp phys_view()` transition limitation is documented (view_design §8, tcb-allowed.md:106-124).
- [x] Exec rewrites minimal/semantically equivalent — there are **no** `// VERUS REWRITE` in upool (0).
- [x] Cross-module regression — `make verify` exit 0.
- [x] Verification — `make verify-kernel` / `make verify` exit 0; 0 in-scope warnings.

### Cheating Elimination
- [x] Zero admit() remaining — 0 in upool.
- [x] Zero assume() remaining — 0.
- [x] Zero trusted functions — 0.
- [x] Zero exec_allows_no_decreases_clause — 0.
- [x] Zero cfg-gated exec code — only the logging-macro gate (allowed).
- [x] Zero external_body unless listed in `tcb-allowed.md` — 2, both listed.
- [x] AST consistency: zero semantic mismatches — checker reports 1 mismatch on `UserFrame::drop`, which is purely the stripped `#[cfg(not(verus_keep_ghost))]` on the `error!` logging macro (pre-approved; macro preserved identically). No semantic divergence.
- [x] All exec rewrites have VERUS REWRITE comment + minimal reproducer — N/A (0 rewrites).
- [x] For each surviving external_body: confirmed listed in `tcb-allowed.md` — `Upool::new` (tcb-allowed.md:106-117), `Upool::alloc` (tcb-allowed.md:118-124).
- [x] No specs weakened — `spec_drift.py` clean.
- [x] Cross-module regression — `make verify` exit 0.
- [x] Verification — exit 0, 0 in-scope warnings.

### Bug Recording
- [x] bugs.md exists — records "No code bugs found" + one intentional deferred-modeling note.
- [x] Each bug is a real code defect — N/A (no bugs recorded; the single note is a deferral, not a defect).
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix — N/A (no bug entries).
- [x] No external_body used to mask a code defect — the 2 `external_body` are design-forced facade boundaries (verified clean by reconciliation), not defect masks.
- [x] Bug entries include provenance — the deferral note attributes provenance (specification phase, deferred to proving).

## Spec Quality
The public-API contracts split cleanly into two tiers.

**Strong / complete:**
- `Upool::new` — `ensures result@.wf()`: exactly the one fact the boot caller needs (`external_body`, TCB).
- `Upool::alloc` — full `match`: Ok `free_frames.contains(uf@)` ∧ `final@ == old@.alloc_one(uf@)`; Err `final@ == old@` ∧ `old@.free_count() == 0`; `wf()` preserved. Both arms meaningful incl. exhaustion + state preservation (`external_body`, TCB).
- `UserFrame::new` / `address` — faithful thin-wrapper round-trip (`result@ == addr@` / `self@`), infallible by type.
- `UserFrame::refcount` — both Ok/Err arms meaningful and bidirectional; directly usable by the CoW `== 1` probe.

**Under-specified (the core finding):** `UserFrame::share`, `UserFrame::drop`,
`UserFrame::leak` do not realize the reference-count discipline that
`caller_analysis.md` identifies as the module's central safety property:
- `share` Ok arm omits the `+1` increment (`add_ref`) and the `refcounts[self@] < 255` headroom; Err arm omits the self/frame-unchanged condition.
- `drop` carries **no functional ensures** (only `opens_invariants none`, `no_unwind`) — the `release` transition is absent.
- `leak` states only `result@ == self@` — the defining *no-release* guarantee is absent.

Root cause (documented, sound, technically forced): `phys_view()` is a 0-argument
`uninterp` constant, so an `old(phys_view())` → `phys_view()` transition is
inexpressible (both sides are the same logic constant ⇒ any such clause is
tautological). The genuine transition must thread through the §8 ghost token in
the **frame free-function layer** (`frame::share`/`frame::free`, presently
`external_body` / best-effort), which is not yet verified. This is a phase
boundary, not proof laziness — but it means the contracts for these three methods
are not yet "sufficient to reject bugs" (spec-design §1.3).

## Caller Coverage
- **Covered: 5 / 8 functions fully** (`new`, `address`, `refcount`, `Upool::new`, `Upool::alloc`); **3 / 8 partial** (`share`, `leak`, `drop`).
  - (codex tallied this at the finer expectation granularity as **8 / 15** expectations — same underlying gap, different denominator.)
- **Missing (all the documented §8 deferral):**
  1. `share`: `F' == F.add_ref(self@)` (the `+1`), `refcounts[self@] < 255`, and the Err-arm frame-unchanged condition.
  2. `drop`: `F' == F.release(self@)` (any functional postcondition at all).
  3. `leak`: the no-release / `phys_view()`-unchanged guarantee.
- Assessment: the deferral is **sound and honestly documented**; every expressible
  snapshot fact is captured. But judged strictly against caller expectations
  (fork/CoW callers depend on share-increments / drop-releases / leak-suppresses),
  it is a **real coverage gap** — acceptable as a bottom-up phase boundary, not
  complete for a final all-properties-realized sign-off.

## Proof Completeness
- Remaining admit(): **0** in upool. [The 4–7 `admit` the global detector reports are all in `manager.proof.rs:12/27/40/153` — **outside the 8-function upool scope**.]
- Remaining external_body not in tcb-allowed.md: **0**. The only two (`Upool::new` @ upool.rs:250, `Upool::alloc` @ upool.rs:271) are both listed. `UserFrame::{new,address,leak,share,refcount,drop}` are fully machine-verified.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES**.
  - `Upool::new` → tcb-allowed.md:106-117.
  - `Upool::alloc` → tcb-allowed.md:118-124.
  - `Upool` struct is explicitly **ELIMINATED** from `external_body` (tcb-allowed.md:101-105) and is machine-verified.

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **2** (both TCB-approved), assume_specification: **0**, cfg-gated exec: **0 semantic** (1 logging-macro gate at upool.rs:207, sanctioned).
  - Locations: `external_body` @ upool.rs:250 (`Upool::new`), upool.rs:271 (`Upool::alloc`). Other cfg attrs (upool.rs:9,11,37) are the standard `include!`/`verus!` boilerplate.
  - `trusted`/`exec_allows_no_decreases`/`spinoff`/`rlimit`: 0.
- Blocker rule (`admit>0` or `assume>0`): **not triggered**. Unapproved `external_body`: **none**.

## AST Consistency
- AST check: **PASS** (no semantic mismatch). `ast_consistency.py` reports 1 mismatch on `UserFrame::drop`, which is solely the stripped `#[cfg(not(verus_keep_ghost))]` on the `error!` logging macro — pre-approved per verus-constraints/ast-consistency; the macro call is preserved identically and exec behavior in a normal build is unchanged. There are **0** `// VERUS REWRITE` comments in upool.

## Verification
- verus: **PASS**. `make verify-kernel MODULE=mm::phys` ⇒ exit 0, 0 errors. `make verify` (all crates + kernel) ⇒ exit 0, 0 verification errors. 0 in-scope warnings (kernel compiles under the verus build; `make build` is a no-op alias). `spec_drift.py` ⇒ clean.
  - Note: the cheating detector prints `status: CHEATING_DETECTED` for the *whole* `mm::phys` tree (admit=7, external_body=14) — entirely pre-existing, out-of-scope counts in `manager.proof.rs`/`frame.rs`/etc. Upool's share is `external_body=2` (TCB), `admit=0`.

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` = "No code bugs found").
- True Bugs: **0**. No new code defects discovered during proving/integrity. The
  `share`/`drop`/`leak` gap is a **spec-coverage deferral**, not a code defect —
  the bodies are correct thin wrappers; only the contracts under-assert. The
  bugs.md deferral note remains valid and is correctly classified per
  bug-reporting (intentional sound limitation, not a False-Positive defect). No
  surviving *verification failure* exists (verus passes), so nothing requires
  (re)classification as a True/Context-Dependent bug.

## Issues (highest priority first)
1. **[Coverage / Spec sufficiency — HIGH, not a hard blocker]** `UserFrame::share`
   (omits the `+1` `add_ref` + headroom + Err frame-unchanged), `UserFrame::drop`
   (no functional ensures — `release` absent), and `UserFrame::leak` (no
   no-release guarantee) do not realize the reference-count discipline
   `caller_analysis.md` flags as the module's core safety property. Documented,
   sound, technically-forced §8 deferral (0-arg `uninterp phys_view()` +
   not-yet-verified `frame::share`/`frame::free`); remediation = lift
   `add_ref`/`release` through the frame free-function layer's §8 ghost token.
2. **[Note — LOW]** `View for Upool::view` is the single `uninterp spec fn` in
   upool — a mechanical consequence of the TCB `external_body` `new`/`alloc`
   facade; sound and documented.
3. **[Note — INFO]** `result.inv()` in `new`/`address`/`leak` is borderline
   subsumed (codex); harmless, advisory only.
4. **[Note — INFO]** AST checker's `UserFrame::drop` mismatch is the pre-approved
   stripped logging-macro cfg-gate; no action.

## Result: FAIL

**Justification.** Every *hard* gate is clean: verus **PASSES** (`make verify` and
`make verify-kernel` exit 0, 0 errors, 0 in-scope warnings); upool **admit=0,
assume=0**; both `external_body` (`Upool::new`, `Upool::alloc`) are
**TCB-approved**; the single AST mismatch is a pre-approved logging-macro cfg-gate
(no semantic change, no `// VERUS REWRITE`); **no spec drift**; **no code bugs**.
There are **NO BLOCKERS** under the task's blocker definition (admit>0 / assume>0 /
unapproved `external_body`).

The strict rubric nonetheless yields **FAIL** because not all checklist items are
checked: the **Caller coverage / Spec sufficiency** items are unchecked — the
reference-count discipline of `UserFrame::share` (the `+1`), `UserFrame::drop`
(`release`, currently no functional ensures), and `UserFrame::leak` (no-release)
is not yet realized as `ensures`, so those three contracts are not "sufficient to
reject bugs" the caller analysis explicitly identifies. This is a documented,
sound, technically-forced §8 deferral to be discharged when the frame
free-function ghost-token layer is verified — not a cheating, soundness, or TCB
violation. A reviewer accepting the deferral as an in-scope phase boundary could
reasonably score **PASS**; under the strict "all properties realized for final
sign-off" reading applied here, the consolidated result is **FAIL on
completeness**, with a clear, bounded remediation path.
