# Final Comprehensive Review: bump-allocator

> Consolidated from two independent sub-agent reviews (read-only):
> - `final_review.claude.md` (model: claude-opus-4.8)
> - `final_review.codex.md`  (model: gpt-5.3-codex)
>
> Both reviewers independently reached **FAIL**. They agree on the substantive
> blockers (spec coverage / contract fidelity); they initially differed only on
> how to score the `align_up` AST mismatch — reconciled below.
>
> Scope (only these functions): `FixedSizeBumpAllocator::alloc_as`,
> `FixedSizeBumpAllocator::alloc`, `align_up`, `BssStorage::as_mut_ptr`.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` + rust-analyzer LSP (`caller_analysis.md`, `script_output.md`)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified — fixed-capacity, vend-at-most-once slot pool
- [x] Pre-existing specs assessed — clean slate (empty `verus!{}` upstream)

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite) — `view_design.md §6`
- [x] All caller-observable state represented (no missing fields) — pool geometry + `allocated`
- [x] No implementation-specific fields — no `next_slot`/`Ordering`/`MaybeUninit` leak
- [x] inv() encodes real constraints (not trivially true) — geometry/bounds/no-wrap/ceiling
- [x] Mathematical types used (int/Seq/Set/Map; addresses keep usize) — `int`/`nat`

### Specification
- [x] Every in-scope exec function has requires/ensures (`fn_coverage.py`) — `align_up`,`alloc`,`alloc_as`,`as_mut_ptr` all have specs; the 3 uncovered (`fmt`,`new`,`default`) are OUT of scope
- [ ] **Caller coverage** — uniqueness/non-aliasing, monotone-capacity transition, and no-spurious-consumption-on-error are NOT in the exec ensures (BLOCKER I-1)
- [ ] **View consistency** — `inv()` does NOT pin `base/stride/unit_size/unit_align/capacity/storage_size` to the type-level constants (`N`,`A`,`S::NUM_UNITS`,`S::STORAGE_SIZE`); `bump_view` is `uninterp`, so `requires bump_view(self).inv()` is not caller-dischargeable (BLOCKER I-3)
- [x] No tautological ensures (no `Err(_) => true`)
- [x] No subsumed ensures
- [ ] **Error paths have meaningful ensures** — `alloc` ensures `Err ⇒ Exhausted` is *over-strong/false* for the boundary `base+storage_size==usize::MAX+1` (`checked_add` → `Err(Overflow)`); trusted via `external_body`, not checked (BLOCKER I-2)
- [x] No assume_specification for workspace-internal code — none
- [x] vstd searched before any assume_specification — `div_ceil` confirmed absent in vstd
- [ ] **Specs written for the caller (usable directly in caller proofs)** — contracts are stated over uninterpreted `bump_view`/`slot_ref_addr` with no axioms tying them to `slot.as_ptr()` or to `N/A/S` (BLOCKER I-3)
- [x] Trait obligations satisfied — `as_mut_ptr` stability formalized; size/writability duties are the unsafe `BssStorage` TCB duty by design
- [x] Spec completeness (advisory) — intentional nondeterminism acceptable
- [x] Loop invariants — only loop (CAS retry in `alloc`) is inside `external_body`; not Verus-checked
- [ ] **No cheating on module's own functions** — `external_body=2` on the module's OWN functions (`alloc`,`alloc_as`); TCB-approved but still a trusted surface
- [x] No specs weakened (`spec_drift.py`) — 0 contract drift vs HEAD
- [ ] **Bug awareness** — the over-strong trusted `Err ⇒ Exhausted` spec (I-2) was not surfaced/classified
- [x] Cross-module regression (`make verify`) — all modules exit 0 (bitmap, sys, nanvix-slab, bump-allocator, kernel)
- [x] Verification (`make verify-bump-allocator` & `make build`) — exit 0, 0 errors; build up-to-date

### Proving
- [x] No specs weakened (`spec_drift.py`) — 0 drift
- [x] Zero remaining admit()
- [x] Zero external_body unless listed — both `alloc`/`alloc_as` ARE in `tcb-allowed.md`
- [x] Zero assume/assume_specification
- [x] No cfg-gated exec code
- [x] Cheating audit reported — admit=0 assume=0 external_body=2 cfg_gate=0
- [x] Claimed Verus limitation has isolated reproducer — `repro/div_ceil_no_spec.rs`
- [x] Exec rewrites minimal and semantically equivalent; `// VERUS REWRITE` present — `align_up` only
- [x] Cross-module regression — pass
- [x] Verification — 0 errors

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code
- [x] Zero external_body unless listed — both listed in `tcb-allowed.md`
- [x] AST consistency: only semantically-equivalent rewrites for verified Verus limitations — `align_up` div_ceil rewrite is proven equivalent (`lemma_ceil_div`) and reproducer-backed; the tool's textual MISMATCH is the *allowed* category per this item's own wording
- [x] All exec rewrites have VERUS REWRITE comment and minimal reproducer
- [x] For each surviving external_body: confirmed listed in `tcb-allowed.md`
- [x] No specs weakened
- [x] Cross-module regression — pass
- [x] Verification — 0 errors

### Bug Recording
- [x] bugs.md exists
- [x] Each recorded bug is a real code defect — bugs.md records *no* code bugs (correct: exec logic is defensive/correct)
- [x] Bug entry format (What/Why/How Verus Helped/Severity/Suggested Fix) — N/A (no entries)
- [x] No external_body used to mask a *code* defect — code is correct; I-2 is a *spec* defect, not a code defect
- [x] Bug entries include provenance — N/A

## Spec Quality
- **`align_up` — GOOD.** `align_up_spec` (`lib.spec.rs:43`) is concrete, total, `nat`-typed; `#[verus_spec]` (`lib.rs:126`) ties `Some`/`None` exactly. Matches caller expectations.
- **`as_mut_ptr` — adequate.** `ensures result as int == base_of::<Self>()` (`lib.rs:232`) anchors stability; size/writability stay TCB by design.
- **`BumpView` — internally well-formed but mis-attached.** Field set passes the substitution test and `inv()` (`lib.spec.rs:102`) is non-trivial, BUT the accessor `bump_view` (`lib.spec.rs:163`) is `uninterp` and `inv()` does not pin fields to `N/A/S` constants. The spec NOTE claiming the pinning (`lib.spec.rs:152-153`) is not borne out by `inv()`.
- **`alloc`/`alloc_as` — the contracts are the problem.** They are the *only* caller-facing contracts, are stated over disconnected `uninterp` symbols, omit the central allocator safety property (non-aliasing), and contain an over-strong (unsound) error postcondition that `external_body` leaves unchecked.

## Caller Coverage
- **Covered: 5 / 11 expectations** (`caller_analysis.md` per-function + Key Invariants); equivalently **3 / 6 Key Invariants**.
- Covered: `align_up` least-multiple/`None`; in-bounds; alignment; `alloc_as` type-match gating; `as_mut_ptr` stability; `Exhausted` boundary path.
- **Missing:**
  - Uniqueness / non-aliasing (Key Invariant #1) — the #1 property for handing out `&'static mut`; `view_design §5.1` specified `forall j … slot != slot_addr(j)` but it is absent from ensures.
  - Monotone-capacity transition (`allocated+1`, `slot == slot_addr(allocated)`).
  - No-spurious-consumption-on-error (`v'.allocated == v.allocated`).
  - Thread-safe handout / no-duplicate-index (#5) — not modeled.
  - `'static` validity / exclusive-ownership (#6) — type-level only.
  - Backend region size/writability obligations — unsafe-trait prose only, not formalized.
- The proof lemmas that would back these (`lemma_geometry`, `lemma_exhausted_boundary`, `lemma_alloc_transition`) are fully proven but have **zero exec call sites** — decoupled from the verified surface.

## Proof Completeness
- Remaining admit(): **0** — no BLOCKER.
- Remaining external_body NOT in `tcb-allowed.md`: **0** — no BLOCKER. (`alloc` `lib.rs:303/318`, `alloc_as` `lib.rs:380/405`; both allow-listed at `tcb-allowed.md:10-17`.)

## TCB Compliance
- All external_body listed in `tcb-allowed.md`: **YES** — both `alloc` and `alloc_as` are pre-approved; no new trust boundary introduced. No BLOCKER on this axis.

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **2** (both TCB-listed), assume_specification: **0**, cfg-gated exec: **0**
- Tool cheating check agrees: `assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
- No `admit>0`/`assume>0` BLOCKER; no unlisted-`external_body` BLOCKER.

## AST Consistency
- AST check: **PASS** (with one allowed deviation). `ast_consistency.py` reports a textual MISMATCH on `align_up` (`value.div_ceil(alignment)` → open-coded ceil + `checked_mul`). This is a *semantically-equivalent* rewrite for a *verified* Verus limitation (no vstd spec for `usize::div_ceil`; minimal reproducer `cheating-elimination/repro/div_ceil_no_spec.rs`), proven equivalent by `lemma_ceil_div` and with the `qd+1` no-overflow argument discharged inline (`lib.rs:152-164`). The checklist explicitly permits such rewrites, so this is not a behavioral mismatch. Cosmetic nit: the comment is labeled `// VERUS REWRITE` rather than the skill's `// VERUS DEVIATION`. (Sub-agent split: codex scored the raw tool MISMATCH as FAIL; claude scored behavioral PASS — reconciled to PASS per the checklist's own allowance.)

## Verification
- verus: **PASS** — `make verify-bump-allocator` exit 0; fresh non-cached run `10 verified, 0 errors`.
- build: **PASS** — `make build` up-to-date (no errors).
- cross-module `make verify`: **PASS** — all modules exit 0.

## Bug Summary
- Total bugs recorded (bugs.md): **0** true code bugs — accurate for exec logic (the allocator body is defensive and correct).
- True Bugs: **0** (no runtime code defect).
- Spec-integrity issue (not a code bug, unrecorded): **I-2** — `external_body`-trusted error postcondition `Err ⇒ Exhausted` on `alloc` is stronger than the real body, which can return `Overflow`/`OutOfBounds`/`Misaligned`. Concrete counterexample: `inv()` clause (d) admits `base+storage_size==usize::MAX+1`, at which `base.checked_add(S::STORAGE_SIZE)` (`lib.rs:341-343`) returns `Err(Overflow)`. A checked proof would reject this ensures; `external_body` suppresses the check. Per `bug-reporting` this is a spec defect (fix the spec), not a `bugs.md` code-bug entry.

## Issues (highest priority first)
- **I-1 (BLOCKER, Spec quality + Caller coverage) — Uniqueness / non-aliasing unverified and unasserted.** The #1 caller invariant (`caller_analysis.md:129-133`) and the `forall j … slot != slot_addr(j)` clause from `view_design §5.1` are absent from `alloc`/`alloc_as` ensures (`lib.rs:307-316`, `:384-403`). The core safety property of an allocator handing out `&'static mut` is not established. `lemma_geometry`'s distinctness exists but is never wired to exec.
- **I-2 (BLOCKER, Soundness) — Over-strong error postcondition trusted via `external_body`.** `alloc` ensures `Err(e) => e == Exhausted` (`lib.rs:315`) and `alloc_as` restricts the error set (`lib.rs:395-401`), but both bodies can return other variants; `external_body` makes this trusted, not checked. Latent unsoundness for any caller that discharges the precondition. Not recorded in `bugs.md`.
- **I-3 (BLOCKER, Spec fidelity) — Contracts and most proofs stated over disconnected `uninterp` symbols.** `bump_view`, `slot_ref_addr`, `base_of` are `uninterp` with no axioms linking them to real state or `slot.as_ptr()`; `inv()` is not pinned to `N/A/S`. `verus-constraints` flags `uninterp + external_body` as "the same effect as `assume`." Result: `requires bump_view(self).inv()` is not caller-dischargeable and the geometry facts are about opaque integers. `lemma_geometry`/`lemma_exhausted_boundary`/`lemma_alloc_transition` (proven) have zero exec call sites.
- **I-4 (Non-blocker, note) — Trusted surface remains.** `external_body=2` on the module's own functions; allow-listed and TCB-compliant, but the entire caller-facing contract lives on trusted bodies, which is what lets I-1/I-2 pass unchecked.
- **I-5 (Nit) — Deviation comment label.** `align_up` uses `// VERUS REWRITE`; the ast-consistency skill prescribes `// VERUS DEVIATION` for confirmed-limitation rewrites.

## Result: FAIL

Rationale: no hard guardrail blocker (admit=0, assume=0, all `external_body` TCB-approved, AST behaviorally consistent, all builds/verification green, no spec drift). **But** the *Specification* and *Caller-coverage* dimensions are not clean: the allocator's #1 safety contract (non-aliasing) is neither proven nor asserted (I-1); an `external_body`-trusted error postcondition is over-strong/unsound and unrecorded (I-2); and the entire contract surface plus most proofs are stated over disconnected `uninterp` symbols with a non-dischargeable precondition (I-3). Under the strict bar (PASS only if **every** checklist item is checked), unchecked Specification items make the result **FAIL**. Both independent reviewers concur.
