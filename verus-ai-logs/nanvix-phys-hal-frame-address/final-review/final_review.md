# Final Comprehensive Review: hal-frame-address

> Consolidated from two independent strict reviews:
> - `final_review.claude.md` (model **claude-opus-4.8**) → PASS
> - `final_review.gpt55.md` (model **gpt-5.5**) → FAIL (sole basis: `from_raw_value` one-sided error/liveness spec)
>
> Adjudication of the single disagreement is recorded in **Issues** and
> **Result**. Authoritative `make` outputs were produced once by the
> orchestrator (`make verify-kernel`, `make build`, `make verify`) to avoid
> shared-target-dir corruption; both sub-agents independently re-ran the
> read-only checkers (`ast_consistency.py`, `spec_drift.py`, guardrail greps)
> and the counts agree.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` (rust-analyzer LSP) output in `find_callers_lsp_output.md`
- [x] Caller expectations (success + failure) documented for each pub function — `caller_analysis.md` §"Caller Expectations"
- [x] Abstract resource identified — opaque, page-aligned physical frame address (`int` view)
- [x] Pre-existing specs assessed — `caller_analysis.md` §"Pre-existing Specs" (only `into_raw_value` had a contract upstream)

### View Design
- [x] Every field passes the substitution test — single field `self@:int` survives a complete rewrite (`view_design.md` substitution table)
- [x] All caller-observable state represented — raw value, frame index, alignment, membership all derive from `self@`
- [x] No implementation-specific fields — `PageAligned<PhysicalAddress>` representation hidden (`closed view`)
- [x] inv() encodes real constraints — alignment + frame-number representability, both load-bearing
- [x] Mathematical types used — `type V = int` (addresses bridged to `usize` only at raw-value boundary)

### Specification
- [x] Every in-scope exec function has requires/ensures — all 4 targets specced (`frame.rs:64-101,116-152`)
- [x] Caller coverage — 5/5 in-scope; every expectation maps to a contract (see Caller Coverage)
- [x] View consistency — specs reference `self@`/`inv()` and maintain the invariant
- [x] No tautological ensures — no `Ok => true` / `result == result`; the only `Err(_) => true` is on an external-bottom trust boundary (see Issues #3)
- [x] No subsumed ensures — each clause adds caller-usable information
- [x] Error paths have meaningful ensures — see adjudication (Issues #3): `from_raw_value` failure is external-bottom & dynamic; a `match` Err arm would be a forbidden tautology, so implication style is the correct encoding
- [x] No assume_specification for workspace-internal code that could be body-verified — the 3 assume_specs are external trait-impl edges blocked by unsupported pointer casts (`verus-unsupported.md`)
- [x] vstd searched before assume_specification — boundaries are `arch`/`sys`/`core::ops::Deref` edges, no vstd equivalent
- [x] Specs written for the caller — directly usable (`fa@`, `inv()`, `spec_frame_number`)
- [x] Trait obligations satisfied — `Debug`/`PartialEq` out of scope; pure-projection semantics preserved
- [x] Spec completeness (advisory) — intentional `from_raw_value` Err nondeterminism matches caller expectations (callers only branch on `Ok`/`Err`)
- [x] Loop invariants — N/A (no loops in any in-scope function)
- [x] No cheating on module's own functions — `admit=0 assume=0 external_body=0` in frame module
- [x] No specs weakened — `spec_drift.py … --before verus-ai/sys-virt-address`: 0 ensures removed; only additions to a previously-unspecced fn
- [x] Bug awareness — `bugs.md` present; no code defect (see Bug Summary)
- [~] Cross-module regression (`make verify`) — fails at `verify-bitmap` on an **unrelated pre-existing vstd toolchain bug**; the authoritative gate `make verify-kernel` (covers the whole kernel crate incl. this module) PASSES (see Verification)
- [x] Verification (`make verify-kernel` + `make build`) — PASS, exit 0, 0 errors

### Proving
- [x] No specs weakened — `spec_drift.py`: 0 contract drift vs HEAD; base delta is additions only
- [x] Zero remaining admit() — **0** in frame module
- [x] Zero external_body unless listed — **0** external_body in frame module
- [x] Zero assume/assume_specification beyond external trust boundaries — `assume=0`; 3 `assume_specification`, all external-bottom & TCB-listed
- [x] No cfg-gated exec code — `cfg(verus_keep_ghost)` only on `include!`/`use`
- [x] Cheating audit — admit=0, external_body=0, assume=0, cfg-gated exec=0 (locations: none)
- [x] Claimed Verus limitation has an isolated reproducer — `usize as *const/*mut u8` cast rejection minimally documented in `verus-unsupported.md`
- [x] Exec rewrites minimal & semantically equivalent — 3 `// VERUS DEVIATION` rewrites verified (see AST Consistency)
- [~] Cross-module regression (`make verify`) — see note above (unrelated bitmap/vstd failure; `verify-kernel` PASS)
- [x] Verification (`make verify-kernel` + `make build`) — 0 errors

### Cheating Elimination
- [x] Zero admit() remaining — 0
- [x] Zero assume() remaining — 0
- [x] Zero trusted functions — 0
- [x] Zero exec_allows_no_decreases_clause — 0
- [x] Zero cfg-gated exec code — 0 (only `include!`/`use` gating)
- [x] Zero external_body unless listed — 0 external_body in module
- [x] AST consistency — 3 mismatches all pre-approved semantically-equivalent rewrites (PASS)
- [x] All exec rewrites have VERUS DEVIATION comment — `frame.rs:71,123,143`; pre-approved `f(complex_expr)→let` deviation needs comment, not a separate reproducer
- [x] For each surviving external_body: listed — N/A (none)
- [x] No specs weakened — `spec_drift.py` clean
- [~] Cross-module regression (`make verify`) — unrelated bitmap/vstd failure; `verify-kernel` PASS
- [x] Verification (`make verify-kernel` + `make build`) — 0 errors

### Bug Recording
- [x] bugs.md exists — documents the (resolved) trust-boundary item; no code bug
- [x] Each "bug" is reconciled — the single entry is correctly classified as a False Positive / external-bottom trust boundary, not a defect
- [x] Each entry has What / Why / Resolution / Severity — present
- [x] No external_body used to mask a code defect — no external_body in module
- [x] Bug entries include provenance — discharge phase noted (proving)

## Spec Quality
The `View`/`inv` and the four contracts are caller-driven, declarative, and use
the shared address-tower vocabulary.

- **View** (`frame.spec.rs:57-62`): `type V = int; closed view = self.0@` — the
  single caller-observable quantity (frame base physical address). `closed`
  hides the two-level newtype delegation; passes the substitution test.
- **inv()** (`frame.spec.rs:80-83`, `pub open`): `self@ % spec_page_size() == 0
  && spec_frame_number(self@) <= spec_max_frame_number()`. Both conjuncts
  load-bearing (alignment relied on at every MMU/allocator site; representability
  makes `into_frame_number`'s internal `unwrap` total).
- **into_raw_value** → `result as int == self@` (`frame.rs:95-101`). *Upgraded
  from an upstream `external_body` trust boundary to a body-verified contract — a
  verification improvement.*
- **into_frame_number** → `requires self.inv(); ensures spec_frame_raw_value(result)
  == spec_frame_number(self@)` (`frame.rs:64-69`). Index `== self@/PAGE_SIZE`.
- **from_frame_number** → `result is Ok` + `Ok(fa) ==> fa@ ==
  spec_from_number(spec_frame_raw_value(n)) && fa.inv()` (`frame.rs:116-121`).
  Proves construction never fails (stronger than the caller's `?`).
- **from_raw_value** → `Ok(fa) ==> fa@ == raw_addr as int && fa.inv()`
  (`frame.rs:138-141`). Newtype identity + alignment on success; Err value-free
  (external-bottom dynamic predicate — see Issues #3).

Round-trip closes algebraically from the two frame-index helpers. No tautological
success postconditions, no subsumed ensures, no machine-type leakage into the
View. **Spec quality: PASS.**

## Caller Coverage
- **Covered: 5 / 5** in-scope functions (`FrameAddress` type, `into_raw_value`,
  `into_frame_number`, `from_raw_value`, `from_frame_number`).
- **Missing:** none material. The only unstated semantics is `from_raw_value`'s
  Err condition, which is intentionally value-free (dynamic platform predicate;
  all real callers only branch on `Ok`/`Err`).
- **Caller-analysis accuracy nit (not a coverage gap):** `caller_analysis.md:26`
  labels the `mm/phys/manager.rs` `from_raw_value` hits as false positives, but
  in the current tree `manager.rs:430` and `:438` are **real** call sites (inside
  the TCB-listed `alloc_many_kernel_frames`). Both only branch on `Ok`/`Err` and
  use the `Ok` arm (`fa@ == raw_addr`, `fa.inv()` for `frame::free`), so they are
  **fully covered** by the existing contract. Recommend correcting the label.

## Proof Completeness
- Remaining admit(): **0** (independently grepped `frame.{rs,spec.rs,proof.rs}`).
- Remaining external_body not in `tcb-allowed.md`: **0** (no `external_body` in
  the frame module at all; the 3 textual hits are prose comments).

## TCB Compliance
- All external_body / assume_specification / axiom fn listed in `tcb-allowed.md`: **YES**
  - `axiom fn lemma_phys_view_is_spec_addr` (`frame.proof.rs:38`) → `tcb-allowed.md:311-334` ✔
  - `assume_specification[<PhysicalAddress as Address>::from_raw_value]` (`frame.spec.rs:110`) → `tcb-allowed.md:285-300` ✔
  - `assume_specification[<PageAligned<T> as Deref>::deref]` (`frame.spec.rs:129`) → `tcb-allowed.md:301-309` ✔
  - `assume_specification[::arch::mem::PAGE_SIZE]` (`frame.spec.rs:45`) → acknowledged as the established `arch` hardware-constant edge (cited `tcb-allowed.md:189/226/257/279`); doc nit: no dedicated bullet (Issues #1).
- No new/unlisted trust boundary introduced. The `tcb-allowed.md:170`
  `into_raw_value` `external_body` entry is now **stale** (code body-verifies it —
  uses *less* trust than permitted); harmless, prune when convenient (Issues #2).

## Guardrails Compliance (frame module, exact counts)
- admit: **0**, assume: **0**, external_body: **0**, assume_specification: **3**,
  cfg-gated exec: **0**.
- Additional: axiom fn: **1** (TCB-listed), uninterp spec fn: **1**
  (`spec_page_size`, governed external-bottom hardware constant pinned via
  `assume_specification`).
- `admit == 0` and `assume == 0` → no guardrail blocker.

## AST Consistency
- AST check: **PASS** (6/9 match; 3 mismatches are the in-scope rewrites).
- Per-diff semantic verdicts (all **equivalent**, each carries `// VERUS DEVIATION
  (pre-approved: f(complex_expr) -> let x = complex_expr; f(x))`):
  - `from_frame_number` (`frame.rs:122`): inner call bound to a local; single
    eval, same `?` site.
  - `from_raw_value` (`frame.rs:142`): inner `?`-before-outer-`?` order preserved.
  - `into_frame_number` (`frame.rs:70`): `self.0.into_frame_number()` →
    `let pa: PhysicalAddress = *self.0; pa.into_frame_number()` — explicit `Copy`
    deref of the auto-`Deref` target; same value, same method (`self`-by-value).
- The `f(complex_expr)→let` rewrite is in the ast-consistency skill's pre-approved
  deviation table (comment required, separate reproducer not required); the
  underlying genuine limitation (`usize as *const/*mut u8` cast rejection) is
  minimally documented in `verus-unsupported.md`.

## Verification
- **verus (`make verify-kernel`): PASS** — exit 0; crate-wide cheating snapshot
  `assume=0 external_body=24 admit=0 trusted=0 no_decreases=0`; none of the 24
  crate `external_body`s lie in `hal/mem/types/address/frame.*`.
- **`make build`: PASS** — exit 0.
- **`make verify` (full cross-module): FAILS at `verify-bitmap` (exit 101) —
  unrelated / pre-existing.** Root cause: 9 `vstd` compile errors in the registry
  dependency `std_specs/atomic.rs` (`ExAtomic`/`AtomicBool` generics macro
  mismatch) — a toolchain/vstd-version issue in the `bitmap` crate's dependency
  graph. `make verify` runs `verify-bitmap` *before* `verify-kernel` and stops on
  first failure, never reaching the frame module. This change's footprint is
  `frame.{rs,spec.rs,proof.rs}` + `mm/phys/upool.spec.rs` only — it touches
  neither `bitmap` nor `vstd`. The authoritative module gate `make verify-kernel`
  (which *does* cover the frame module and the entire kernel crate) passes
  standalone. **Not a regression; not a blocker for this module** (checklist item
  marked `[~]` to flag the environmental failure transparently).

## Bug Summary
- Total bugs recorded: **0 code bugs** (`bugs.md` has one non-bug entry).
- True Bugs: **none**.
- The single `bugs.md` entry — the bridge fact `spec_addr(&pa) == pa@` — is
  correctly classified as a **False Positive / external-bottom trust boundary**
  (not a defect). It is honestly discharged via the governed `axiom fn`
  `lemma_phys_view_is_spec_addr` (no `admit`, no `assume`, no `external_body`),
  TCB-registered, removed when `impl Address for PhysicalAddress` is verified. No
  real code bug is masked.
- **`upool.spec.rs` lock-step change:** `UserFrame::inv()` gains
  `spec_frame_number(self@) <= spec_max_frame_number()` — a **STRENGTHENING**
  (added conjunct), confirmed not a weakening by `spec_drift.py`. Required to keep
  `UserFrame::inv()` in lock-step with `FrameAddress::inv()` (`self@ == addr@`);
  the proof burden is paid at every `UserFrame` constructor and `make
  verify-kernel` passing proves they discharge it. Outside the named target list
  but a sound, necessary consequential `.spec.rs` change.

## Issues (highest priority first)
1. **(Advisory — adjudicated NON-blocking) One-sided error/liveness spec on
   `from_raw_value`.** The contract is success-only (`Ok(fa) ==> ...`) and the
   underlying `assume_specification[<PhysicalAddress as Address>::from_raw_value]`
   uses `Err(_) => true` (`frame.spec.rs:117`). gpt-5.5 graded this a HIGH blocker
   (FAIL); claude-opus-4.8 graded it informational. **Adjudication: non-blocking**,
   because: (a) `from_raw_value` is a **stateless** constructor whose only failure
   source is an **external-bottom, dynamic** physical-validity predicate — the
   spec-design "Static vs Dynamic" rule mandates *runtime-check + Err* here, and
   that boundary is TCB-approved (`tcb-allowed.md:285-300`); (b) the boundary
   exposes no validity predicate, so a `match` Err arm could only state
   `Err(_) => true`, which the **"no tautological ensures"** rule forbids — the
   implication style that omits the vacuous arm is the correct encoding; (c) a
   liveness predicate would have to be an *uninterpreted* `spec_valid_physical`
   that **no caller can discharge** (raw_addr is arbitrary boot input) and **no
   caller needs** — spec-design's minimality/direct-usability rules counsel
   against it; (d) **every** real caller (`boot_init.rs:207`, `manager.rs:430`,
   `manager.rs:438`) only branches on `Ok`/`Err` and consumes the `Ok` arm; (e)
   it mirrors the verified sibling `phys.rs` pattern and weakens nothing
   (`spec_drift` clean). *Optional future hardening:* when `impl Address for
   PhysicalAddress` is verified, replace the value-free Err with a named validity
   predicate to add liveness — at that point the trust boundary disappears anyway.
2. **(Low — doc)** `::arch::mem::PAGE_SIZE` `assume_specification` is only
   *acknowledged* in `tcb-allowed.md` (cited as precedent), not given a dedicated
   bullet. Add an explicit one-line entry for audit completeness. Not a soundness
   issue.
3. **(Low — doc)** `tcb-allowed.md:170` still lists `FrameAddress::into_raw_value`
   as an allowed `external_body`, but the code now body-verifies it (uses less
   trust than allowed). Stale entry; prune when convenient.
4. **(Low — doc)** `caller_analysis.md:26` mislabels `manager.rs:430/438` as
   `from_raw_value` false positives; they are real, fully-covered callers. Correct
   the label.
5. **(Environmental — not this change)** `make verify` fails at `verify-bitmap` on
   a pre-existing `vstd`/toolchain incompatibility. Track separately; unrelated to
   `hal-frame-address`.

No genuine blockers: no `admit`, no `assume`, no un-allowlisted `external_body`,
no spec weakening, no semantic AST divergence, no masked code bug. All open items
are low-severity documentation/advisory nits or a pre-existing environmental
failure outside this module.

## Result: PASS

**Justification.** The four in-scope contracts plus the `FrameAddress` View/inv
are correct, complete against every caller expectation (5/5), declarative, and
caller-usable. Guardrails for the frame module are clean: `admit=0`, `assume=0`,
`external_body=0`; the 3 `assume_specification`s and 1 `axiom fn` are each
TCB-registered, and the single `uninterp spec fn` is a governed external-bottom
hardware constant. The 3 AST mismatches are pre-approved, commented,
semantically-equivalent intermediate-value rewrites. `spec_drift` shows zero
contract weakening; the `upool.spec.rs` lock-step edit is a sound strengthening.
`make verify-kernel` (the authoritative module gate, covering the whole kernel
crate) passes (exit 0) and `make build` succeeds; the `make verify` failure is an
isolated, pre-existing `vstd`/`bitmap` toolchain issue with no causal link to this
change. `bugs.md` records no code bug, and the former bridge `admit()` is honestly
discharged via a TCB-governed axiom that masks no defect.

The two independent reviews diverged only on `from_raw_value`'s one-sided error
spec. That point is adjudicated **non-blocking** (Issues #1): for a stateless
constructor with an external-bottom dynamic failure that no caller observes, the
implication-style success-only contract is the correct, precedent-following
encoding — a forced `Err` arm would only reintroduce the tautology the guardrails
forbid. It is recorded as an advisory observation, not a defect.
