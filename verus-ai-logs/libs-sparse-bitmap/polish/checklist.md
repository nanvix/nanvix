# Polish Report: sparse-bitmap

## Proof Extraction
- Blocks extracted: 1
  - `sort_chunks_by_offset:509` (8 lines) → `lemma_sort_swap_step` in lib.proof.rs
- Blocks kept inline: 5 (each justified below)
  - `commit_cross_chunk_alloc:943` (17 lines) — contains ghost mutation (`committed = committed + take`); cannot extract
  - `try_alloc_cross_chunk_from:1141` (6 lines) — single `lemma_walk_fail_gap_case` call; line count from 9 parameters
  - `try_alloc_cross_chunk_from:1152` (6 lines) — same single lemma call, different branch
  - `try_alloc_cross_chunk_from:1212` (7 lines) — single `lemma_walk_fail_set_bit_case` call; 11 parameters
  - `try_alloc_cross_chunk_from:1249` (6 lines) — single `lemma_cross_chunk_range_is_free` call; 8 parameters

## Minimization
- Redundant assertions removed: 68
  - `commit_cross_chunk_alloc` postcondition block: 14 lines removed (18→4)
  - `commit_cross_chunk_alloc` entry insert block: 24 lines removed (29→5)
  - `commit_cross_chunk_alloc` pre-loop setup block: 24 lines removed entirely (24→0)
  - `commit_cross_chunk_alloc` inner pre-commit block: 20 lines removed (28→8→5)
  - `commit_cross_chunk_alloc` inner post-insert block: 33 lines removed (51→18)
  - `commit_cross_chunk_alloc` pre-commit free bits block: 7 lines removed entirely (7→0)
  - `commit_cross_chunk_alloc` entry Ok branch: 3 lines removed (6→3)
  - `commit_cross_chunk_alloc` inner Ok branch: 2 lines removed (5→3)
  - `try_alloc_cross_chunk_from` pfp update block: 23 lines removed (28→5)
  - `alloc_range` single-chunk Ok block: 1 line removed (6→5)
- Redundant lemmas/hints removed: 12
  - `lemma_seq_remove_insert_is_update`: removed from 2 call sites
  - `lemma_chunk_update_preserves_structure`: removed from 2 call sites
  - `lemma_seq_remove_insert_identity`: not removed (still needed)
  - `lemma_lifted_set_bits_alloc_range`: removed from 1 call site (entry insert)
  - Various bridge assertions (`pre_commit_chunks =~= old_chunks_seq`, etc.): removed from 5+ sites
  - `lemma_range_set_insert_end` assert-forall guards: removed from 2 loop bodies

## Verification Bug Fixed
- `try_alloc_cross_chunk_from`: loop invariant `trailing_free == entry_cap - bit_idx` doesn't hold at `break`.
  Moved 2 clauses from `invariant` to `invariant_except_break` (Verus requires `invariant_except_break` BEFORE `invariant`).

## Final State
- Verification: 90 verified, 0 errors
- Cheating: assume=0, external_body=0, admit=0, trusted=0
- Coverage: 17/17 exec functions have contracts
- Proof blocks: 46 total, 5 over threshold (all justified above), 41 OK
