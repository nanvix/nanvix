## Turn 2: external_body on Cache::remove — Step 1 (exact error + fix direction)

### Progress Tracker
- external_body on user functions: **6 remaining**
  1. ~~`Cache::new` — ELIMINATED (turn 1)~~
  2. `Cache::get` — lib.rs:171
  3. `Cache::put` — lib.rs:211
  4. `Cache::remove` — lib.rs:257
  5. `Cache::clear` — lib.rs:274
  6. `Cache::evict` — lib.rs:292
  7. `CacheGuard::deref` — lib.rs:91
- Spec quality issues: 0
- Current drill-down: function `Cache::remove`, step 1/6

### Verification of Previous Fix

**CONFIRMED.** The fixer successfully eliminated `external_body` from `Cache::new`.

1. `#[verus_verify(external_body)]` is **gone** from `Cache::new` (lib.rs:141–156).
2. `make verify-cache` passes: **13 verified, 0 errors** (exit 0).
3. `external_body` count: **8 total** (was 9). 6 on user functions (was 7). ✅ Decreased by 1.
4. New infrastructure added:
   - `btreemap_view_spec<K,V>` — uninterp spec fn (orphan-rule workaround for View)
   - `assume_specification` for `BTreeMap::new` — matches vstd
   - `cache_contents_of`, `cache_lru_of` — closed spec fns connecting concrete→abstract
   - Cache View changed from `uninterp` to `closed spec fn` with interpreted body
   - `lemma_new_view` proof fn proving postconditions

The infrastructure laid down (interpreted Cache View, `btreemap_view_spec`, proof lemma
pattern) should now make the next functions easier.

### Issue

**`Cache::remove` (lib.rs:257–267)** has `external_body`. The function body is:
```rust
pub fn remove(&mut self, key: &K) {
    self.entries.remove(key);
}
```

This is a **single BTreeMap call** — the simplest remaining target after `new`.

**Step 1 (completed by reviewer):** I removed `external_body` and ran `make verify-cache`.
Exact error:

```
error: `alloc::collections::btree::map::impl&%20::remove` is not supported
  (note: you may be able to add a Verus specification to this function
   with `assume_specification`)
  = help: pub assume_specification<K, V, A, Q>
          [alloc::collections::BTreeMap::<K, V, A>::remove]
          (_0: &alloc::collections::BTreeMap<K, V, A>, _1: &Q)
          -> core::option::Option<V>
          where A: core::alloc::Allocator + core::clone::Clone,
```

Same pattern as `new` — Verus tells us exactly what to do.

### Specific Question

**Remove `external_body` from `Cache::remove` and make it verify.**

Specifically:

1. Add `assume_specification` for `BTreeMap::remove` in `lib.spec.rs`. The cache
   only calls `self.entries.remove(key)` where `key: &K` and `K: Ord + Clone`, so
   `Key == Q`. You can simplify the vstd spec (which uses the complex
   `borrowed_key_removed` / `Borrow<Q>` machinery) to use `btreemap_view_spec`
   directly:
   ```
   btreemap_view_spec(m) == btreemap_view_spec(old(m)).remove(*k)
   ```

2. Remove `#[verus_verify(external_body)]` from `Cache::remove`.

3. Add a proof lemma (following the `lemma_new_view` pattern) to connect the
   `btreemap_view_spec` postcondition to the `CacheView::spec_remove` ensures.
   You will need to `reveal` the closed view functions and prove that
   `cache_contents_of` and `cache_lru_of` are correctly updated after the
   BTreeMap remove.

4. Run `make verify-cache` and report the **exact** result.

**Note on `cache_lru_of`:** The non-empty case uses `cache_lru_of_nonempty` which
is uninterpreted. You will likely need additional `assume_specification` axioms
connecting `cache_lru_of` to `cache_lru_of_nonempty` after a remove operation,
or you may need to restructure. Report the exact error if this blocks you.

### Evidence

**Commands run by reviewer:**

1. Verified fixer's turn 1 result:
```
$ make verify-cache
verification: cached (no recompilation), — (exit 0)
cheating: assume=0 external_body=8 admit=0 trusted=0 no_decreases=0 cfg_gate=0
```

2. Confirmed `external_body` removed from `Cache::new`:
```
$ grep -n external_body src/libs/cache/src/lib.rs
91:    #[verus_verify(external_body)]     ← deref
171:    #[verus_verify(external_body)]    ← get
211:    #[verus_verify(external_body)]    ← put
257:    #[verus_verify(external_body)]    ← remove
274:    #[verus_verify(external_body)]    ← clear
292:    #[verus_verify(external_body)]    ← evict
```
(6 user functions, down from 7)

3. Removed `external_body` from `Cache::remove` and ran verification:
```
error: `alloc::collections::btree::map::impl&%20::remove` is not supported
  = help: pub assume_specification<K, V, A, Q>
          [alloc::collections::BTreeMap::<K, V, A>::remove] ...
```

4. Checked vstd `BTreeMap::remove` spec (vstd/std_specs/btree.rs:776-791):
   Uses `borrowed_key_removed` + `maps_borrowed_key_to_value` with `Borrow<Q>`.
   For `Key == Q` case, axiom_deref_key_removed (line 755-763) simplifies to:
   `new_m == old_m.remove(*k)`.

5. Checked vstd `BTreeMap::clear` spec (vstd/std_specs/btree.rs:793-798):
   Trivial: `m@ == Map::empty()`. **Next target after remove.**
