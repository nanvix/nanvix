# Polish Report: kpool

## Proof Extraction
- Blocks extracted: 5 (into named lemmas in kpool.proof.rs)

| Original location | Lines | New lemma |
|---|---|---|
| `alloc()` overflow check (L182–189 orig) | 8 | `lemma_index_mul_no_overflow` |
| `alloc()` addr alignment (L191–193 orig) | 3 | `lemma_addr_page_aligned` |
| `alloc()` page_index identity (L195–198 orig) | 4 | `lemma_page_index_eq` |
| `free()` negative-div proof (L469–488 orig) | 19 | `lemma_negative_div` |
| `alloc_range()` loop body offset (L351–358 orig) | 8 | `lemma_offset_sum` |

- Blocks kept inline: 16
  - Most are ≤5 lines or single assertions bridging exec→spec values
  - `alloc_range()` post-loop proof (37 lines): kept inline because extracting triggers SMT quantifier interference that breaks the `assert forall...by` block
  - `alloc_range()` error path (7 lines): kept inline — extracting (`lemma_no_free_range`) destabilizes the same post-loop proof via leaked quantifier triggers
  - `free()` clear-OK proof (10 lines): tightly coupled to match arm context; assertions form a single reasoning chain

## Minimization
- Redundant assertions removed: ~20
  - `new()`: removed 3 redundant assertions (trivially implied by bitmap postcondition)
  - `alloc()`: removed ~5 intermediate assertions replaced by lemma calls
  - `alloc_range()`: removed ~4 intermediate assertions in loop body (replaced by lemma calls)
  - `alloc_range()` error path: removed ~8 stepping-stone assertions
  - `free()`: removed 2 redundant bridging assertions, all inline comments
- Redundant lemmas/hints removed: 2
  - `lemma_mul_distributive`: subsumed by `lemma_offset_sum`
  - `lemma_no_free_range`: unused (kept inline due to SMT interference)

## Verification Result
- **29 verified, 0 errors** (down from 31 — removed 2 unused lemmas)
- No admits, no spec weakening
