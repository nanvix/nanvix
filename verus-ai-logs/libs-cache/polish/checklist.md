# Polish Report: cache

## Proof Extraction
- Blocks extracted: 0
- Blocks kept inline: 0
- Justification: All exec functions use `external_body`; there are zero inline
  `proof { }` blocks in `lib.rs`. All proofs already reside in `lib.proof.rs`.

## Minimization
- Redundant assertions removed: 16
  1. `assert(cache.lru_order.contains(key))` — post-call restatement in `lemma_spec_get_inv` (was line 216)
  2. `assert(cache.lru_order.contains(key))` — post-call restatement in `lemma_spec_put_inv` overwrite branch (was line 258)
  3. `assert(cache.lru_order.to_set().contains(key))` — derivable from inv() in `lemma_spec_get_inv` (was line 211)
  4. `assert(cache.lru_order.to_set().contains(key))` — derivable from inv() in `lemma_spec_put_inv` overwrite branch (was line 252)
  5. `assert(cache.lru_order.to_set().remove(key).insert(key) =~= ...)` — set identity in `lemma_spec_get_inv` (was line 212)
  6. `assert(cache.lru_order.to_set().remove(key).insert(key) =~= ...)` — set identity in `lemma_spec_put_inv` overwrite branch (was line 253)
  7. `assert(cache.lru_order.to_set().contains(victim))` — derivable from index in `lemma_spec_put_inv` eviction branch (was line 268)
  8. `assert(cache.contents.dom().contains(victim))` — derivable from inv() in `lemma_spec_put_inv` eviction branch (was line 269)
  9. `assert(key != victim)` — derivable in `lemma_spec_put_inv` eviction branch (was line 270)
  10. `assert(!cache.contents.dom().remove(victim).contains(key))` — redundant in `lemma_spec_put_inv` eviction branch (was line 292)
  11. `assert(new_lru.to_set() =~= result.contents.dom())` — summary assert in `lemma_spec_put_inv` eviction branch (was line 293)
  12. `assert(cache.lru_order.contains(key))` / `assert(cache.lru_order.to_set().contains(key))` — two inner asserts in `!sub.contains(key)` by-block (was lines 275-276)
  13. `assert(s.to_set().contains(key))` — bridge assert in `lemma_filter_neq_len` (was line 111)
  14. `assert(sub.contains(x))` / `assert(s.contains(x))` / `assert(x != s[0])` — three inner asserts in first forall-by in `lemma_drop_first_to_set` (was lines 146,149,150)
  15. `assert(s.contains(x) && x != s[0])` / `assert(idx >= 1int)` — two inner asserts in second forall-by in `lemma_drop_first_to_set` (was lines 156,158)
  16. `assert(filtered.contains(x))` — inner assert in first forall-by in `lemma_filter_neq_to_set` (was line 78)
  17. `assert(!cache.lru_order.contains(key))` — full by-block in `lemma_spec_put_inv` below-capacity branch (was lines 303-307)

- Redundant let bindings removed: 7
  1. `let result = cache.spec_get(key).0` in `lemma_spec_get_inv`
  2. `let mru = cache.move_to_mru(key)` in `lemma_spec_get_inv`
  3. `let result = cache.spec_put(key, value)` in `lemma_spec_put_inv` overwrite branch
  4. `let mru = cache.move_to_mru(key)` in `lemma_spec_put_inv` overwrite branch
  5. `let result = cache.spec_put(key, value)` in `lemma_spec_put_inv` eviction branch
  6. `let victim = cache.lru_order[0]` in `lemma_spec_put_inv` eviction branch
  7. `let result = cache.spec_put(key, value)` / `let new_lru = ...` in `lemma_spec_put_inv` below-capacity branch
  8. `let result = cache.spec_remove(key)` in `lemma_spec_remove_inv`

- Redundant lemmas/hints removed: 0
- Dead spec functions removed: 0

## Kept (required by solver)
- `assert(cv.contents.dom() =~= Set::<K>::empty())` / `assert(cv.lru_order.to_set() =~= Set::<K>::empty())` in `lemma_spec_new_inv` and `lemma_spec_clear_inv` — extensional equality hints needed by SMT
- `assert(cache.contents.insert(key, value).dom() =~= cache.contents.dom())` in `lemma_spec_put_inv` overwrite branch — needed for domain equality
- All `assert(!filtered.contains(key))` by-blocks in get/put — needed to satisfy `lemma_push_preserves_no_dup` precondition

## Verification
- Final result: 11 verified, 0 errors, 0 admits
