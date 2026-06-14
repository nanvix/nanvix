# Final Comprehensive Review: bump-allocator

> Consolidated from two independent sub-agent reviews (one per model) plus
> orchestrator-run ground-truth checks. Both independent reviewers — `claude-opus-4.8`
> (`final_review.claude.md`) and `gpt-5.3-codex` (`final_review.codex.md`) — reached
> **FAIL** on the same root cause: incomplete caller coverage. All mechanical
> guardrails pass.
>
> In-scope functions: `FixedSizeBumpAllocator::alloc`, `FixedSizeBumpAllocator::alloc_as`,
> `align_up`, `BssStorage::as_mut_ptr` (`BackendA/B/C::as_mut_ptr`). Out-of-scope
> `fmt`/`new`/`default` are intentionally unspecced and not flagged.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` run; cross-crate misses recovered via grep + source reading (documented limitation in `caller_analysis.md`)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified — fixed-capacity pool of `NUM_UNITS` equal-sized slots
- [x] Pre-existing specs assessed — clean slate (empty `verus!{}` blocks upstream)

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite)
- [x] All caller-observable state represented (no missing fields)
- [x] No implementation-specific fields (only caller-observable state)
- [x] inv() encodes real constraints (not trivially true)
- [x] Mathematical types used (int/Seq/Set/Map; addresses as `int`)

### Specification
- [x] Every in-scope exec function has requires/ensures (3/6 coverage from `fn_coverage`; the 3 uncovered are out-of-scope `fmt`/`new`/`default`)
- [ ] **Caller coverage**: 3 of 6 key caller invariants have NO corresponding requires/ensures (Uniqueness/non-aliasing, Monotone-capacity/Exhausted, No-spurious-consumption) — **BLOCKER**
- [ ] **View consistency**: specs reference `bump_view` fields and `requires inv()`, but the `v → v'` transition and uniqueness are NOT wired; the supporting lemmas are orphan/floating
- [ ] **No tautological ensures**: `Err(_) => true` present on both `alloc` (lib.rs:283) and `alloc_as` (lib.rs:364)
- [x] No subsumed ensures
- [ ] **Error paths have meaningful ensures**: `alloc` error path carries no state-preservation guarantee
- [x] No assume_specification for workspace-internal code (`div_ceil` is std)
- [x] vstd searched before any assume_specification (no `div_ceil` spec in vstd)
- [ ] **Specs written for the caller**: the foundational non-aliasing guarantee the kernel's `unsafe` code relies on is not deliverable through the exec contract
- [x] Trait obligations satisfied (`as_mut_ptr` base-pinning; alignment/range duties are the `unsafe trait` TCB contract)
- [ ] Spec completeness (advisory): missing uniqueness/capacity/no-consumption properties
- [x] Loop invariants: the only loop (CAS reservation) is inside an `external_body` fn — body not verified, so no invariant required
- [x] No cheating on module's own functions: admit=0, assume=0, external_body=2 (both TCB-approved), trusted=0
- [x] No specs weakened: `spec_drift.py … --before HEAD` → 0 drift
- [x] Bug awareness: no fundamentally incorrect code; `bugs.md` accurate
- [x] Cross-module regression: `make verify` → exit 0, all modules 0 errors (no regression introduced)
- [x] Verification: `make verify-bump-allocator` exit 0 / 0 errors; `make build` exit 0

### Proving
- [x] No specs weakened: `spec_drift.py` clean
- [x] Zero remaining admit()
- [x] Zero external_body unless listed in `tcb-allowed.md` — 2, both listed
- [x] Zero assume/assume_specification beyond external-bottom: `assume_specification` ×1 on `<usize>::div_ceil` (std external-bottom, allowed)
- [x] No cfg-gated exec code (only `#[cfg(verus_keep_ghost)]` ghost-include gates and `#[cfg(test)]`)
- [x] Cheating audit: admit=0, external_body=2 (lib.rs:271 `alloc`, lib.rs:348 `alloc_as`), assume=0, cfg-gated exec=0
- [x] Claimed Verus limitation has isolated reproducer — `verus-unsupported.md` records exact errors + minimal triggers for break-with-value, `usize→*mut` cast, and `AtomicUsize::load` in `spec`
- [x] Exec rewrites minimal and semantically equivalent — none exist (no `// VERUS REWRITE` comments)
- [x] Cross-module regression: `make verify` passes
- [x] Verification: `make verify-bump-allocator` 0 errors

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code
- [x] Zero external_body unless listed in `tcb-allowed.md` (2, both listed)
- [x] AST consistency: zero mismatches (12/12 MATCH)
- [x] All exec rewrites have VERUS REWRITE comment and minimal reproducer (vacuous — none)
- [x] For each surviving external_body: confirmed listed in `tcb-allowed.md`
- [x] No specs weakened: `spec_drift.py` clean
- [x] Cross-module regression: `make verify` passes
- [x] Verification: `make verify-bump-allocator` 0 errors

### Bug Recording
- [x] bugs.md exists
- [x] Each bug is a real code defect — N/A: `bugs.md` correctly records "no code bugs found" (vacuously satisfied)
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix — N/A (no bug entries)
- [x] No external_body used to mask a code defect — the 2 external_body are int-to-ptr trust boundaries, not defect masks
- [x] Bug entries include provenance — `bugs.md` notes specification-phase provenance

## Spec Quality
- **`align_up`** — the one in-scope function whose body is actually verified. Bidirectional
  contract pinned to `align_up_spec` (concrete `open spec fn`); total, overflow-safe in
  `int`/`nat`. **Correct, complete, caller-usable.**
- **`assume_specification [<usize>::div_ceil]`** — legitimate std external-bottom boundary
  (`requires y != 0`, ceiling-division `ensures`); vstd confirmed to lack a `div_ceil` spec.
  **Acceptable.**
- **`as_mut_ptr`** — `ensures result as int == base_of::<Self>()` encodes pointer stability and
  binds to `BumpView::base`. The `≥ STORAGE_SIZE` / writable / `A`-aligned duties remain the
  `unsafe trait` TCB contract. Codex notes these range/alignment duties are not surfaced as a
  contract — a minor completeness observation, acceptable as a TCB assumption.
- **`alloc` / `alloc_as`** — `external_body` (TCB-approved). Success arms guarantee per-slot
  alignment + in-bounds over an `uninterp slot_ref_addr` (mirrors `raw-array`); `alloc_as`
  adds genuinely useful bidirectional `SizeMismatch`/`AlignmentMismatch` guard arms. **However**
  the contract is materially weaker than `view_design.md §5`: the returned slot is not bound to
  `slot_addr(allocated)`, there is no distinctness clause, no `allocated+1` transition, no
  `Exhausted` boundary, and no no-spurious-consumption arm. The `Err(_) => true` arms are
  tautological. The lemmas that prove these abstract facts (`lemma_geometry`,
  `lemma_exhausted_boundary`, `lemma_alloc_transition`) are **orphan** — no exec contract
  references `slot_addr`/`geometry_ok`/`spec_alloc`/`has_capacity`/`lemma_*`.

## Caller Coverage
Canonical list = the 6 "Key Invariants (caller perspective)" in `caller_analysis.md`.
- Covered: **3 / 6** — In-bounds, Alignment, Stable-size-contract.
- Missing:
  - **Uniqueness / non-aliasing** — not on any exec contract; only in floating `lemma_geometry`/`lemma_alloc_transition`. This is the foundational guarantee the kernel's `unsafe` page-table code depends on.
  - **Monotone capacity / Exhausted boundary** — `Err(_) => true`; no `has_capacity`/`allocated` surfaced; only in floating `lemma_exhausted_boundary`.
  - **No spurious consumption on error** — no `v'.allocated == v.allocated` arm.
- (Granular cross-check by the codex reviewer: 11 / 17 individual success+failure obligations covered — same three families missing, plus `as_mut_ptr` range/alignment duties and per-variant error semantics.)
- Cause: `view_design.md §7` legitimately defers the `v → v'` transition to a proving-phase
  atomic-ghost/`PointsTo` token (Verus cannot read `AtomicUsize` in `spec` — documented
  limitation with reproducer). The deferral is well-founded, but the consequence is that these
  caller expectations are **not currently delivered** to callers.

## Proof Completeness
- Remaining admit(): **0**. (The only textual `admit` hit is a stale header *comment* at
  `lib.proof.rs:6`; all three lemma bodies are real proofs.)
- Remaining external_body not in `tcb-allowed.md`: **0**. Both `external_body`
  (`lib.rs:271 alloc`, `lib.rs:348 alloc_as`) are listed in `verus-ai-logs/tcb-allowed.md`.

## TCB Compliance
- All external_body listed in `verus-ai-logs/tcb-allowed.md`: **YES** — `alloc` (lines 16–20),
  `alloc_as` (lines 21–23). No external_body outside the approved TCB.

## Guardrails Compliance
- admit: 0, assume: 0, external_body: 2, assume_specification: 1, cfg-gated exec: 0
  (Verus cheating-check independently confirms: `assume=0 external_body=2 admit=0 trusted=0
  no_decreases=0 cfg_gate=0`.)

## AST Consistency
- AST check: **PASS** — `ast_consistency.py` 12/12 functions MATCH, 0 mismatched, 0 missing,
  0 extra. No `// VERUS REWRITE` comments exist (semantic-equivalence check vacuous).

## Verification
- verus: **PASS** — `make verify-bump-allocator` exit 0, 0 errors (`6 verified`). `make build`
  exit 0. Cross-module `make verify` exit 0, all modules 0 errors. (The `CHEATING_DETECTED`
  pipeline tag is driven solely by the 2 TCB-approved external_body, not a real finding.)

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` records "no code bugs found", reconciled and still valid).
- True Bugs: **0**. The in-scope exec code is correct — every address computation uses
  `checked_*`, bounds/alignment are validated before a slot is handed out, `align_up` guards
  `alignment == 0`. The missing caller coverage is a **spec-completeness gap caused by a
  documented Verus limitation** (atomics not spec-readable), not a code defect — correctly
  *not* classified as a bug per the bug-reporting skill.
- No surviving verification failure requires classification (verus 0 errors).

## Issues (highest priority first)
1. **[BLOCKER] Uniqueness / non-aliasing not on the exec contract.** Designed in
   `view_design.md §5.1`, dropped from `lib.rs`. The kernel's `unsafe` soundness depends on it.
2. **[BLOCKER] Exhausted/monotone-capacity boundary and no-spurious-consumption not on the
   exec contract.** `Err(_) => true` on both functions; no transition/`has_capacity` surfaced.
3. **[Major] Floating/orphan lemmas.** `lemma_geometry`, `lemma_exhausted_boundary`,
   `lemma_alloc_transition` connect to no exec contract (spec-design anti-pattern #5). Wiring
   them requires the deferred atomic-ghost token attaching `BumpView` as the allocator's View.
4. **[Minor] Tautological `Err(_) => true` arms** on `alloc`/`alloc_as` (acceptable per
   `caller_analysis.md` — all errors collapse to `OutOfMemory` — but they carry no
   state-preservation guarantee).
5. **[Minor] Stale `lib.proof.rs` header comment** (lines 6–7) claims lemma bodies are
   `admit()` placeholders; they are proven. Update the comment.
6. **[Minor] `as_mut_ptr` range/alignment duties** are TCB assumptions rather than contract.

## Result: FAIL

Every mechanical guardrail passes — **admit 0, assume 0, both `external_body` in the TCB, AST
PASS, verus 0 errors, build OK, cross-module verify OK, spec-drift clean.** But the explicit PASS
bar requires *all checklist items checked*, and the Specification section's **caller coverage** is
incomplete: 3 of the 6 documented key caller invariants (Uniqueness/non-aliasing,
Monotone-capacity/Exhausted, No-spurious-consumption) are not surfaced on any in-scope exec
contract, and the lemmas that establish them float disconnected from the API. Both independent
reviewers reached the same verdict. The gap is well-founded (deferred to the atomic-ghost/`PointsTo`
token blocked by the documented "atomics not spec-readable" Verus limitation), but per the stated
strict criteria — any unchecked item is FAIL — the effort is **not yet complete**.
