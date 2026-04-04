# Polish & Strengthen Report: kheap

## Strengthening

| Property ID | Status | Location or Reason |
|-------------|--------|--------------------|
| FN-1c (tightest fit) | STRENGTHENED | `layout_to_allocator` ensures: added `forall\|j\| 0 <= j < opt_idx ==> block_sizes()[j] < size` |
| FN-1c (lemma) | NEW | `lemma_slab_for_size_tightest_fit` in proof.rs — previous tier too small |
| TYPE-3 (monotonicity) | NEW | `lemma_block_sizes_strictly_increasing` — block_sizes() strictly increasing |
| LIVE-2 (totality) | NEW | `lemma_slab_for_size_total` — spec_slab_for_size defined for [1, max_slab_size()] |

Already proven (no change needed):
- TYPE-1, TYPE-2, TYPE-3 (core): KheapView::inv()
- FN-1 a/b/c/d: layout_to_allocator ensures
- FN-2 b/c/e/f/g: from_raw_parts ensures
- FN-3 a-g: allocate ensures (state transition, frame, error)
- FN-4 a-f: deallocate ensures (state transition, frame, error)
- MOD-1/2/3: lemma_kheap_inv_implies_cross_slab_disjointness
- MOD-5: lemma_allocate_conserves, lemma_deallocate_conserves
- LIVE-5: lemma_alloc_dealloc_round_trip

Excluded (documented in property_analysis.md):
- MOD-4 (no null): requires non-zero HEAP_STORAGE address — architectural assumption
- TYPE-4 (storage containment): requires HEAP_STORAGE address modeling
- FN-5/6/7 (GlobalAlloc, init): use `static mut` — cannot model in Verus
- GLOBAL-1..5: cross-module/architectural properties

## Proof Extraction

- Blocks extracted: 0
- Blocks kept inline: 1 (with justification)

  | Block | Location | Lines | Justification |
  |-------|----------|-------|---------------|
  | from_raw_parts ptr::add setup | kheap.rs:192–207 | 16 (was 35) | Extraction attempted (3 strategies: quantified lemma, enumerated-ensures lemma, hybrid). All fail: Verus SMT solver requires these assertions in the same verification context as the function body. Lemma `ensures` don't propagate as SMT-level facts for ptr::add preconditions and the function's postcondition. Documented Verus limitation. |

## Minimization

- Redundant assertions removed: 19 (from the from_raw_parts proof block)
  - 12 ptr::add bound assertions (`addr + i*slab_size*size_of::<u8>() <= usize::MAX` and `i*slab_size*size_of::<u8>() <= isize::MAX` for i=1..6): Verus derives these automatically from `size_of::<u8>() == 1`, `i*slab_size <= size`, and the function's requires.
  - 7 comment lines removed as part of the block reduction.
- Redundant lemmas/hints removed: 0 (all existing lemmas prove distinct properties)

## Summary

- Verification: **19 verified, 0 errors, 0 admits**
- Build: **cargo build passes**
- AST consistency: **no new exec deviations** (all 4 mismatches pre-existing)
- Coverage rate: **57%** (4/7 exec functions have contracts; remaining 3 use `static mut` global state)
- Cheating: assume=0, external_body=2 (pre-existing: ExLayout, usize_to_mut_ptr), admit=0, trusted=0
