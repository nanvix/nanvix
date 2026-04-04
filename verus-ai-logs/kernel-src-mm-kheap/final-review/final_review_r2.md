# Final Comprehensive Review: kheap (Round 2)

## Grade: B+

## Spec Quality

**Strong.** The four verified functions (`layout_to_allocator`, `from_raw_parts`,
`allocate`, `deallocate`) have well-crafted external-top contracts:

- **Bidirectional error paths**: `allocate` and `deallocate` have fully bidirectional
  failure conditions with explicit state preservation (`self@ == old(self)@`).
- **Exact state transitions**: `spec_allocate`/`spec_deallocate` on `KheapView` encode
  complete state changes via `Seq::update` — frame conditions are implicit (only the
  target slab changes).
- **Declarative abstractions**: Specs use `KheapView`, `spec_slab_for_size`,
  `block_sizes()` — mathematical abstractions independent of implementation.
- **Caller-oriented**: A caller can reason about routing (`layout_to_allocator`), state
  transitions (`allocate`/`deallocate`), and constructor guarantees (`from_raw_parts`)
  without reading the implementation.
- **Tightest-fit clause** on `layout_to_allocator` (FN-1c strengthened) adds genuine
  value for memory efficiency reasoning.
- **No anti-patterns**: No tautological postconditions, no one-sided error specs
  (with the noted `from_raw_parts` caveat), no code-as-spec, no missing frame conditions.

**View abstraction** (`KheapView` with `Seq<SlabView>`): Clean, compositional, uses
`ext_equal`, enables quantified invariants over all slabs.

**Weakness**: `from_raw_parts` error branch only specifies `e.code == InvalidArgument`
(not the full bidirectional FN-2g). The fix report correctly explains this is the strongest
provable statement: the full `Err ⟹ ¬checks` would be violated if an inner
`Slab::from_raw_parts` fails after kheap checks pass. The Ok-branch forward implications
provide the contrapositive (`Ok ==> checks`, so `¬checks ==> Err`) via Result
exhaustiveness. The remaining gap (Slab-level failures) is unreachable in practice (LIVE-1).

## Property Coverage

- **Covered: ~34 / 40 in-scope verifiable** (out of 59 total property IDs)
- **Partial: TYPE-4, FN-2g, MOD-4 (conditional), MOD-7**
- **Uncovered (in-scope): LIVE-1, LIVE-2**
- **Not formalized (negligible risk): TYPE-5, TYPE-6**
- **Out of scope by design: FN-5a–c, FN-6a–c, FN-7b–d, GLOBAL-1–5** (14 properties)

| Category | Covered | Partial | Uncovered (in-scope) | Out-of-scope |
|----------|---------|---------|---------------------|--------------|
| TYPE-1–6 | 1,2,3 | 4 | 5, 6 (compiler guarantee) | — |
| FN-1a–d | all 4 | — | — | — |
| FN-2b–g | b,c,d,e,f | g | — | — |
| FN-3b–g | all 6 | — | — | — |
| FN-4b–f | all 5 | — | — | — |
| FN-5,6,7 | — | — | — | all 9 |
| MOD-1–7 | 1,2,3,4*,5,6 | 7 | — | — |
| LIVE-1–6 | 3,4,5,6 | — | 1, 2 | — |
| GLOBAL-1–5 | — | — | — | all 5 |

\* MOD-4 covered by conditional lemma (`base_addr > 0`); runtime fact not axiomatized.

## Proof Completeness

- **Remaining admit(): 0** [confirmed by both reviewers]
- 10 fully verified proof functions, zero escapes.
- Cheating metrics: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0.

| Proof | Property |
|-------|----------|
| `lemma_regions_ordered` | Transitive slab ordering |
| `lemma_kheap_inv_implies_cross_slab_disjointness` | MOD-1, MOD-2, MOD-3 |
| `lemma_slab_for_size_valid` | Index validity + size bound |
| `lemma_alloc_dealloc_round_trip` | LIVE-5 |
| `lemma_allocate_conserves` | MOD-5 (alloc direction) |
| `lemma_deallocate_conserves` | MOD-5 (dealloc direction) |
| `lemma_slab_for_size_tightest_fit` | FN-1c strengthened |
| `lemma_block_sizes_strictly_increasing` | TYPE-3 strengthened |
| `lemma_slab_for_size_total` | Totality over [1, max_slab_size()] |
| `lemma_no_null_address` | MOD-4 (conditional) — **NEW in R2** |

## Trust Boundary Audit

- **assume_specification: 2** (both human-approved)
- **axiom: 0** (custom; vstd built-in axioms used via `broadcast use`)
- **external_body: 2** (1 type spec, 1 helper function)

| Item | Location | Approved? | Assessment |
|------|----------|-----------|------------|
| `Layout::size` assume_spec | spec.rs:83–85 | ✅ `[x]` in property_analysis.md | Minimal — uninterpreted accessor, correct |
| `Error::new` assume_spec | spec.rs:88–91 | ✅ `[x]` in property_analysis.md | Minimal — only constrains `.code` field |
| `ExLayout` external_type_spec + external_body | spec.rs:59–61 | N/A (type decl) | Standard pattern for opaque foreign types |
| `usize_to_mut_ptr` external_body | spec.rs:95–100 | ✅ `[x]` in property_analysis.md (added R1) | Trivially correct (`addr as *mut u8` preserves address); cfg-gated Verus workaround |

- **Unapproved items: 0 BLOCKER**

## Exec Fidelity

- **AST check: PASS**

```
Functions: 3 MATCH, 4 MISMATCH, 0 missing, 0 extra
Structs:   3 MATCH, 0 MISMATCH, 0 missing, 0 extra
```

All 4 mismatches are pre-approved deviations:

| Function | Deviation | Classification |
|----------|-----------|----------------|
| `layout_to_allocator` | Named return `-> (result: ...)` | Pre-approved |
| `from_raw_parts` | Named return; `mem::PAGE_SIZE` cfg-gated; `addr as *mut u8` cfg-gated; `info!()` cfg-gated | Pre-approved + legitimate cfg-gating |
| `allocate` | Named return; `|_|` → `|_e|` (10 closures) | Pre-approved + documented Verus limitation |
| `deallocate` | Named return; `|_|` → `|_e|` (10 closures) | Pre-approved + documented Verus limitation |

Original exec code preserved under `#[cfg(not(verus_keep_ghost))]` in all cases.

## Verification

- **verus: PASS** — `make verify-kernel MODULE=mm::kheap` → **20 verified, 0 errors**

```
=== Results ===
  20 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0
  coverage: 4/7 exec functions have contracts
```

## R1 Issue Resolution

All 6 issues from the previous review were verified against actual code:

| Issue | Claimed Fix | Verified? | Regression? |
|-------|------------|-----------|-------------|
| 1. Unverified wrappers (Medium) | Verus limitation | ✅ Correct | None |
| 2. MOD-4 no null (Low) | Conditional lemma added | ✅ Confirmed in proof.rs:199–223 | None |
| 3. LIVE-1/LIVE-2 (Low) | Verus limitation | ✅ Correct | None |
| 4. Err underspecified (Low) | Not implementable | ✅ Sound reasoning | None |
| 5. usize_to_mut_ptr doc (Info) | Added to assumptions | ✅ Confirmed in property_analysis.md:727 | None |
| 6. TYPE-5/TYPE-6 (Info) | No change needed | ✅ Correct | None |

No regressions detected. No weakened specs. `lemma_no_null_address` is a genuine
improvement (+1 verified item). Verification count increased from 19 to 20.

## Issues (highest priority first)

1. **[Medium] 3 unverified wrapper functions** — `GlobalAlloc::alloc`, `GlobalAlloc::dealloc`,
   `init()` have no Verus contracts. Core logic is verified, but the GlobalAlloc API
   surface and initialization are not machine-checked. **Verus limitation (static mut).**

2. **[Low] LIVE-1/LIVE-2 not machine-checked** — Slab construction feasibility and
   init() infallibility are argued convincingly from constant analysis but not formally
   proven. Requires bidirectional Slab spec + static mut support. **Verus limitation.**

3. **[Low] FN-2g Err branch partial** — `from_raw_parts` error branch only says
   `e.code == InvalidArgument`. Bidirectional failure condition not provable without
   LIVE-1. **Strongest provable statement given current Slab spec.**

4. **[Info] TYPE-5, TYPE-6 not formalized** — Compiler guarantees, negligible risk.

## Result: FAIL

**Rationale**: Grade B+ — a strong verification effort with clean proofs, zero admits,
rigorous bidirectional specs, proper abstractions, and minimal trust boundaries. The R1
fixes were properly executed (conditional MOD-4 lemma, documentation update). However:

- 3/7 exec functions (43%) lack contracts, leaving the GlobalAlloc API unverified
- LIVE-1/LIVE-2 lack formal proofs
- FN-2g is partially specified

These gaps are overwhelmingly caused by Verus tooling limitations (static mut, upstream
Slab spec directionality), not methodology failures. The verified core demonstrates
proper technique throughout. A grade of A would require Verus evolution or architectural
restructuring to move global state management out of scope.

**Improvement from R1**: +1 verified item (`lemma_no_null_address`), documentation
fix for `usize_to_mut_ptr` assumption. No regressions. Grade unchanged (B+).

---

*Consolidated from independent reviews by Claude Opus 4.6 (final_review_r2.claude.md)
and GPT-5.3-Codex (final_review_r2.gpt.md). Reviewers agreed on all major findings;
minor disagreement on Issue 4 contrapositive argument resolved in favor of its soundness
(Result exhaustiveness makes `Ok ⟹ P` equivalent to `Err ⟹ ¬P` for binary Result).*
