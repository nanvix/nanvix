# Final Comprehensive Review: virt-identity-map

> Consolidated from two independent sub-agent reviews:
> - `final_review.claude.md` (claude-opus-4.8)
> - `final_review.codex.md` (gpt-5.3-codex)
>
> Both reviewers independently reached **FAIL**. Ground truth confirmed by direct
> grep/view and by `make verify-kernel` (status `CHEATING_DETECTED`).

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified
- [x] Pre-existing specs assessed (if any exist from upstream verification)

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite)
- [x] All caller-observable state represented (no missing fields)
- [x] No implementation-specific fields (only caller-observable state)
- [ ] inv() encodes real constraints (not trivially true) — `internal_inv()` is hardcoded `true`; `identity_map_view()` is `uninterp`, so the abstract `mapped` set is **never tied to the concrete page tables**. The well-formedness `inv()` carries only page-alignment + pre-init-empty facts. The implementation-consistency invariant is a placeholder.
- [x] Mathematical types used (`Set<int>`, `bool`; addresses keep usize where appropriate)

### Specification
- [x] Every in-scope exec function has requires/ensures
- [ ] Caller coverage: several caller expectations not encoded (see Caller Coverage)
- [ ] View consistency: specs reference `identity_map_view()` fields, but the view is `uninterp` and `internal_inv()==true`, so `inv()` does not constrain real state — consistency is vacuous
- [x] No tautological ensures (`Err(_) => true`) — error arms are meaningful (`!accessible`, `!mapped.contains`, `inv()`)
- [x] No subsumed ensures
- [x] Error paths have meaningful ensures (match style)
- [~] No assume_specification for workspace-internal code — `FixedSizeBumpAllocator::new` is a workspace `bump_allocator` crate (temporarily allowed placeholder); `<[T]>::as_ptr` is std (acceptable)
- [x] vstd searched before any assume_specification
- [ ] Specs written for the caller / usable in caller proofs — undermined because bodies are entirely `admit()`-ed (no proof actually connects spec to implementation)
- [x] Trait obligations satisfied (none for in-scope functions)
- [ ] Spec completeness (advisory): under-specified vs caller expectations
- [x] Loop invariants: in-scope functions contain no loops
- [ ] No cheating on module's own functions: **3 `admit()` in-scope** — BLOCKER
- [ ] No specs weakened (spec_drift): not establishable as PASS while bodies are admitted (nothing proven to compare semantics against)
- [x] Bug awareness: no functional code defect found in the three functions
- [ ] Cross-module regression (`make verify`): not clean — crate-wide status `CHEATING_DETECTED`
- [ ] Verification (`make verify-kernel` / `make build`): exit 0 but `CHEATING_DETECTED` — BLOCKER

### Proving
- [ ] No specs weakened (spec_drift) — see above
- [ ] **Zero remaining admit()** — FAIL: 3 admits (one per in-scope function)
- [x] Zero external_body in-scope (in-scope files have none)
- [~] Zero assume/assume_specification — 0 `assume`, but 2 `assume_specification` + 1 `external_type_specification` remain in spec.rs (std + not-yet-verified workspace dep; allowed as external-bottom placeholders, not the decisive blocker)
- [x] No cfg-gated exec branches/expressions/match-arms (only `error!` logging on error paths)
- [x] Cheating audit performed — exact counts/locations reported below
- [ ] Verus-limitation reproducers — N/A (no rewrites claimed)
- [x] Exec rewrites minimal/equivalent — no `// VERUS REWRITE` comments exist
- [ ] Cross-module regression (`make verify`) clean — FAIL (CHEATING_DETECTED)
- [ ] Verification 0 errors/0 warnings — admit-gated pass only

### Cheating Elimination
- [ ] **Zero admit() remaining** — FAIL: 3
- [x] Zero assume() remaining
- [x] Zero trusted functions (in-scope)
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code (only error-path logging)
- [x] Zero external_body in-scope (unauthorized) — none in the three functions
- [x] AST consistency: zero mismatches (no rewrites)
- [x] All exec rewrites have VERUS REWRITE + reproducer — N/A (none)
- [x] Each surviving external_body listed in tcb-allowed.md — none in-scope; the underlying HAL/BSS boundaries it builds on are pre-approved
- [ ] No specs weakened (spec_drift) — not establishable while admitted
- [ ] Cross-module regression clean — FAIL
- [ ] Verification 0 errors/0 warnings — admit-gated only

### Bug Recording
- [x] bugs.md exists if bugs were found — none found, file correctly absent... BUT see note: the unproven (admitted) state is a process gap, not a code bug
- [x] Each bug is a real code defect — N/A (no bugs recorded)
- [x] Each bug entry has What/Why/How Verus Helped/Severity/Suggested Fix — N/A
- [x] No external_body used to mask a code defect — confirmed
- [x] Bug entries include provenance — N/A

## Spec Quality
The public API contracts on the three functions are **well-written in prose and shape**:
match-style `Ok/Err` arms, meaningful (non-tautological) error postconditions
(`!accessible`, `!mapped.contains(...)`, `inv()` preservation), and a clean
`IdentityMapView { initialized, mapped: Set<int> }` abstraction with idempotent
`spec_install_page`/`spec_map_page` transitions and a fully-proven `*.proof.rs`
lemma set (no admit in proof.rs).

However the specs are **vacuous against the implementation**: `identity_map_view()`
is `uninterp` and `IdentityMapView::internal_inv()` is hardcoded `true`. The abstract
`mapped` set is never connected to the concrete PDE/PTE bits or the BSS pool, and—
critically—every exec body begins with `proof! { admit(); }`, so no proof links any
spec to the code. The contracts are aspirational, not established.

## Caller Coverage
- Covered: ~5 / 8 caller expectations
- Missing / under-specified:
  - **No-frame-allocator-recursion** ("page tables come from the BSS pool; safe to call
    from within `KernelFrame::new`") — not expressed in any spec.
  - **TLB consistency** (`invlpg` makes the new mapping immediately effective) — not in
    `ensure_pte`/`identity_map_page` ensures.
  - **Explicit permission predicate** (present + writable + supervisor) — only described
    in doc comments / view prose; membership in `mapped` is asserted to *imply* it but no
    spec predicate exposes it to callers.
  - `ensure_pt` `Ok` arm omits the caller-relied facts "PDE at `pde_idx` is present after
    the call" and "`pt_paddr` is that PT's frame address" (only page-alignment + `inv()`).

## Proof Completeness
- Remaining admit(): **3** — each a BLOCKER:
  - `src/kernel/src/mm/virt/identity_map.rs:534` `ensure_pt`
  - `src/kernel/src/mm/virt/identity_map.rs:632` `ensure_pte`
  - `src/kernel/src/mm/virt/identity_map.rs:719` `identity_map_page`
  (cheating-detail.txt records the same three at decl lines 533/627/718.)
- Remaining external_body not in tcb-allowed.md: **0** in-scope (the three files declare no `external_body`).

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES** for in-scope code (no in-scope
  `external_body`). The underlying trusted boundaries the functions build on
  (`Table::read`/`write`, `paging::invlpg`, `FixedSizeBumpAllocator::alloc`/`alloc_as`)
  are all pre-approved in `tcb-allowed.md`. No new trust boundary introduced.

## Guardrails Compliance (in-scope files)
- admit: **3** (identity_map.rs:534, 632, 719) — **BLOCKER**
- assume: 0
- external_body: 0
- assume_specification: 2 (identity_map.spec.rs:178 `<[T]>::as_ptr` (std); :182 `FixedSizeBumpAllocator::new` (workspace dep placeholder))
- external_type_specification: 1 (identity_map.spec.rs:142 `ExPageTableBss`)
- cfg-gated exec: 5 `#[cfg(not(verus_keep_ghost))]` — all guard `error!(...)` logging on error paths (allowed category), plus 2 `#[cfg(verus_keep_ghost)]` import/attr gates

(Crate-wide for context: `make verify-kernel` reports `assume=0 external_body=11 admit=31 trusted=0 no_decreases=0 cfg_gate=15`, status `CHEATING_DETECTED`.)

## AST Consistency
- AST check: **PASS** — no `// VERUS REWRITE` comments exist in any in-scope file; no exec rewrites to validate, so no possible semantic mismatch.

## Verification
- verus: **FAIL** — `make verify-kernel` returns exit code 0, but the harness reports
  `status: CHEATING_DETECTED` because of the admits. The exit-0 is an artifact of
  `admit()` discharging the proof obligations vacuously; it is **not** a genuine pass.

## Bug Summary
- Total bugs recorded: 0 (`bugs.md` does not exist — correctly, as no functional defect
  was discovered in the three functions).
- True Bugs: 0. No logic error, safety violation, or incorrect behavior was found in the
  implementation. The failure here is **process/completeness**: the proving phase was not
  actually completed — all three in-scope bodies are wholly `admit()`-ed, so their
  correctness is unestablished. This is not a recordable "bug" per the bug-reporting skill
  (it is a verification gap, not a code defect), but it is a hard release blocker.

## Issues (highest priority first)
1. **BLOCKER — 3 `admit()` in the in-scope functions** (`ensure_pt`, `ensure_pte`,
   `identity_map_page`). Nothing about the function bodies is proven. Must be eliminated
   with real proofs.
2. **BLOCKER — verification status `CHEATING_DETECTED`.** `make verify-kernel` only "passes"
   because the admits discharge obligations vacuously.
3. **Major — spec is vacuous against the implementation.** `identity_map_view()` is
   `uninterp` and `internal_inv()` is `true`; the abstract `mapped` set is never tied to the
   real page tables, so even the stated contracts are not connected to the code.
4. **Moderate — incomplete caller coverage:** no-frame-allocator-recursion, TLB consistency,
   explicit present/writable/supervisor permission predicate, and `ensure_pt`'s
   "PDE present / `pt_paddr` is the PT frame address" are unspecified.
5. **Minor — residual placeholders:** `assume_specification` for `FixedSizeBumpAllocator::new`
   (workspace `bump_allocator`, not yet verified) and the `ExPageTableBss`
   `external_type_specification` remain; acceptable as external-bottom placeholders but must
   be superseded when those modules are verified.

## Result: **FAIL**

Both independent reviewers (claude-opus-4.8 and gpt-5.3-codex) concur. The proving and
cheating-elimination phases did **not** complete: all three in-scope functions are entirely
`admit()`-ed (3 admits → 3 BLOCKERS) and the toolchain reports `CHEATING_DETECTED`.
Multiple checklist items are unchecked. PASS requires zero admits and a genuinely clean
verification; neither holds.
