# Independent Review: kheap (Claude Opus 4.6) — Round 2

**Module**: `mm::kheap` — Kernel Heap Allocator
**Baseline**: 20 verified, 0 errors. cheating: assume=0 external_body=2 admit=0 trusted=0.
**Coverage**: 4/7 exec functions have contracts. Unverified: `alloc`, `dealloc`, `init`.
**AST Consistency**: 4 mismatches, all pre-approved.

---

## Spec Quality Assessment

### `layout_to_allocator` (kheap.rs:366–401) — **Strong**

Pure routing function. The spec is well-structured:

- **Ok branch**: Establishes index validity (`opt_idx.is_some()`), size sufficiency
  (`block_sizes()[idx] >= spec_layout_size(*layout)`), index–enum correspondence, and
  *tightest fit* (`forall|j| 0 <= j < idx ==> block_sizes()[j] < size`). The tightest-fit
  clause (FN-1c strengthened) is valuable — it means a caller can reason that no smaller
  slab could service the request, which is important for memory efficiency arguments.
- **Err branch**: Bidirectional — error iff `spec_slab_for_size` returns `None`, which
  by the open spec definition means size is 0 or exceeds max. Clean.
- **Frame**: Implicit — pure function, no state mutation. Appropriate.
- **No anti-patterns**: Declarative, caller-oriented, not code-as-spec.

### `from_raw_parts` (kheap.rs:122–263) — **Good, minor weakness**

Constructor. The spec covers:

- **Requires**: No-wrap (`addr + size <= usize::MAX`) and isize bound. These are safety
  preconditions delegated from pointer arithmetic. Appropriate as `requires` since
  callers control the inputs.
- **Ok branch**: Invariant established (`heap.inv()`), all slabs empty (FN-2c), slab
  containment within partitions (FN-2e), and forward implications encoding precondition
  validity (FN-2g forward: `addr % PAGE_SIZE == 0 ∧ size >= MIN_HEAP_SIZE ∧ size %
  MIN_HEAP_SIZE == 0`).
- **Err branch**: Only `e.code == ErrorCode::InvalidArgument`.

**Weakness**: The Err branch is weaker than ideal. The property analysis specifies
FN-2g as bidirectional (`Err <==> precondition violation`), but the implementation only
states `e.code == InvalidArgument`. The fix report (Issue 4) correctly explains why the
full bidirectional condition is unprovable: inner `Slab::from_raw_parts` calls can fail
even when kheap-level checks pass, unless LIVE-1 is proven (which requires bidirectional
Slab specs). This is **correctly classified** as a limitation, not a spec deficiency.

However, the Ok-branch forward implications do provide the contrapositive for kheap-level
checks (`Ok ==> checks passed`, therefore `¬checks ==> ¬Ok`). A caller can recover
the partial bidirectional condition. The gap is only for Slab-level failures, which are
unreachable in practice (LIVE-1). Acceptable.

### `allocate` (kheap.rs:266–314) — **Strong**

- **Requires**: `old(self).inv()` — standard invariant precondition.
- **Ok branch**: Slab index is valid, returned address was free in the correct slab
  (FN-3b), pointer is block-aligned (FN-3c), exact state transition via `spec_allocate`
  (FN-3d). The state transition encoding is excellent — `spec_allocate` uses
  `Seq::update` which implicitly encodes frame (all other slabs unchanged).
- **Err branch**: State preserved (`self@ == old(self)@`, FN-3g). Error iff size
  unsupported or slab exhausted (FN-3f). Bidirectional.
- **Invariant preserved**: `self.inv()` (FN-3e).
- **No anti-patterns**: Fully declarative, uses abstract state transitions.

### `deallocate` (kheap.rs:317–363) — **Strong**

Symmetric to `allocate`:

- **Ok branch**: Pointer was allocated in correct slab (FN-4b), exact state transition
  via `spec_deallocate` (FN-4c), invariant preserved (FN-4d).
- **Err branch**: State preserved (FN-4f), error iff unsupported size or pointer not
  allocated (FN-4e). Bidirectional.
- **No anti-patterns**.

### View Abstraction — **Well-designed**

`KheapView` with `Seq<SlabView>` is clean and compositional:
- Uses `ext_equal` for extensional equality.
- Enables quantified invariants over all slabs.
- `spec_allocate`/`spec_deallocate` encode state transitions using `Seq::update`,
  which implicitly preserves all non-target slabs (frame condition).
- `all_allocated`/`all_free` aggregate views are available for cross-slab reasoning.
- `KheapView::inv()` captures TYPE-1, TYPE-2, TYPE-3 cleanly.

### Overall Spec Quality: **A-**

The specs are strong, declarative, caller-oriented, with bidirectional error paths and
proper frame conditions. The only weakness is the Err branch of `from_raw_parts`, which
is correctly identified as a limitation rather than a spec deficiency.

---

## Property Coverage

### Detailed Property Mapping

| Property ID | Description | Status | Evidence |
|-------------|-------------|--------|----------|
| **TYPE-1** | KheapView well-formedness | ✅ Covered | `KheapView::inv()` spec.rs:198–203: `slabs.len() == NUM_OF_SLABS` ∧ `∀i: slabs[i].inv()` |
| **TYPE-2** | Slab region disjointness | ✅ Covered | `KheapView::inv()` spec.rs:205–206: `∀i: slabs[i].end_addr <= slabs[i+1].start_addr` |
| **TYPE-3** | Block-size sequence | ✅ Covered | `KheapView::inv()` spec.rs:208–209: `∀i: slabs[i].block_size == block_sizes()[i]`; `lemma_block_sizes_strictly_increasing` proves strict monotonicity |
| **TYPE-4** | Heap storage containment | ⚠️ Partial | `from_raw_parts` ensures slab containment relative to `addr` param (kheap.rs:138–141), but cannot tie `addr` to `HEAP_STORAGE` (static mut limitation) |
| **TYPE-5** | SlabSize enum discriminants | ❌ Not formalized | Rust compiler guarantee. Verus trusts the type system. Negligible risk |
| **TYPE-6** | HeapStorage alignment | ❌ Not formalized | `#[repr(align(4096))]` + `static_assert!`. Compiler guarantee. Negligible risk |
| **FN-1a** | Supported size → Ok | ✅ Covered | `layout_to_allocator` ensures Ok when `opt_idx.is_some()` (kheap.rs:370) |
| **FN-1b** | Slab large enough | ✅ Covered | kheap.rs:374: `block_sizes()[opt_idx.unwrap()] >= spec_layout_size(*layout)` |
| **FN-1c** | Tightest fit | ✅ Covered | kheap.rs:378–379: `∀j < idx: block_sizes()[j] < size`. Strengthened with `lemma_slab_for_size_tightest_fit` |
| **FN-1d** | Error iff unsupported | ✅ Covered | kheap.rs:382: `spec_slab_for_size(...).is_none()` — bidirectional |
| **FN-2b** | Constructor establishes inv | ✅ Covered | kheap.rs:133: `heap.inv()` |
| **FN-2c** | All slabs initially empty | ✅ Covered | kheap.rs:135–136: `∀i: allocated_addrs == Set::empty()` |
| **FN-2d** | Block sizes match sequence | ✅ Covered | Implied by `heap.inv()` which includes TYPE-3 (block_sizes match) |
| **FN-2e** | Slab containment in partition | ✅ Covered | kheap.rs:138–141: explicit start/end bounds per slab |
| **FN-2f** | Error code | ✅ Covered | kheap.rs:149: `e.code == ErrorCode::InvalidArgument` |
| **FN-2g** | Bidirectional failure | ⚠️ Partial | Forward direction only (Ok implies checks passed, kheap.rs:143–145). Reverse not provable due to Slab spec limitation (see Issue 4 discussion) |
| **FN-3b** | Returned addr was free | ✅ Covered | kheap.rs:278 |
| **FN-3c** | Block-aligned pointer | ✅ Covered | kheap.rs:280 |
| **FN-3d** | Exact state transition | ✅ Covered | kheap.rs:282 via `spec_allocate` |
| **FN-3e** | Invariant preserved | ✅ Covered | kheap.rs:272 |
| **FN-3f** | Error iff unsupported/exhausted | ✅ Covered | kheap.rs:289–291 — bidirectional |
| **FN-3g** | State preserved on error | ✅ Covered | kheap.rs:287 |
| **FN-4b** | Ptr was allocated | ✅ Covered | kheap.rs:329 |
| **FN-4c** | Exact state transition | ✅ Covered | kheap.rs:331 via `spec_deallocate` |
| **FN-4d** | Invariant preserved | ✅ Covered | kheap.rs:322 |
| **FN-4e** | Error iff unsupported/unallocated | ✅ Covered | kheap.rs:338–340 — bidirectional |
| **FN-4f** | State preserved on error | ✅ Covered | kheap.rs:336 |
| **FN-5a–c** | `GlobalAlloc::alloc` | ❌ Out of scope | `static mut` limitation. Thin wrapper |
| **FN-6a–c** | `GlobalAlloc::dealloc` | ❌ Out of scope | `static mut` limitation. Thin wrapper |
| **FN-7b–d** | `init()` postconditions | ❌ Out of scope | `static mut` limitation |
| **MOD-1** | Cross-slab alloc disjointness | ✅ Covered | `lemma_kheap_inv_implies_cross_slab_disjointness` proof.rs:33–65 |
| **MOD-2** | Cross-slab free disjointness | ✅ Covered | Same lemma, second conjunct |
| **MOD-3** | Global allocation uniqueness | ✅ Covered | Same lemma, all three conjuncts |
| **MOD-4** | No null address | ✅ Covered (conditional) | `lemma_no_null_address` proof.rs:199–223. Conditional on `base_addr > 0`. The runtime fact that HEAP_STORAGE has non-zero address is not axiomatized |
| **MOD-5** | Allocation conservation | ✅ Covered | `lemma_allocate_conserves` + `lemma_deallocate_conserves` proof.rs:104–158 |
| **MOD-6** | Routing consistency | ✅ Covered | `layout_to_allocator` is a pure function — deterministic by construction. Both `allocate` and `deallocate` use the same `spec_slab_for_size` |
| **MOD-7** | Memory-region containment | ⚠️ Partial | `from_raw_parts` ensures slabs within the `addr` parameter range. Cannot connect to HEAP_STORAGE bounds (static mut) |
| **LIVE-1** | Slab construction feasibility | ❌ Not proven | Argued convincingly in property_analysis.md §6 from constant analysis, but not machine-checked. Would require bidirectional Slab spec |
| **LIVE-2** | init() always succeeds | ❌ Not proven | Depends on LIVE-1 + static mut access |
| **LIVE-3** | Alloc succeeds when free blocks exist | ✅ Covered | Implied by FN-3f bidirectional: `Err ==> ... free_addrs == empty`, contrapositive gives `free_addrs ≠ empty ==> ¬Err` |
| **LIVE-4** | Dealloc succeeds for allocated ptr | ✅ Covered | Implied by FN-4e bidirectional: `Err ==> ¬allocated`, contrapositive gives `allocated ==> ¬Err` |
| **LIVE-5** | Alloc-dealloc round-trip | ✅ Covered | `lemma_alloc_dealloc_round_trip` proof.rs:78–100 |
| **LIVE-6** | Failure recoverability | ✅ Covered | FN-3g and FN-4f: state preserved on error, inv maintained |
| **GLOBAL-1–5** | Cross-module properties | ❌ Out of scope | Architectural properties requiring system-wide reasoning |

### Summary

| Category | Covered | Partial | Uncovered (in-scope) | Out of scope |
|----------|---------|---------|---------------------|--------------|
| TYPE-1–6 | 1, 2, 3 | 4 | 5, 6 | — |
| FN-1a–d | all 4 | — | — | — |
| FN-2b–g | b, c, d, e, f | g (partial) | — | — |
| FN-3b–g | all 6 | — | — | — |
| FN-4b–f | all 5 | — | — | — |
| FN-5, 6, 7 | — | — | — | all 9 |
| MOD-1–7 | 1, 2, 3, 4*, 5, 6 | 7 | — | — |
| LIVE-1–6 | 3, 4, 5, 6 | — | 1, 2 | — |
| GLOBAL-1–5 | — | — | — | all 5 |

\* MOD-4 is conditional (requires `base_addr > 0` precondition). Counted as covered since
the lemma is fully verified; the gap is the runtime fact that HEAP_STORAGE address > 0.

**Totals**: ~34 covered, 3 partial, 2 uncovered in-scope (LIVE-1, LIVE-2), 14 out of scope.

---

## Proof Completeness

**Remaining `admit()`: 0**

All 10 proof functions are fully verified with no escapes:

| # | Proof Function | Property | Status |
|---|---------------|----------|--------|
| 1 | `lemma_regions_ordered` | Transitive slab ordering | ✅ Verified |
| 2 | `lemma_kheap_inv_implies_cross_slab_disjointness` | MOD-1, MOD-2, MOD-3 | ✅ Verified |
| 3 | `lemma_slab_for_size_valid` | Index validity + size bound | ✅ Verified |
| 4 | `lemma_alloc_dealloc_round_trip` | LIVE-5 | ✅ Verified |
| 5 | `lemma_allocate_conserves` | MOD-5 (alloc) | ✅ Verified |
| 6 | `lemma_deallocate_conserves` | MOD-5 (dealloc) | ✅ Verified |
| 7 | `lemma_slab_for_size_tightest_fit` | FN-1c strengthened | ✅ Verified |
| 8 | `lemma_block_sizes_strictly_increasing` | TYPE-3 strengthened | ✅ Verified |
| 9 | `lemma_slab_for_size_total` | Totality of routing | ✅ Verified |
| 10 | `lemma_no_null_address` | MOD-4 (conditional) | ✅ Verified |

Cheating metrics: assume=0, external_body=2, admit=0, trusted=0, no_decreases=0. Clean.

---

## Trust Boundary Audit

### assume_specification (2)

| # | Item | Location | What It Assumes | Approved? | Assessment |
|---|------|----------|----------------|-----------|------------|
| 1 | `Layout::size` | spec.rs:83–85 | Uninterpreted accessor: `result == spec_layout_size(*layout)`. Only binds the method return to an abstract spec function — does not constrain the value | ✅ `[x]` in property_analysis.md | **Correct and minimal.** The ensures is the weakest possible — it names the return but makes no claims about concrete values. Sound. |
| 2 | `Error::new` | spec.rs:88–91 | Constructor: `result.code == code`. Only constrains the error code field | ✅ `[x]` in property_analysis.md | **Correct and minimal.** Does not constrain `reason` or other fields. Sound. |

### external_body (2)

| # | Item | Location | What It Assumes | Approved? | Assessment |
|---|------|----------|----------------|-----------|------------|
| 1 | `ExLayout` (external_type_specification + external_body) | spec.rs:59–61 | `Layout` is an opaque type — no field access, no ensures | N/A (type decl) | **Standard pattern** for foreign opaque types in Verus. No soundness concern. |
| 2 | `usize_to_mut_ptr` | spec.rs:95–100 | `result as usize == addr` — address round-trip preservation | ✅ `[x]` in property_analysis.md (added by fix report) | **Trivially correct.** `addr as *mut u8` followed by `result as usize` is identity. Cfg-gated workaround for Verus lacking usize→pointer cast. Body visible and correct. |

### axiom (0 custom)

No custom axioms. Standard vstd broadcast axioms used (`group_control_flow_axioms`,
`layout_of_primitives`, div_mod lemmas) — these are part of the trusted vstd library.

### Unapproved trust boundaries: **None**

All trust boundaries are either:
- Human-approved in the Needed Assumptions checklist, or
- Standard Verus patterns for foreign type declarations.

The `usize_to_mut_ptr` external_body was noted as missing from the assumptions list
in the R1 review and has since been added (fix report Issue 5). Confirmed present in
property_analysis.md line 727.

---

## Previous Issue Resolution

### Issue 1 — [Medium] 3 unverified wrapper functions

**Claimed**: Classification (E) Verus limitation — no change possible.

**Verification**: ✅ **Correctly classified.** `alloc`, `dealloc`, and `init` all access
`static mut HEAP` (kheap.rs:408–446). Verus has no ownership model for mutable statics.
The functions are thin wrappers delegating to the fully verified `Kheap::allocate`,
`Kheap::deallocate`, and `Kheap::from_raw_parts`. No regression — core contracts unchanged.

**Assessment**: Genuine limitation. The 4 verified functions cover the core logic. The
unverified wrappers add only global-state access and null-pointer fallback. The gap is
real but well-documented and inherent to current Verus capabilities.

### Issue 2 — [Low] MOD-4 (no null address) unproven

**Claimed**: Added `lemma_no_null_address` as a conditional proof.

**Verification**: ✅ **Fix confirmed.** `lemma_no_null_address` exists at proof.rs:199–223.
It takes `base_addr > 0` and `slab_size > 0` as preconditions plus the heap invariant and
slab layout constraints, then proves no slab contains address 0. The proof is sound:

- `slab.start_addr >= base_addr + i * slab_size >= base_addr > 0`
- `SlabView::inv()` constrains all addresses to `[start_addr, end_addr)`, so all > 0.

The lemma correctly separates the formal proof (conditional) from the runtime fact
(HEAP_STORAGE has non-zero address, a linker guarantee). This is the right approach —
axiomatizing the linker placement would introduce an unjustified trust boundary.

**Verification count**: Increased from 19 to 20 — confirmed this is the new lemma.

### Issue 3 — [Low] LIVE-1/LIVE-2 informal

**Claimed**: Classification (E) Verus limitation.

**Verification**: ✅ **Correctly classified.** The explanation is technically precise:
- LIVE-1 requires the Slab spec to be bidirectional (`¬error_conditions ==> Ok`). The
  current Slab spec only provides `Err ==> error_conditions`. Without modifying the
  upstream Slab crate, kheap cannot formally prove its `from_raw_parts` calls succeed.
- LIVE-2 depends on LIVE-1 and additionally requires `static mut` access.

The constant analysis in property_analysis.md §6 (LIVE-1, LIVE-2) convincingly argues
these hold from the known constants. This is a spec-level limitation, not a soundness gap.

No code changes, no regressions.

### Issue 4 — [Low] `from_raw_parts` error branch underspecified

**Claimed**: Classification (D) Reviewer suggestion not implementable.

**Verification**: ✅ **Correctly analyzed.** The fix report's argument is sound:
- Adding `Err ==> ¬checks` would be violated if an inner Slab::from_raw_parts fails
  after all kheap checks pass (the `?` operator propagates the inner error).
- Proving that inner calls always succeed when kheap checks pass is exactly LIVE-1,
  which requires bidirectional Slab specs.
- The current ensures (`e.code == InvalidArgument`) is the strongest provable statement.
- The Ok-branch forward implications provide the contrapositive for callers.

No code changes, no regressions.

### Issue 5 — [Info] `usize_to_mut_ptr` not in Needed Assumptions

**Claimed**: Documentation fix — added to property_analysis.md.

**Verification**: ✅ **Fix confirmed.** Line 727 of property_analysis.md now reads:
`[x] usize_to_mut_ptr — cfg-gated helper for addr as *mut u8 cast, external_body
with ensures result as usize == addr; trivially correct, workaround for Verus
lacking usize-to-pointer cast support`

Properly documented with `[x]` approval status.

### Issue 6 — [Info] TYPE-5/TYPE-6 not formalized

**Claimed**: Classification (D) No change needed.

**Verification**: ✅ **Correct.** TYPE-5 (enum discriminants) and TYPE-6 (struct alignment)
are Rust compiler guarantees. TYPE-5 relies on `repr(C)` / integer-repr enum semantics;
TYPE-6 relies on `#[repr(align(4096))]` enforced at compile time and checked by
`static_assert::assert_eq_align!`. Formalizing these would require axioms about
compiler-level layout rules, which Verus does not model.

No code changes, no regressions.

### Summary of Issue Resolution

| Issue | Claimed Fix | Verified? | Regression? |
|-------|------------|-----------|-------------|
| 1. Unverified wrappers | Verus limitation | ✅ Correct | None |
| 2. MOD-4 no null | Conditional lemma added | ✅ Confirmed | None |
| 3. LIVE-1/LIVE-2 | Verus limitation | ✅ Correct | None |
| 4. Err underspecified | Not implementable | ✅ Correct | None |
| 5. usize_to_mut_ptr doc | Added to assumptions | ✅ Confirmed | None |
| 6. TYPE-5/TYPE-6 | No change needed | ✅ Correct | None |

**No regressions detected. No weakened specs. All fixes are genuine.**

---

## Issues Found

### Remaining Issues (carried forward, no new issues)

1. **[Medium] 3 unverified wrapper functions** — `alloc`, `dealloc`, `init` lack
   contracts. This is a genuine Verus limitation (static mut). The core logic they
   delegate to is fully verified. **Status: known limitation, cannot fix without
   Verus evolution.**

2. **[Low] LIVE-1/LIVE-2 not machine-checked** — Slab construction feasibility and
   init infallibility are argued from constant analysis but not formally proven. Requires
   bidirectional Slab spec + static mut support. **Status: known limitation, convincingly
   argued informally.**

3. **[Low] FN-2g Err branch partial** — `from_raw_parts` error branch only says
   `e.code == InvalidArgument`, not the full bidirectional condition. The forward direction
   is available from the Ok branch. **Status: strongest provable statement given current
   Slab spec.**

4. **[Info] TYPE-5, TYPE-6 not formalized** — Compiler guarantees, negligible risk.

### New Issues Found in R2

**None.** The R1 fix report addressed all actionable items correctly. No new issues,
regressions, or weakened specs detected.

---

## Overall Assessment

### Grade: **B+**

### Rationale

**Strengths:**
- **Zero verification escapes**: 0 admits, 0 assumes, 0 trusted. Cheating metrics are
  pristine.
- **Strong bidirectional specs**: All four verified functions have rigorous bidirectional
  error paths with state preservation.
- **Clean abstraction**: `KheapView` / `Seq<SlabView>` is well-designed, compositional,
  and enables quantified reasoning.
- **Exact state transitions**: `spec_allocate`/`spec_deallocate` with `Seq::update`
  encode both the state change and the frame condition elegantly.
- **Comprehensive proof library**: 10 lemmas covering disjointness (MOD-1/2/3),
  conservation (MOD-5), round-trip (LIVE-5), tightest-fit, monotonicity, totality, and
  no-null (MOD-4 conditional). All fully verified.
- **Minimal trust boundary**: Only 2 `assume_specification` (both human-approved, both
  minimal) and 2 `external_body` (one standard type decl, one trivial cfg workaround).
- **Clean AST consistency**: All 4 mismatches are pre-approved deviations (named returns,
  closure naming, cfg-gating). No accidental exec modifications.
- **R1 fixes properly executed**: `lemma_no_null_address` added correctly,
  `usize_to_mut_ptr` documented, all classifications well-reasoned.

**Weaknesses preventing A:**
- **Coverage gap**: 3/7 exec functions (43%) unverified. While justified by Verus's
  static mut limitation, this leaves the entire `GlobalAlloc` API surface and
  initialization uncontracted. A caller of `alloc`/`dealloc` gets no machine-checked
  guarantees.
- **LIVE-1/LIVE-2 informal**: These are important liveness properties (the kernel heap
  *will* initialize successfully). The constant analysis is convincing but not
  machine-checked.
- **FN-2g partial**: The error branch of the constructor is weaker than the property
  analysis specifies.

**Why not A:** The 43% function coverage gap is significant — it means the module's
actual public API (GlobalAlloc) has no contracts. The core logic is verified, but the
trust gap between verified internals and unverified API wrappers is not bridged.

**Why not B:** The quality within the verified scope is excellent — the specs are
strong, the proofs are clean, and the trust boundary is minimal. The unverified functions
are thin wrappers with a clearly documented Verus limitation. The effort demonstrates
proper methodology throughout.

**Comparison to R1:** No change in grade. R1 identified real issues; the fix report
addressed all actionable ones correctly and classified the rest accurately. The
`lemma_no_null_address` addition is a genuine improvement (+1 verified item, MOD-4
now conditionally proven). No regressions.

### Result: **FAIL** (grade < A)

The verification effort is methodologically sound and high quality within its scope.
The B+ grade reflects genuine Verus tooling limitations rather than methodological
failures. A grade of A (PASS) would require either (a) Verus supporting `static mut`
so the wrapper functions can be contracted, or (b) a restructuring that moves global
state management out of kheap's verification scope.
