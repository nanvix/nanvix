## Response to Turn 1

### Goal
Split `Cache::evict` to remove `external_body` from the entire function, isolating only the 3-line iterator chain into a minimal `external_body` helper (`find_lru_victim`).

### Attempts 1-6: Recursive induction on `s.drop_last()`
- Strategy: Prove `s.filter(|k| k != first) =~= s.subrange(1, s.len())` by recursion following `Seq::filter`'s own recursive structure (via `drop_last`/`last`).
- Variations tried:
  1. Direct recursion with `s.drop_last()` and nested subrange indexing
  2. Explicit `subrange(0, n-1)` instead of `drop_last()` with manual index assertions
  3. `lemma_slice_of_slice` to compose nested subranges without manual indexing
  4. Fixed `first: K` parameter to avoid closure identity issues across recursion levels
  5. Explicit `=~=` chaining assertions to help the solver connect intermediate facts
  6. Targeted `assert ... by {}` blocks with focused `reveal`
- Result: All failed with the same postcondition error. Root cause: the solver cannot chain the transitive equalities needed to connect `dl.filter(pred) =~= dl.subrange(1, dl.len()) =~= s.subrange(1, n-1)` with `s.filter(pred) == dl.filter(pred).push(s.last())` and `s.subrange(1, n-1).push(s.last()) =~= s.subrange(1, n)`. Each intermediate `=~=` produces `==` via extensionality, but Z3 fails to close the final chain.

### Attempt 7: Non-recursive proof via `filter_distributes_over_add` ✅
- **Key insight**: Instead of following `filter`'s recursive structure, use compositional reasoning. Split `s` at position 1 and reason about each half independently:
  1. `s == s.subrange(0,1) + s.subrange(1,n)` (via `lemma_split_at`)
  2. `s.filter(pred) == head.filter(pred) + tail.filter(pred)` (via `filter_distributes_over_add`)
  3. `head = [first]`, `pred(first) == false` → `head.filter(pred) == empty` (via `reveal_with_fuel(Seq::filter, 2)`)
  4. `tail` has no element equal to `first` (by `no_duplicates`) → `tail.filter(pred) == tail` (via existing `lemma_filter_neq_absent`)
  5. Combine: `s.filter(pred) == empty + tail == tail == s.subrange(1, n)`

- Changes in **`lib.rs`** (lines 308–360):
  - Added `find_lru_victim` (lines 308–331): `external_body` helper with full specs isolating only the iterator chain
  - Rewrote `evict` (lines 338–360): removed `external_body`, body calls `find_lru_victim` + `btreemap_remove` + proof block with `lemma_evict_view`

- Changes in **`lib.proof.rs`** (lines 478–598):
  - Replaced recursive `lemma_filter_neq_first_is_subrange` with non-recursive proof using `filter_distributes_over_add`, `reveal_with_fuel(Seq::filter, 2)`, and `lemma_filter_neq_absent`
  - `lemma_filter_first_is_subrange` wrapper unchanged
  - Added `lemma_evict_view` proof for evict postconditions (uses `axiom_cache_lru_of_remove`, `lemma_filter_first_is_subrange`, `lemma_subrange_no_dup`, `lemma_drop_first_to_set`)

- Result: **22 verified, 0 errors** ✅

### Final Result
- Status: **FIXED**
- external_body count change: was 8 → now 8 (swapped `evict` for `find_lru_victim`; count unchanged but trust boundary is dramatically narrower — only 3-line iterator chain vs entire function body)
- Verification: **22 verified, 0 errors** (up from 18 verified)
- AST consistency: ✅ All 20 functions and 3 structs MATCH (no exec code modified)

```
verification results:: 22 verified, 0 errors

=== Cheating Pattern Check ===
  ⚠️  external_body: 8
  Affected functions:
    - axiom_cache_lru_of_remove (line 402): external_body
    - deref (line 97): external_body
    - btreemap_remove (line 121): external_body
    - get (line 208): external_body
    - put (line 238): external_body
    - find_lru_victim (line 325): external_body

=== Function Coverage ===
  9/10 exec functions have contracts.
```

### Evidence

The key proof technique that succeeded where recursion failed:

**`filter_distributes_over_add`** (vstd `seq_lib.rs:292`) decomposes `s.filter(pred)` into independently provable pieces by splitting `s` at index 1. This avoids:
1. Nested subrange indexing that doesn't auto-trigger axioms
2. Closure identity issues across recursion levels
3. Transitive `=~=` chaining that the solver can't close

The complete proof for `lemma_filter_neq_first_is_subrange` is now 5 clean steps, each independently verifiable by the SMT solver.
