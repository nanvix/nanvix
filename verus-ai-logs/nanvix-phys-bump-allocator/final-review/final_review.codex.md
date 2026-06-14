# Final Independent Verification Review — bump_allocator

Reviewed scope (read-only):
- `src/libs/bump_allocator/src/lib.rs`
- `src/libs/bump_allocator/src/lib.spec.rs`
- `src/libs/bump_allocator/src/lib.proof.rs`
- `verus-ai-logs/nanvix-phys-bump-allocator/{caller_analysis.md,view_design.md,bugs.md}`
- `verus-ai-logs/tcb-allowed.md`

Guardrails count line: **admit=0, assume=0, external_body=2, assume_specification=0, cfg-gated exec code=0**.
AST consistency: **FAIL** (1 mismatch: `align_up`).
Caller Coverage: **5/11** expectations covered.
Bug Summary: **No confirmed runtime code bug in in-scope exec logic; unresolved issues are spec/integrity gaps.**

---

## 1) Spec quality review

### What is good
- `align_up` has a precise match-style contract against `align_up_spec` (`lib.rs:126-132`), and `align_up_spec` is concrete (`lib.spec.rs:43-54`).
- `BumpView::inv()` is non-trivial and encodes meaningful arithmetic/geometric constraints (`lib.spec.rs:102-119`).
- `as_mut_ptr` has a stable-base postcondition (`lib.rs:232-235`).

### Blockers
1. **`bump_view` is uninterpreted and not connected to concrete allocator constants/state.**
   - Definition is uninterpreted (`lib.spec.rs:163-165`).
   - `alloc`/`alloc_as` require `bump_view(self).inv()` (`lib.rs:305`, `lib.rs:382`).
   - But `inv()` does **not** tie fields to `N`, `A`, `S::NUM_UNITS`, `S::STORAGE_SIZE` (`lib.spec.rs:102-119`), despite the claim that it does (`lib.spec.rs:152-153`).
   - This makes the API precondition effectively unestablishable from visible exec facts (`new()` has no postcondition, `lib.rs:275-280`) and risks vacuous caller reasoning.

2. **External-top contracts are under-specified versus caller needs.**
   - `alloc` success only states local alignment/in-bounds (`lib.rs:307-314`) and failure only `Err(Exhausted)` (`lib.rs:315`), omitting cross-call uniqueness, capacity transition, and no-spurious-consumption.
   - `alloc_as` similarly omits state-transition/frame properties (`lib.rs:384-403`).

3. **Uninterpreted spec functions are used as modeling escapes (`base_of`, `slot_ref_addr`, `bump_view`: `lib.spec.rs:27`, `36`, `163`)**, weakening caller-facing meaning for address/state facts.

### BumpView substitution test
- Field choices are mostly abstraction-level (base/stride/unit/capacity/storage/allocated) and not obviously implementation-specific (`lib.spec.rs:66-81`): **pass**.
- `inv()` clauses are non-trivial: **pass**.
- Main failure is **attachment fidelity** (uninterpreted accessor + missing pinning), not field choice.

---

## 2) Caller coverage (success/failure + key invariants)

Reference expectations: `caller_analysis.md:67-143`.

| # | Expectation | Covered? | Evidence |
|---|---|---|---|
| 1 | `align_up` least-multiple / `None` iff zero-align or overflow | ✅ | `lib.rs:126-132`, `lib.spec.rs:43-54` |
| 2 | Uniqueness / non-aliasing across successful allocations | ❌ | No cross-call relation in `alloc`/`alloc_as` ensures (`lib.rs:307-316`, `384-403`) |
| 3 | In-bounds returned slot | ✅ | `lib.rs:312-314`, `391-393` |
| 4 | Alignment of returned slot | ✅ | `lib.rs:311`, `390` |
| 5 | Monotone capacity + exact `Exhausted` boundary | ❌ | No `v -> v'`/`allocated` transition in exec contracts |
| 6 | Type-match gating (`size_of`, `align_of`) for `alloc_as` | ✅ | `lib.rs:388-401` |
| 7 | No spurious consumption on error | ❌ | No error-path state-preservation clause (no frame/transition in exec specs) |
| 8 | `as_mut_ptr` stability | ✅ (partial) | `result == base_of::<Self>()` (`lib.rs:234`), but not linked to `bump_view.base` |
| 9 | Backing region size/writability/alignment obligations usable by callers | ❌ | Present only in unsafe-trait prose (`lib.rs:214-221`), not formalized in ensures/requires |
|10| Thread-safe handout uniqueness under concurrency | ❌ | No contract-level concurrency/non-duplication property |
|11| `'static` validity/exclusive ownership dependence | ❌ | Not captured in formal contract beyond unsafe prose |

**Coverage: 5/11** (missing: 2,5,7,9,10,11).

---

## 3) Proof completeness

- `admit()` count: **0** (no code occurrences in reviewed files).
- `assume()` count: **0**.
- `external_body` count: **2** (`lib.rs:303`, `lib.rs:380`).
- No unapproved `external_body` found.

Status: **No admit/assume blocker**. External-body usage remains trusted-surface dependent.

---

## 4) TCB compliance

`external_body` functions in code:
- `FixedSizeBumpAllocator::alloc` (`lib.rs:303`)
- `FixedSizeBumpAllocator::alloc_as` (`lib.rs:380`)

Both are explicitly allow-listed in `verus-ai-logs/tcb-allowed.md:10-17`.

Status: **TCB allow-list compliance PASS**.

---

## 5) AST consistency (align_up rewrite)

Checks performed:
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py --base-ref origin/dev src/libs/bump_allocator/src/lib.rs summary`
  - Result: `align_up` **MISMATCH**, all other items MATCH.
- `... diff --name "align_up"`
  - Confirms replacement of `value.div_ceil(alignment).checked_mul(alignment)` with open-coded ceil division + `checked_mul`.

Semantic-equivalence audit:
- Ceiling-division equivalence is proven in `lemma_ceil_div` (`lib.proof.rs:22-53`).
- Overflow of `qd + 1` on `r != 0` is explicitly argued/proved (`lib.rs:154-163`).
- Minimal reproducer is referenced in code (`lib.rs:144`) and exists (`.../repro/div_ceil_no_spec.rs:1-49`); rerun reproduces unsupported-`div_ceil` error.

**AST consistency result: FAIL (strict rule: any mismatch is blocker).**

---

## 6) Verification run

Required command executed:
- `cd /home/ruize/nanvix-phy && make verify-bump-allocator`
- Exit code: **0** (logs: `verus-ai-logs/verify-bump-allocator/verus-logs/verus_2026-06-15_01-31-41.log`).

Fresh non-cached corroboration run:
- `scripts/verify.sh --crate bump-allocator ... --target-dir /home/ruize/nanvix-phy/build/verus-fresh-bump`
- `10 verified, 0 errors`, exit **0** (`verus-ai-logs/verify-bump-allocator-fresh/verus-logs/verus_2026-06-15_01-35-13.log:6-8,38`).

Reported cheating/coverage:
- `assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0` (`...fresh.../verus_2026-06-15_01-35-13.log:10-16`).
- Coverage: `3/6 exec functions have contracts` (`...fresh.../coverage-unverified.txt:1`).

---

## 7) Guardrails compliance

Exact counts (reviewed module files):
- `admit`: **0**
- `assume`: **0**
- `external_body`: **2**
- `assume_specification`: **0**
- cfg-gated **exec** code: **0**
  - Only cfg attributes found are excluded categories: `no_std` (`lib.rs:83`), `verus_keep_ghost` include-gates (`lib.rs:101`, `105`), and test module gate (`lib.rs:430`).

Rule check:
- `admit > 0` or `assume > 0` => blocker. Current counts satisfy this rule.

---

## 8) Bug reconciliation (`bugs.md`)

`bugs.md` currently states no code bugs and describes proof status (`bugs.md:5-37`).

Reconciliation:
- **Still valid:** no concrete runtime code bug was found in the in-scope exec logic.
- **Not captured in bugs log:** major unresolved verification-integrity/spec issues found in this review:
  1) unestablishable/vacuous `bump_view(self).inv()` precondition surface,
  2) caller-contract coverage gaps (uniqueness, monotone capacity transition, no-spurious-consumption, concurrency/'static obligations),
  3) AST mismatch on `align_up` under strict mismatch policy.

Classification of surviving unresolved issues:
- **Missing spec / contract design gap** (not a proven runtime code bug).
- **Process/integrity blocker** for AST mismatch.

---

## Issues (highest priority first)

1. **BLOCKER — Contract vacuity/unestablishability via uninterpreted `bump_view`** (`lib.spec.rs:163-165`, `lib.rs:305`, `lib.rs:382`, `lib.spec.rs:102-119`).
2. **BLOCKER — Caller expectations under-covered (coverage 5/11)**, especially uniqueness, monotone boundary transition, and no-spurious-consumption (`caller_analysis.md:67-143` vs `lib.rs:307-316`, `384-403`).
3. **BLOCKER — AST mismatch on `align_up` vs base branch** (ast_consistency summary/diff; `lib.rs:137-169`).
4. **Non-blocker note — Trusted surface remains (`external_body=2`) but is allow-listed** (`lib.rs:303`, `380`; `tcb-allowed.md:10-17`).

## Result: FAIL

Reason: blockers remain in spec fidelity/caller coverage and AST consistency under strict mismatch policy.
