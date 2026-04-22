# Polish Report: cache

## Proof Extraction
- Blocks extracted: 0
- Blocks kept inline: 3 (with justification)
  1. `new()` (3 lines): Single lemma call — below threshold.
  2. `clear()` (3 lines): Single lemma call — below threshold.
  3. `remove()` (3 lines, **reduced from 6**): Removed 3 redundant `reveal()` calls
     (`View::view`, `cache_contents_of`, `cache_lru_of`) that were already
     performed inside `lemma_remove_view`. Block reduced from 6 → 3 lines,
     now below the extraction threshold.

## Minimization
- Redundant assertions removed: 11
  - `remove()` inline block: 3 redundant `reveal()` calls (duplicated in `lemma_remove_view`)
  - `lemma_remove_view` present branch: 4 assertions removed
    - `cache_contents_of(new_self.entries) =~= cache_contents_of(old_entries).remove(key)`
    - `new_self@.contents =~= old_view.contents.remove(key)`
    - `new_self@.lru_order =~= filtered`
    - `new_self@.capacity == old_view.capacity`
  - `lemma_remove_view` absent branch: 7 assertions removed
    - `cache_contents_of(old_entries).dom() =~= btreemap_view_spec(old_entries).dom()`
    - `btreemap_view_spec(new_self.entries) == btreemap_view_spec(old_entries)`
    - `cache_contents_of(new_self.entries) =~= cache_contents_of(old_entries)`
    - `!old_view.lru_order.contains(key)`
    - `cache_lru_of(new_self.entries) == cache_lru_of(old_entries)`
    - `new_self@.contents/lru_order/capacity =~= old_view.*` (3 field-by-field assertions)
    - `new_self@ =~= old_view`
    - `new_self@ =~= old_view.spec_remove(key)` (also removed from present branch)
- Redundant lemmas/hints removed: 0
- Dead spec functions removed: 0

## Kept (required by solver)
- `assert(cv.contents.dom() =~= Set::empty())` / `assert(cv.lru_order.to_set() =~= Set::empty())` in `lemma_spec_new_inv`, `lemma_spec_clear_inv`, `lemma_new_view`, and `lemma_clear_view` — extensional equality hints needed by SMT for `inv()` proof
- `assert(cache.contents.insert(key, value).dom() =~= cache.contents.dom())` in `lemma_spec_put_inv` overwrite branch — needed for domain equality
- All `assert(!filtered.contains(key))` by-blocks in get/put — needed to satisfy `lemma_push_preserves_no_dup` precondition

## Verification
- Final result: 18 verified, 0 errors, 0 admits
- All inline proof blocks: 3 total, all ≤3 lines
