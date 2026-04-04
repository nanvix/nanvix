# Final Comprehensive Review: kheap

## Grade: B+

## Spec Quality

**Strong.** The four verified functions (`layout_to_allocator`, `from_raw_parts`,
`allocate`, `deallocate`) have well-crafted external-top contracts:

- **Bidirectional error paths**: All four functions specify both success and error
  conditions with explicit state preservation (`self@ == old(self)@`) on error.
- **Exact state transitions**: `allocate`/`deallocate` use `spec_allocate`/
  `spec_deallocate` on `KheapView`, encoding frame conditions implicitly via
  `Seq::update` — only the target slab changes.
- **Declarative abstractions**: Specs use `KheapView`, `spec_slab_for_size`,
  `block_sizes()` — mathematical abstractions independent of implementation.
- **Caller-oriented**: A caller can reason about routing (`layout_to_allocator`),
  state transitions (`allocate`/`deallocate`), and constructor guarantees
  (`from_raw_parts`) without reading the implementation.
- **No anti-patterns detected**: No tautological postconditions, no one-sided
  error specs, no code-as-spec, no missing frame conditions.

**Weaknesses:**
1. `from_raw_parts` error branch only specifies `e.code == ErrorCode::InvalidArgument`;
   the full bidirectional failure condition (FN-2g) is encoded indirectly via the Ok
   branch's forward implications. Explicitly stating `Err ⟹ ¬preconditions` would
   improve readability.
2. `spec_layout_size` is uninterpreted — callers cannot reason about concrete Layout
   sizes. This is inherent to Layout's opacity and acceptable.
3. Three thin wrapper functions (`alloc`, `dealloc`, `init`) are completely unverified
   due to `static mut` limitations, leaving the GlobalAlloc API surface uncontracted.

**View Abstraction**: `KheapView` with `Seq<SlabView>` is well-designed — clean,
compositional, uses `ext_equal`, and enables quantified invariants.

## Property Coverage

- **Covered: 33 / 40 in-scope verifiable** (out of 59 total property IDs)
- **Not covered (in-scope)**: TYPE-5, TYPE-6, MOD-4, LIVE-1, LIVE-2
- **Not covered (out-of-scope by design)**: FN-5a–c, FN-6a–c, FN-7b–d, GLOBAL-1–5 (14 properties)
- **Documented observations**: BUG-1–5 (5 bugs acknowledged, not formal properties)
- **Partially covered**: TYPE-4, MOD-7

Detailed mapping:

| Category | Covered | Partial | Uncovered (in-scope) | Out-of-scope |
|----------|---------|---------|---------------------|--------------|
| TYPE-1–6 | 1,2,3 | 4 | 5, 6 | — |
| FN-1a–d | all 4 | — | — | — |
| FN-2b–g | all 6 | — | — | — |
| FN-3b–g | all 6 | — | — | — |
| FN-4b–f | all 5 | — | — | — |
| FN-5,6,7 | — | — | — | all 9 |
| MOD-1–7 | 1,2,3,5,6 | 7 | 4 | — |
| LIVE-1–6 | 3,4,5,6 | — | 1, 2 | — |
| GLOBAL-1–5 | — | — | — | all 5 |

**Notes on disagreement between reviewers:**
- The GPT reviewer counted only 11 properties because it searched for literal property
  ID strings in spec/proof files. The Claude reviewer mapped ensures clauses to property
  IDs (e.g., `kheap.rs:278` covers FN-3b, `kheap.rs:280` covers FN-3c). The Claude
  methodology is correct — properties are covered by ensures clauses on exec functions,
  not by explicit property-ID comments.
- The 14 out-of-scope properties (FN-5/6/7, GLOBAL-*) require modelling `static mut`
  global state, which Verus cannot currently do. The 5 BUGs are documented observations,
  not verification targets.

## Proof Completeness

- **Remaining admit(): 0**
- Clean — no verification escapes of any kind.

9 proof functions, all fully verified:

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

## Trust Boundary Audit

- **assume_specification: 2** (both human-approved)
- **axiom: 0** (custom; vstd built-in axioms used via `broadcast use`)
- **external_body: 2** (1 type spec, 1 helper function)

| Item | Location | Approved? | Assessment |
|------|----------|-----------|------------|
| `Layout::size` assume_spec | spec.rs:83–85 | ✅ `[x]` in property_analysis.md | Minimal — uninterpreted accessor, correct |
| `Error::new` assume_spec | spec.rs:88–91 | ✅ `[x]` in property_analysis.md | Minimal — only constrains `.code` field |
| `ExLayout` external_type_spec + external_body | spec.rs:59–61 | N/A (type decl) | Standard pattern for opaque foreign types |
| `usize_to_mut_ptr` external_body | spec.rs:95–100 | ⚠️ Not in Needed Assumptions list | Trivially correct (`addr as *mut u8` preserves address); cfg-gated workaround for Verus limitation. No blocker. |

- **Unapproved items: 0 BLOCKER**

`usize_to_mut_ptr` is not listed in the Needed Assumptions checklist but its ensures
(`result as usize == addr`) is universally true for Rust's integer-to-pointer cast.
This should be added to the assumptions list for documentation completeness, but is
not a soundness concern.

## Exec Fidelity

- **AST check: PASS**

Tool output:
```
Functions: 3 MATCH, 4 MISMATCH, 0 missing, 0 extra
Structs:   3 MATCH, 0 MISMATCH, 0 missing, 0 extra
```

All 4 mismatches are pre-approved or documented:

| Function | Deviation | Classification |
|----------|-----------|----------------|
| `layout_to_allocator` | Named return `-> (result: ...)` | Pre-approved |
| `from_raw_parts` | Named return; `mem::PAGE_SIZE` cfg-gated; `addr as *mut u8` cfg-gated; `info!()` cfg-gated | Pre-approved + legitimate cfg-gating |
| `allocate` | Named return; `\|_\|` → `\|_e\|` (10 closures) | Pre-approved + documented Verus limitation |
| `deallocate` | Named return; `\|_\|` → `\|_e\|` (10 closures) | Pre-approved + documented Verus limitation |

Original exec code preserved under `#[cfg(not(verus_keep_ghost))]` in all cases.
No accidental exec modifications. No missing or extra elements.

## Verification

- **verus: PASS** — `make verify-kernel MODULE=mm::kheap` → **19 verified, 0 errors**

```
=== Results ===
  19 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0
  coverage: 4/7 exec functions have contracts
```

The 3 uncontracted functions (`alloc`, `dealloc`, `init`) are thin wrappers accessing
`static mut` globals — a known Verus limitation. The core allocator logic they delegate
to is fully verified.

## Issues (highest priority first)

1. **[Medium] 3 unverified wrapper functions** — `GlobalAlloc::alloc`, `GlobalAlloc::dealloc`,
   `init()` have no Verus contracts. Core logic is verified, but the GlobalAlloc API
   surface and initialization are not machine-checked. Properties FN-5/6/7, LIVE-2 are
   unverified.

2. **[Low] MOD-4 (no null address) unproven** — No proof that the allocator never returns
   address 0. Follows from HEAP_STORAGE having a non-zero address (linker placement), but
   not formalized.

3. **[Low] LIVE-1/LIVE-2 informal** — Slab construction feasibility and init() infallibility
   are argued convincingly in property_analysis.md but lack machine-checked proofs.

4. **[Low] `from_raw_parts` error branch underspecified** — Err branch says only
   `e.code == InvalidArgument`; bidirectional failure condition (FN-2g) is only
   recoverable by contrapositive from the Ok branch. An explicit Err condition
   would improve spec readability.

5. **[Info] `usize_to_mut_ptr` not in Needed Assumptions list** — Should be documented
   for completeness. No soundness risk.

6. **[Info] TYPE-5, TYPE-6 not formalized** — Enum discriminant correctness and struct
   alignment are Rust compiler guarantees, not Verus-verified. Negligible risk.

## Result: FAIL

**Rationale**: The verification effort is high quality — clean proofs, zero admits, strong
bidirectional specs, proper abstractions, and minimal trust boundaries. However, strict
grading requires grade A for PASS, and the following gaps prevent that:

- 3 out of 7 exec functions (43%) lack any contracts, leaving the entire GlobalAlloc
  API surface and initialization unverified
- MOD-4 (no-null safety property) is unproven
- LIVE-1/LIVE-2 (construction feasibility, init infallibility) lack formal proofs
- `from_raw_parts` error branch is weaker than the property analysis specifies

Grade **B+** reflects a strong effort with meaningful verification of core logic, held
back by incomplete function coverage and a few unproven safety/liveness properties. The
unverified functions are justified by Verus's `static mut` limitation — this is a tooling
gap, not a methodology failure.

---

*Consolidated from independent reviews by Claude Opus 4.6 (final_review.claude.md) and
GPT-5.3-Codex (final_review.gpt.md).*
