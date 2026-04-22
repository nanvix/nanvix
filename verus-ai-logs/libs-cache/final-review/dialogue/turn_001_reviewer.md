## Turn 1: external_body on Cache::evict — Step 2 (isolate unverifiable iterator chain)

### Progress Tracker
- external_body on user functions: **4 remaining** (each is a defect)
  1. `CacheGuard::deref` — lib.rs:93
  2. `Cache::get` — lib.rs:190
  3. `Cache::put` — lib.rs:230
  4. `Cache::evict` — lib.rs:318
- external_type_specification: 4 (expected, not defects)
- Infrastructure external_body: 2 (btreemap_remove wrapper, axiom_cache_lru_of_remove proof axiom — acceptable)
- Spec quality issues: 0
- Current drill-down: function `Cache::evict`, step 2/6

### Verification of Previous Fix
First turn of this dialogue round. Previous rounds (turns 1–2) successfully
eliminated `external_body` from `Cache::new`, `Cache::remove`, and `Cache::clear`.
Confirmed baseline: 18 verified, 0 errors, external_body=8 total (4 on user functions).

### Triage of All 4 Remaining external_body User Functions

I removed `external_body` from each function independently and ran `make verify-cache`.
Here is the exact error for each:

| Function | Error | Line | Root Cause |
|---|---|---|---|
| `Cache::evict` | `only variables are supported here, not general patterns` | 336 | Pattern destructuring `\|(_, e)\|` in closure arg of iterator chain |
| `Cache::put` | `&mut types, except in special cases` | 245 | `self.entries.get_mut(&key)` returns `Option<&mut CacheEntry<V>>` |
| `Cache::get` | `&mut types, except in special cases` | 209 | `self.entries.get_mut(key)` returns `Option<&mut CacheEntry<V>>` |
| `CacheGuard::deref` | `field expression for an opaque datatype` | 98 | `self.value` — CacheGuard is `external_body` struct (due to `&'a mut V` field) |

**Tractability assessment:**
- `Cache::evict`: **HIGH** — only the 3-line iterator chain is unverifiable; the conditional remove (`if let Some(key) = ... { self.entries.remove(&key); }`) is verifiable using the existing `btreemap_remove` wrapper.
- `Cache::put`: **MEDIUM** — `get_mut` returns `&mut` (true blocker). Needs a wrapper that encapsulates `get_mut + in-place mutation` into a single `external_body` call. Rest of function (capacity check, eviction call, insertion) is verifiable.
- `Cache::get`: **LOW** — same `get_mut` blocker PLUS must construct `CacheGuard` which is an opaque type. Two compounding blockers.
- `CacheGuard::deref`: **LOW** — struct itself is opaque due to `&mut` in field. Field access is fundamentally blocked until Verus supports `&mut` in struct fields.

### Issue

**`Cache::evict` (lib.rs:318–341)** has `external_body` on the entire function,
but only the iterator chain (lines 333–337) is unverifiable. The conditional
remove (lines 338–340) is verifiable — `btreemap_remove` wrapper already exists.

The function body is:
```rust
fn evict(&mut self) {
    let victim: Option<K> = self
        .entries
        .iter()                                    // ← no Verus spec for BTreeMap::iter
        .min_by_key(|(_, e)| e.last_used)          // ← pattern destructuring + iterator combinator
        .map(|(k, _)| k.clone());                  // ← pattern destructuring + Option::map
    if let Some(key) = victim {
        self.entries.remove(&key);                  // ← verifiable (btreemap_remove exists)
    }
}
```

**Step 2 — Isolate:** The ENTIRE function is NOT unverifiable. Only the iterator
chain is blocked. This is a textbook case for splitting into a verified outer
function + minimal `external_body` inner helper.

### Specific Question

**Split `Cache::evict` into a verified outer function + minimal external_body helper.**

Specifically:

1. Create a minimal `external_body` helper function that isolates ONLY the
   iterator chain (1–3 lines of exec code):
   ```rust
   #[verus_verify(external_body)]
   #[verus_spec(ret =>
       ensures
           btreemap_view_spec(*entries).dom().len() > 0 ==> {
               &&& ret is Some
               &&& cache_lru_of(*entries).len() > 0
               &&& ret->Some_0 == cache_lru_of(*entries)[0]
           },
           btreemap_view_spec(*entries).dom().len() == 0 ==> ret is None,
   )]
   fn find_lru_victim(entries: &BTreeMap<K, CacheEntry<V>>) -> Option<K> {
       entries.iter().min_by_key(|(_, e)| e.last_used).map(|(k, _)| k.clone())
   }
   ```

2. Remove `#[verus_verify(external_body)]` from `Cache::evict` and rewrite the
   body to call the helper + `btreemap_remove` (VERUS REWRITE, same pattern as
   `Cache::remove`):
   ```rust
   fn evict(&mut self) {
       // VERUS REWRITE: extracted iterator chain into find_lru_victim
       if let Some(key) = Self::find_lru_victim(&self.entries) {
           // VERUS REWRITE: originally self.entries.remove(&key)
           btreemap_remove(&mut self.entries, &key);
       }
   }
   ```

3. Add a proof lemma (following the `lemma_new_view` / `lemma_remove_view`
   pattern) that proves `evict`'s postconditions from the helper's ensures
   and `btreemap_remove`'s ensures. You will need:
   - `axiom_cache_lru_of_remove` (already exists) for the LRU update after remove
   - A new helper connecting `cache_lru_of(entries)[0]` to the eviction victim
   - `reveal` of the closed view functions

4. Run `make verify-cache` and report the **exact** result.

**Note:** The `find_lru_victim` spec axiomatizes that the iterator-based minimum
equals the abstract LRU victim (`cache_lru_of(entries)[0]`). This is sound because
`cache_lru_of` is defined as "sorted by last_used ascending", and `min_by_key`
finds the entry with the smallest `last_used`, which is exactly index 0. This is
a narrower trust boundary than the current state where ALL of `evict`'s logic is
trusted.

### Evidence

**Commands run by reviewer:**

1. Baseline verification (all functions with external_body):
```
$ make verify-cache
verification: cached (no recompilation), — (exit 0)
cheating: assume=0 external_body=8 admit=0 trusted=0 no_decreases=0 cfg_gate=0
status: CHEATING_DETECTED
```

2. Removed `external_body` from `Cache::evict`, ran verification:
```
$ make verify-cache
error: The verifier does not yet support the following Rust feature:
       only variables are supported here, not general patterns
   --> src/libs/cache/src/lib.rs:336:26
    |
336 |             .min_by_key(|(_, e)| e.last_used)
    |                          ^^^^^^
```

3. Removed `external_body` from `Cache::put`, ran verification:
```
error: The verifier does not yet support the following Rust feature:
       &mut types, except in special cases
   --> src/libs/cache/src/lib.rs:245:30
```

4. Removed `external_body` from `Cache::get`, ran verification:
```
error: The verifier does not yet support the following Rust feature:
       &mut types, except in special cases
   --> src/libs/cache/src/lib.rs:209:30
```

5. Removed `external_body` from `CacheGuard::deref`, ran verification:
```
error: disallowed: field expression for an opaque datatype
  --> src/libs/cache/src/lib.rs:98:9
```
