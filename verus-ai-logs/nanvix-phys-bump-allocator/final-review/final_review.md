# Final Comprehensive Review: bump-allocator

> Consolidated from two independent sub-agent reviews:
> - `final_review.claude.md` (claude-opus-4.8) — RESULT: FAIL
> - `final_review.codex.md` (gpt-5.3-codex) — RESULT: FAIL
>
> Both models converged independently on the same verdict and the same root cause.
> Scope: `FixedSizeBumpAllocator::alloc_as`, `FixedSizeBumpAllocator::alloc`,
> `align_up`, `BssStorage::as_mut_ptr` (the only in-scope functions). Files:
> `src/libs/bump_allocator/src/lib.rs`, `lib.spec.rs`, `lib.proof.rs`.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — LSP run (`find_callers_lsp_output.md`) + documented manual recovery of cross-crate kernel callers
- [x] Caller expectations (success + failure) documented for each pub function (`caller_analysis.md:53-129`)
- [x] Abstract resource identified (`caller_analysis.md:106-113`)
- [x] Pre-existing specs assessed — clean slate, empty `verus!{}` blocks upstream (`caller_analysis.md:131-139`)

### View Design
- [x] Every field passes the substitution test (`view_design.md:287-305`)
- [x] All caller-observable state represented (base/stride/unit_size/unit_align/capacity/storage_size/allocated)
- [x] No implementation-specific fields (no `AtomicUsize`/`PhantomData` in `BumpView`)
- [x] inv() encodes real constraints (`lib.spec.rs:116-133`) — non-trivial
- [x] Mathematical types used (`int`/`nat`; addresses as `int`)

### Specification
- [x] Every in-scope exec function has requires/ensures (`fn_coverage`: 3/6, the 3 uncovered are `fmt`/`new`/`default`, all out of scope; `as_mut_ptr` carries a trait-level spec)
- [ ] **Caller coverage** — uniqueness/non-aliasing, monotone-capacity/Exhausted boundary, and no-spurious-consumption are NOT expressed in `alloc`/`alloc_as` contracts (see Caller Coverage below). **UNCHECKED — BLOCKER**
- [~] View consistency — `requires bump_view(self).inv()` is present, but the ensures express **no** `v → v'` transition and never reference `BumpView::slot_addr`/`spec_alloc`; `inv()` preservation is not stated. Partial.
- [ ] **No tautological ensures (`Err(_) => true`)** — `alloc`'s *only* error arm is `Err(_) => true` (`lib.rs:283`); `alloc_as` has a trailing `Err(_) => true` (`lib.rs:364`). **UNCHECKED — BLOCKER**
- [ ] **No subsumed ensures** — n/a given the gaps; the surviving Ok-arm facts are abstract over an uninterpreted address (see below)
- [ ] **Error paths have meaningful ensures** — `alloc` gives no meaning to `Exhausted`/`Overflow`/`OutOfBounds`/`Misaligned`; designed arms (`view_design.md:227-235`) were dropped. **UNCHECKED**
- [x] No assume_specification for workspace-internal code (the single one targets std `usize::div_ceil`)
- [x] vstd searched before assume_specification — both reviewers searched local vstd trees; no `div_ceil` spec exists; external-bottom std boundary is legitimate
- [ ] **Specs written for the caller** — the foundational uniqueness guarantee the kernel `unsafe` soundness relies on (`caller_analysis.md:117-119`) is not delivered in a caller-usable contract. **UNCHECKED**
- [~] Trait obligations satisfied — `as_mut_ptr` spec pins only `result as int == base_of::<S>()` (`lib.rs:200-204`); stability/size/writability/alignment duties from `caller_analysis.md:37-43,100-102` are not in the contract (partly TCB by nature, but base alignment `base_of % A == 0` from `view_design.md:182-184` is also absent)
- [ ] Spec completeness (advisory) — fails: caller-critical properties missing
- [x] Loop invariants — the only loop (`alloc` CAS retry) lives in an `external_body` fn; no spec loop obligations
- [x] No cheating on module's own functions — `admit`=0, `assume`=0, `trusted`=0; `external_body`=2 (both TCB-listed); `assume_specification`=1 (std `div_ceil`)
- [x] No specs weakened vs git — `spec_drift.py`: `Contract drift: 0` (NB: this only compares against the git baseline; the spec was *born* weaker than `view_design.md §5`, which git cannot detect)
- [x] Bug awareness — `bugs.md` "no code bugs" is consistent with final code (checked_add/checked_mul/div_ceil guards present)
- [x] Cross-module regression — `make verify-bump-allocator` clean; per claude review crate build + tests pass
- [x] Verification — `make verify-bump-allocator` exit 0, 0 errors; `make build` no-op/pass

### Proving
- [x] No specs weakened — `spec_drift.py`: 0 drift
- [x] Zero remaining admit()
- [x] Zero external_body unless listed in `tcb-allowed.md` — both (`alloc`, `alloc_as`) are listed (`tcb-allowed.md:16-23`)
- [x] Zero assume/assume_specification except external-bottom — only std `div_ceil`
- [x] No cfg-gated exec code
- [x] Cheating audit — admit=0, external_body=2 (TCB), assume=0, cfg-gated exec=0
- [~] Claimed Verus limitation has isolated reproducer — the duplicate-impl-path / atomic-unreadable limitations are *described* (`lib.spec.rs:12-17,162-176`) but the consequence is a **weaker delivered contract**, not just a rewrite; the deferral was never completed
- [x] Exec rewrites minimal/equivalent — none (no `// VERUS REWRITE` comments; AST clean)
- [x] Cross-module regression — verify clean
- [x] Verification — 0 errors

### Cheating Elimination
- [x] Zero admit()
- [x] Zero assume()
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code
- [x] Zero external_body unless listed in `tcb-allowed.md` — both listed
- [x] AST consistency: zero mismatches (claude: MATCH vs base branch AND vs pre-verus baseline `e79991a92` — 12 funcs + 7 structs; codex: `Consistent: 12 functions, 7 structs match`)
- [x] All exec rewrites have VERUS REWRITE comment + reproducer — none needed (no rewrites)
- [x] Each surviving external_body confirmed in `tcb-allowed.md`
- [x] No specs weakened vs git — 0 drift
- [x] Cross-module regression — pass
- [x] Verification — 0 errors

### Bug Recording
- [x] bugs.md exists; states no code bugs found
- [x] Each "bug" is a real code defect — n/a (none recorded); claim consistent with code
- [x] Entry format — n/a (no bugs)
- [x] No external_body used to mask a code defect — the 2 external_body are genuine raw-pointer/`usize as *mut` trust boundaries, not defect masks
- [x] Provenance — specification phase noted

## Spec Quality
`align_up` is excellent: a clear, total, View-independent numeric contract
(`lib.rs:126-133`) pinned to `align_up_spec` (`lib.spec.rs:57-68`), consistent with
the `usize::div_ceil` assumption and the `checked_mul` overflow signal. `BumpView`
and `inv()` are well-designed (non-trivial, math-typed, pass the substitution test).

The **API contracts for `alloc`/`alloc_as` are materially weaker than both the
design (`view_design.md §5`) and the caller requirements**:
- `alloc` Ok-arm bounds only per-call alignment + in-bounds over the **uninterpreted**
  `slot_ref_addr(slot)` (`lib.rs:276-282`), never tying the returned reference to
  `BumpView::slot_addr(v.allocated)`. Because `alloc` is `external_body`, these
  ensures are *assumed*, not derived; over an uninterpreted address they do not
  connect to real pointer identity.
- No post-state / transition (`allocated + 1`), so monotone capacity and the
  `Exhausted` boundary are absent.
- `alloc`'s sole error arm is the textbook-forbidden `Err(_) => true`.
- The geometry/transition lemmas (`lemma_geometry`, `lemma_alloc_transition`,
  `lemma_exhausted_boundary`) and the helpers `slot_addr`/`geometry_ok`/`spec_alloc`
  are **orphan** — grep-confirmed they are referenced only inside `lib.proof.rs`,
  never by any exec contract in `lib.rs`. They are provable over an *arbitrary*
  `BumpView` but never instantiated for an actual allocation.

The documented reason (atomic cursor unreadable in spec; `impl View` duplicate-impl
panic) is transparent and honest, but the consequence is that the **foundational
uniqueness / non-aliasing guarantee remains unverified at final review**.

## Caller Coverage
- Covered: **~12 / 22** (codex strict matrix); by invariant family: **3 / 6**
  delivered (claude).
- Delivered: In-bounds (per-slot), Alignment (per-slot), Stable-size (`alloc_as`
  `SizeMismatch`/`AlignmentMismatch`), `align_up` (4/4).
- **Missing:**
  - Uniqueness / non-aliasing across calls (`caller_analysis.md:73-76,117-119`) — the
    `forall j` distinctness clause designed at `view_design.md:223-225,274` is absent.
  - Monotone capacity / `Exhausted` boundary (`caller_analysis.md:79,123-125`).
  - No-spurious-consumption on error (`caller_analysis.md:77-80,128-129`) — `Err(_)=>true`.
  - `alloc` per-error meaning for `Exhausted`/`Overflow`/`OutOfBounds`/`Misaligned`.
  - `as_mut_ptr` base alignment / size / writability semantic duties
    (`caller_analysis.md:37-43,100-102`).

## Proof Completeness
- Remaining admit(): **0**.
- Remaining external_body not in `tcb-allowed.md`: **0** (both `alloc` @ `lib.rs:271`
  and `alloc_as` @ `lib.rs:348` are listed in `tcb-allowed.md:16-23`).

## TCB Compliance
- All external_body listed in `tcb-allowed.md`: **YES**. No new trust boundaries
  introduced. No BLOCKER on this axis.

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **2** (both TCB-listed),
  assume_specification: **1** (std `usize::div_ceil`, legitimate external-bottom),
  cfg-gated exec: **0**.
- No guardrail/cheating BLOCKER. (`make verify` prints `CHEATING_DETECTED` solely
  because external_body=2, which are pre-approved TCB entries.)

## AST Consistency
- AST check: **PASS** (both reviewers; vs base branch and vs pre-verus baseline
  `e79991a92`; no `// VERUS REWRITE` / `// VERUS DEVIATION` comments; spec drift 0).

## Verification
- verus: **PASS** — `make verify-bump-allocator` exit 0, 0 errors; `make build` pass.

## Bug Summary
- Total bugs recorded: **0**.
- True Bugs: **0**. `bugs.md`'s "no code bugs found" is consistent with the final
  code state. No undocumented code defect was discovered during proving. The
  outstanding issues are **specification-completeness defects**, not runtime-code
  bugs, and are correctly absent from `bugs.md` (per the bug-reporting skill, which
  scopes `bugs.md` to real code defects).

## Issues (highest priority first)
1. **BLOCKER — Uniqueness/non-aliasing not caller-delivered.** The kernel's `unsafe`
   soundness depends on every `alloc_as` returning a slot distinct from all prior
   ones; no `alloc`/`alloc_as` ensures expresses this. The supporting lemma
   `lemma_alloc_transition` is orphan (proof-only).
2. **BLOCKER — Tautological / missing error specs.** `alloc`: `Err(_) => true`
   (`lib.rs:283`); `alloc_as` trailing `Err(_) => true` (`lib.rs:364`). The designed
   `v'.allocated == v.allocated` "no-consumption" arms and the `Exhausted` boundary
   (`view_design.md:227-235`) were not implemented.
3. **BLOCKER — Monotone-capacity transition absent.** `alloc`/`alloc_as` contracts
   carry no post-state, so `allocated+1` / `Exhausted ⇔ allocated==capacity` are not
   surfaced to callers; `lemma_exhausted_boundary` is orphan.
4. **Major — Uninterpreted-address bridge weakens the Ok arm.** Ensures range over
   uninterpreted `slot_ref_addr`/`bump_view`; combined with `external_body` they are
   assumed about an abstract symbol never tied to `slot_addr(v.allocated)`. A
   degenerate "same-slot-every-call" allocator would satisfy the current contract.
5. **Minor — `as_mut_ptr` semantic duties under-specified** (base alignment, size).
6. **Non-blocking note** — `assume_specification[usize::div_ceil]` is acceptable
   (no vstd spec exists); keep it.

### Required to reach PASS
Wire the designed contracts (`view_design.md §5`) into the `alloc`/`alloc_as`
ensures via the deferred atomic-ghost / `PointsTo` token: tie the returned reference
to `slot_addr(v.allocated)`, state the `allocated+1` transition and `Exhausted`
boundary, surface the `forall j` distinctness, and replace `Err(_) => true` with the
`v'.allocated == v.allocated` no-consumption arms — making
`lemma_geometry`/`lemma_alloc_transition`/`lemma_exhausted_boundary` live instead of
orphan. (Strictly within the in-scope functions; no TCB change.)

## Result: **FAIL**

Rationale: all mechanical gates pass (admit=0, assume=0, external_body=2 both
TCB-listed, AST MATCH, spec drift 0, verus 0 errors), but multiple **Specification**
checklist items are unchecked — caller coverage is incomplete (uniqueness,
monotone-capacity, no-spurious-consumption) and `alloc`/`alloc_as` carry tautological
`Err(_) => true` arms. Per the review rule ("PASS only if ALL checklist items are
checked"), any unchecked item is FAIL. Both independent sub-agents (claude-opus-4.8
and gpt-5.3-codex) reached this verdict separately.
