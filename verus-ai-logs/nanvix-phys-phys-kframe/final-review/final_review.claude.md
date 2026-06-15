# Final Review (claude-opus-4.8): phys-kframe

Independent, strict, tool-verified review of the Verus effort for `mm::phys::kframe`.
In-scope functions: `KernelFrame::new`, `KernelFrame::drop`, `KernelFrame::base`.
All commands below were run by the reviewer; claims in prior logs were re-checked, not trusted.

Baselines used:
- Original (pre-verification) exec source: `114fce7df` ("[kernel] E: Unify kernel frame allocation").
- Current working tree on branch `verus-ai-prove-bottom-up` (HEAD `3b0085e35`).

---

## Checklist

### Caller Analysis
- [x] All pub fns have callers searched (tool-verified) — `caller_analysis.md` is backed by
  rust-analyzer LSP output (`find_callers_output.md`); `new` (2 sites in `manager`), `base`
  (2 sites in `virt::kpage`), `drop` (implicit, manager error path + `KernelStack::drop`).
- [x] Caller success+failure documented — success and failure paths described for `new`
  (Ok: `kf@==base@`; Err: caller frees `base` itself), `base` (pure), `drop` (frees once).
- [x] Abstract resource identified — owning handle to one page-sized physical frame; View = `int`
  physical address.
- [x] Pre-existing specs assessed — section present; correctly notes only `new` had a spec
  upstream and `base`/`drop` were unspecified.

### View Design
- [x] Every field passes substitution test — `View::V = int` (physical address) survives any
  rewrite of internal storage. Documented and correct.
- [x] All caller-observable state represented — address + page-alignment cover every caller.
- [x] No impl-specific fields — `int` only; no `mapped`/`allocated` leakage (rejected, correctly).
- [x] inv() encodes real constraints — `self@ % spec_page_size() == 0`; consumed by `base()`
  callers (`into_page_address`). Real, not tautological.
- [x] Mathematical types used — `int` view; addresses stay `usize` at exec boundary. Consistent
  with sibling `FrameAddress`/`FrameAllocView` algebra.

### Specification
- [x] Every in-scope exec fn has requires/ensures — `fn_coverage`/`make` coverage: `new`, `base`,
  `drop` all carry `#[verus_spec]`. (`map_frame` is intentionally contract-via-assume_specification.)
- [x] Caller coverage verified for new/base — `kf@==base@`, `result@==self@`, `result.inv()` all
  match caller needs.
- [ ] Caller coverage for drop — **GAP**: caller_analysis & view_design state drop "frees the
  frame exactly once"; the actual `drop` spec has **no functional ensures** (only
  `opens_invariants none`, `no_unwind`). Effect on the allocator is not captured. (Caused by
  callee `frame::free` being `external_body` with an empty contract — out of scope to fix here,
  but the caller expectation is not met by a spec.)
- [x] View consistency — specs reference `self@`/`base@`/`inv()`; maintain `inv()`.
- [x] No tautological ensures — except the acknowledged weak `Err(_) => true` on `new` (below).
- [x] No subsumed ensures.
- [~] Error paths meaningful — `new`'s `Err(_) => true` is a weak error path; justified by the
  address-only View + caller freeing `base` itself, but it is genuinely minimal.
- [ ] No assume_specification for workspace-internal code — **VIOLATED (flagged tension)**:
  `assume_specification[ KernelFrame::map_frame ]` is on **workspace-internal** kernel code, not a
  std/external callee. It is TCB-listed but contradicts the spec-design principle.
- [x] vstd searched before assume_specification — n/a to map_frame (internal); not a vstd gap.
- [x] Specs written for caller.
- [x] Trait obligations satisfied (Drop) — `drop` has a Verus spec (`opens_invariants none`,
  `no_unwind`); Drop obligation met (even if functionally empty).
- [~] spec-completeness advisory — drop functional postcondition missing (see GAP above).
- [x] Loop invariants — none expected; none present.
- [x] No cheating on own functions — in-scope kframe: `admit=0 assume=0 external_body=0 trusted=0`
  (grep + `make` cheating scan; kframe absent from cheating-detail.txt).
- [x] No specs weakened — `spec_drift.py ... --before HEAD` ⇒ exit 0, 0 drift.
- [x] Bug awareness — bugs.md exists (but see stale entry below).
- [x] Cross-module regression — `make verify-kernel MODULE=mm::phys` ⇒ exit 0.
- [x] Verification reported.

### Proving
- [x] No specs weakened — `spec_drift` before HEAD exit 0.
- [x] Zero admit() — in-scope kframe = 0.
- [x] Zero external_body unless TCB-listed — in-scope kframe = 0 external_body.
- [ ] Zero assume/assume_specification except external-bottom trust boundaries — **TENSION**:
  `map_frame`'s boundary is **cross-module** (`mm::virt`, a not-yet-verified sibling), not a true
  external-bottom (hardware/FFI/std). It is TCB-listed but is not a hardware/FFI boundary.
- [x] No cfg-gated exec code (logging exception) — only cfg-gate in kframe is `drop`'s `error!`
  (allowed logging exception).
- [x] Cheating audit counts+locations — provided below.
- [ ] Claimed Verus limitations have isolated reproducer — **MISSING**: the `// VERUS REWRITE`
  comment on `map_frame` claims three limitations (uninterp `identity_map_view().inv()`, external
  `PageAligned::from_raw_value`, `error!` "Unsupported constant type"), but there is **no
  isolated reproducer file** (no `verus-unsupported.md`, no reproducer.rs in the kframe logs dir).
  Only prose justification (review-methodology: "Justification is not a fix").
- [ ] Exec rewrites minimal+equivalent (VERUS REWRITE) — the extraction is minimal & semantically
  equivalent, BUT it rewrites in-scope exec (`new`) and adds a new TCB entry (see blocker).
- [x] Cross-module regression — exit 0.
- [x] Verification 0 errors — `make` reports `verified, 0 errors`.

### Cheating Elimination
- [x] Zero admit (kframe) — 0.
- [x] Zero assume (kframe) — 0.
- [x] Zero trusted (kframe) — 0.
- [x] Zero exec_allows_no_decreases_clause — global `no_decreases=0`.
- [x] Zero cfg-gated exec (logging allowed) — only `drop` logging cfg-gate.
- [x] Zero external_body unless TCB-listed — kframe has 0 external_body; all 15 crate-wide are
  TCB-listed and out-of-scope.
- [ ] AST consistency zero mismatches — **2 MISMATCH + 1 EXTRA**. `drop` mismatch = sanctioned
  logging cfg-gate (OK). `new` mismatch + `map_frame` EXTRA = exec extraction (NOT a pre-approved
  deviation; introduces a trust boundary).
- [ ] All exec rewrites have VERUS REWRITE comment+reproducer — comment YES, reproducer **NO**.
- [x] Each surviving external_body confirmed in TCB — yes (all out-of-scope).
- [ ] No new trust boundaries added (TCB FIXED) — **VIOLATED**: TCB allow-list was edited in
  commit `a2b7376d8` to **remove** `KernelFrame::new` and **add** `KernelFrame::map_frame`.
- [x] No specs weakened — drift exit 0.
- [x] Cross-module regression — exit 0.
- [x] Verification 0 errors.

### Bug Recording
- [x] bugs.md exists.
- [x] Each recorded item is a real defect or honest note (build-hygiene dup import is real).
- [ ] Entries current — **STALE/CONTRADICTORY**: the "Proving-phase note" claims `new` *retains*
  `external_body`; the current code does NOT (it uses `map_frame` extraction + assume_specification).
- [x] No external_body masking a defect — none in kframe.
- [x] Provenance included.

---

## Spec Quality

In-scope specs (all in `kframe.rs`, View/inv in `kframe.spec.rs`):

- `new` (kframe.rs:68–88): `requires base.inv(); ensures Ok(kf) => kf@==base@ && kf.inv(),
  Err(_) => true`. Sound and matches both `manager` callers (`lemma_kernel_alloc_one`,
  `kernel_frames_contiguous`). The `Err(_) => true` arm is a **weak error path**: callers actually
  rely on "frame not consumed on Err" and free `base` themselves. With the address-only View this
  cannot be expressed without modeling allocator state in `new`'s contract, so the weakness is
  acknowledged and defensible, not tautological-by-mistake. Minor.
- `base` (kframe.rs:132–141): `requires self.inv(); ensures result@==self@, result.inv()`. Exactly
  what `virt::kpage` needs. Correct, non-tautological, non-subsumed.
- `drop` (kframe.rs:197–206): `opens_invariants none, no_unwind`, **no functional ensures**. Satisfies
  the Drop trait obligation but does NOT capture the caller-relied "frees the frame exactly once."
  Root cause is the callee `frame::free` (TCB `external_body`, empty contract). Out-of-scope to fix
  but it is a real spec-completeness gap relative to caller_analysis/view_design claims.
- `inv` (kframe.spec.rs:20–22) and `View` (kframe.spec.rs:4–10): clean, caller-abstract, mirror
  `UserFrame`. Good.

Verdict: specs for `new`/`base` are correct and complete enough; `drop` is functionally empty
relative to documented caller expectations; one weak error path on `new`.

## Caller Coverage (Covered 2/3 fully, 1 partial)

| Fn | Caller expectation | Spec clause | Status |
|---|---|---|---|
| new (Ok) | `kf@ == base@`, well-formed | `Ok(kf) => kf@==base@ && kf.inv()` | ✅ |
| new (Err) | frame NOT consumed (caller frees) | `Err(_) => true` | ⚠ weak (no state guarantee; relies on trusted map_frame not touching allocator) |
| base | `result@==self@`, page-aligned | `result@==self@ && result.inv()` | ✅ |
| drop | frees frame exactly once via allocator | (none — only `opens_invariants none`/`no_unwind`) | ❌ not captured |

Missing/weak: drop has no allocator-effect ensures; new's Err path carries no state guarantee.

## Proof Completeness

- In-scope `admit()`: **0** (new/base/drop). `grep` + `make` scan: kframe absent from
  cheating-detail.txt.
- In-scope `external_body`: **0**. `new` is `#[verus_verify]` + `#[verus_spec]` (NOT external_body,
  contrary to bugs.md). `map_frame` carries no `external_body` (its impl has no `#[verus_verify]`;
  contract supplied by assume_specification).
- `make verify-kernel MODULE=mm::phys` ⇒ `verified, 0 errors`, exit 0.

## TCB Compliance

All external_body / type-specs found crate-wide (15) are TCB-listed; **none in kframe**:
frame.rs {instance, init, alloc, alloc_contiguous, free, book, alloc_range}, manager.rs {init,
kernel_watermark}, mod.rs {book_physical_memory_regions, book_mmio_regions}, mod.spec.rs
{ExLinkedList}, upool.rs {Upool, new, alloc}. The in-scope `assume_specification[ KernelFrame::map_frame ]`
**IS listed** (tcb-allowed.md:100–113).

YES — all are listed. BUT: the map_frame entry is a **newly-added** boundary (commit `a2b7376d8`
replaced the prior `KernelFrame::new` external_body entry), so although "listed," it violates the
"TCB is FIXED; no new trust boundaries" constraint.

## Guardrails Compliance

KFRAME (in-scope new/base/drop + map_frame helper):
`admit:0  assume:0  external_body:0  assume_specification:1  cfg-gated-exec:1`
- assume_specification:1 = `KernelFrame::map_frame` (kframe.spec.rs:34) — TCB-listed, but
  workspace-internal + newly-added to the fixed TCB.
- cfg-gated-exec:1 = `drop`'s `#[cfg(not(verus_keep_ghost))] error!` (kframe.rs:203) — ALLOWED
  logging exception.

OUT-OF-SCOPE (siblings frame/manager/mod/upool/virt — NOT a kframe failure):
`admit:7 (manager.proof.rs ×4, identity_map.rs ×3)  external_body:15  cfg_gate:12  assume:0
trusted:0  no_decreases:0`. All TCB-listed or pre-existing.

## AST Consistency (FAIL — 1 sanctioned + 1 unsanctioned)

`ast_consistency.py --base-ref 114fce7df ... summary`: matched=4, mismatched=2, extra=1.
- `KernelFrame::drop` MISMATCH — only the `error!` line gained `#[cfg(not(verus_keep_ghost))]`.
  **Sanctioned** (logging exception). Semantically equivalent.
- `KernelFrame::new` MISMATCH + `KernelFrame::map_frame` EXTRA_IN_VERUS — the identity-map side
  effect was **extracted** out of `new`'s body into a new `map_frame` fn. `new`+`map_frame` together
  are semantically equivalent to the original `new`, BUT this is an exec rewrite of an **in-scope**
  function, it is **not** in the pre-approved deviation table, and it introduced a new
  `assume_specification` trust boundary. The `// VERUS REWRITE` comment exists but **no isolated
  reproducer** backs the claimed Verus limitation.

VERUS REWRITE assessment: the extraction is minimal and behavior-preserving, but it is exactly the
kind of "rewrite exec code + add trust boundary" that verus-constraints forbids, and it lacks the
required reproducer. Note: `new` could have retained its already-TCB-listed `external_body` (the
prior known-good PASS), and `error!` is handleable via the same logging cfg-gate `drop` uses — so
the rewrite was not strictly forced.

## Verification (PASS, exit 0; cheating out-of-scope)

`make verify-kernel MODULE=mm::phys` ⇒ Exit code 0; module `mm::phys` verified; status
`CHEATING_DETECTED` from **out-of-scope** siblings only. Global: `assume=0 external_body=15 admit=7
trusted=0 no_decreases=0 cfg_gate=12`. kframe contributes **zero** to every cheating dimension
except the allowed `drop` logging cfg-gate (not enumerated as a violation). `spec_drift --before
HEAD` exit 0.

## Bug Summary

- Total recorded: 3 notes (no-bugs statement; duplicate-import build-hygiene fix; "Proving-phase
  note").
- True bugs: 0 correctness bugs in scope (consistent with findings). The duplicate-import fix is a
  real (cosmetic/build) defect, properly recorded.
- Stale entries: **1** — the "Proving-phase note" asserts `KernelFrame::new` *retains*
  `external_body`; the CURRENT code does not (it uses the `map_frame` extraction +
  `assume_specification`). This directly contradicts both the source and tcb-allowed.md (which
  removed `new` and added `map_frame`). Must be corrected.

## Issues (highest priority first)

1. **[BLOCKER] New trust boundary added to a FIXED TCB via in-scope exec rewrite.** The TCB
   allow-list was edited (commit `a2b7376d8`) to remove `KernelFrame::new` and add
   `KernelFrame::map_frame`; correspondingly the in-scope exec body of `new` was rewritten
   (AST MISMATCH) to extract the side effect into `map_frame`, which is given an empty
   `assume_specification`. The prompt states the TCB is fixed and no new trust boundaries may be
   added; verus-constraints forbids rewriting exec code to enable verification.
2. **[BLOCKER] Missing reproducer for claimed Verus limitation.** The `// VERUS REWRITE` /
   verification_todo.md justification for `map_frame` has no isolated reproducer file. Required by
   the exec-rewrite acceptance criteria; "justification is not a fix."
3. **[MAJOR] assume_specification on workspace-internal code.** `map_frame` is kernel code (could
   be verified once `mm::virt` is), not a std/external/hardware callee — a cross-module boundary
   masquerading as external-bottom. Tension explicitly flagged.
4. **[MAJOR] Stale/contradictory bugs.md.** "Proving-phase note" claims `new` retains
   `external_body`; it does not. Misleads readers about the actual trust surface.
5. **[MINOR] drop spec functionally empty.** No allocator-effect ensures despite caller_analysis /
   view_design claiming "frees the frame exactly once." Root cause is `frame::free`'s empty TCB
   contract (out of scope), but the caller expectation is unmet by any spec.
6. **[MINOR] `new` `Err(_) => true`.** Weak error path; relies on the trusted `map_frame` not
   touching the allocator on Err.

## Result: FAIL

The kframe in-scope functions are themselves clean of `admit`/`assume`/`external_body` and the
module verifies at exit 0 with no spec drift — a genuinely strong result. However, this is a STRICT
review and several in-scope checklist items cannot be checked off: the in-scope exec body of
`KernelFrame::new` was rewritten and a new `assume_specification[ KernelFrame::map_frame ]` trust
boundary was added to the **fixed** TCB allow-list (commit `a2b7376d8`), producing an unsanctioned
AST MISMATCH on an in-scope function with **no isolated reproducer** for the claimed Verus
limitation, while bugs.md still describes the superseded `external_body`-on-`new` design.

**Single most important blocker:** the `KernelFrame::new` → `map_frame` exec extraction adds a new
workspace-internal `assume_specification` trust boundary to a TCB that was declared fixed, an
exec-code rewrite that verus-constraints forbids and that ships without the required reproducer —
even though the already-TCB-listed `external_body` on `new` was a sanctioned no-rewrite alternative.
