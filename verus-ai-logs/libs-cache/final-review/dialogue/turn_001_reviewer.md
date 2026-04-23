## Turn 1: external_body on Cache::put — Step 4 (rewrite to avoid get_mut)

### Progress Tracker
- external_body on user functions: **3 remaining** (each is a defect)
  1. `CacheGuard::deref` — lib.rs:93
  2. `Cache::get` — lib.rs:190
  3. `Cache::put` — lib.rs:230
- Infrastructure external_body helpers (from prior drill-downs):
  - `btreemap_remove` — lib.rs:114 (1-line stdlib wrapper, Step 6 outcome)
  - `find_lru_victim` — lib.rs:315 (3-line iterator helper, Step 2/6 outcome)
  - `axiom_cache_lru_of_remove` — lib.proof.rs:401 (proof axiom)
- external_type_specification: 2 expected (`ExCacheEntry`, `ExCacheGuard`) + 2 vstd infra (`ExBTreeMap`, `ExGlobal`)
- Spec quality issues: 0
- Current drill-down: function `Cache::put`, step 4/6

### Verification of Previous Fix
First turn of this dialogue round. Verified current baseline myself:

```
$ make verify-cache
verification: cached (no recompilation), — (exit 0)
cheating: assume=0 external_body=8 admit=0 trusted=0 no_decreases=0 cfg_gate=0
coverage: 9/10 exec functions have contracts
status: CHEATING_DETECTED
```

Previous rounds eliminated `external_body` from `Cache::new`, `Cache::remove`,
`Cache::clear`, and `Cache::evict`. Confirmed: 22 verified, 0 errors (exit 0).
external_body count on user functions: **3** (deref, get, put).

### Triage: 3 Remaining User Function external_body

I removed `external_body` from each function independently and ran `make verify-cache`:

| # | Function | Error when removed | Line | Root Cause | Tractability |
|---|---|---|---|---|---|
| 1 | `Cache::put` | `&mut types, except in special cases` | 245 | `self.entries.get_mut(&key)` | **HIGH** — only get_mut line blocked; rewrite can avoid it |
| 2 | `Cache::get` | `&mut types, except in special cases` | 209 | `self.entries.get_mut(key)` | **LOW** — get_mut + CacheGuard construction (opaque &mut type) |
| 3 | `CacheGuard::deref` | `field expression for an opaque datatype` | 98 | `self.value` on opaque struct | **BLOCKED** — struct inherently opaque due to `&'a mut V` field |

**Drill-down steps already completed for `Cache::put`:**

- **Step 1 (done):** Error confirmed: `&mut types, except in special cases` at line 245.
- **Step 2 (done):** Only `self.entries.get_mut(&key)` is unverifiable. Branches 1
  (zero-capacity return) and 3 (new key + evict/insert) use only `BTreeMap::len`,
  `self.evict()`, and `BTreeMap::insert` — all have specs and verify.
- **Step 3 (done):** Searched vstd for `get_mut` spec — zero occurrences:
  ```
  $ grep -c 'get_mut' ~/.cargo/registry/src/*/vstd-0.0.0-2026-04-12-0118/std_specs/btree.rs
  0
  ```
  vstd comment at line 124 of lib.vstd_btree.rs confirms: `contains_key`, `get`,
  `get_mut`, `remove` cannot be monomorphized via `assume_specification` due to
  `Borrow<Q>` generics.

### Issue

**`Cache::put` (lib.rs:230–265)** has `external_body` on the entire function.

The function body has 3 branches:
```rust
pub fn put(&mut self, key: K, value: V) {
    // Branch 1: zero-capacity — trivial return (VERIFIABLE)
    if self.capacity == 0 { return; }

    // Branch 2: key exists — get_mut + in-place update (BLOCKED by get_mut &mut)
    if let Some(entry) = self.entries.get_mut(&key) {  // ← line 245
        self.counter += 1;
        entry.value = value;
        entry.last_used = self.counter;
        return;
    }

    // Branch 3: new key — capacity check + evict + insert (VERIFIABLE)
    if self.entries.len() >= self.capacity { self.evict(); }
    self.counter += 1;
    self.entries.insert(key, CacheEntry { value, last_used: self.counter });
}
```

**Step 4 — Rewrite.** The `get_mut` call can be avoided entirely by using
`btreemap_remove` (already exists, already has specs) to detect key existence,
then always inserting via `BTreeMap::insert` (has vstd spec). This is the standard
verification technique for avoiding `&mut` references from map lookups:

```rust
pub fn put(&mut self, key: K, value: V) {
    if self.capacity == 0 { return; }

    // VERUS REWRITE: use btreemap_remove to check existence instead of get_mut
    let existed = btreemap_remove(&mut self.entries, &key);

    if existed.is_none() {
        // New key — may need eviction
        if self.entries.len() >= self.capacity {
            self.evict();
        }
    }

    self.counter += 1;
    self.entries.insert(key, CacheEntry { value, last_used: self.counter });
}
```

This is semantically equivalent to the original: both produce the same `Map<K, V>`
contents and the same LRU ordering. The existing VERUS REWRITE precedents in
`Cache::remove` (lib.rs:284) and `Cache::evict` (lib.rs:352-354) confirm this
pattern is accepted in the codebase.

### Specific Question

**Rewrite `Cache::put` to avoid `get_mut`, remove `external_body`, and make it verify.**

Specifically:

1. **Rewrite the body** (lines 238–265) using the remove+insert pattern shown above.
   Mark the rewrite with `// VERUS REWRITE: replace get_mut with remove+insert`.

2. **Remove `#[verus_verify(external_body)]`** from `Cache::put` (line 230).

3. **Add a new axiom `axiom_cache_lru_of_insert`** in lib.proof.rs (analogous to
   `axiom_cache_lru_of_remove`) that captures how `cache_lru_of` changes after
   `BTreeMap::insert`. The axiom should state:

   > Inserting a new `CacheEntry` with `last_used` strictly greater than all
   > existing entries' `last_used` values appends the key to the end of `cache_lru_of`
   > (i.e., makes it the MRU).

   The soundness argument: `cache_lru_of` is "sorted by `last_used` ascending", so
   the entry with the highest `last_used` must appear last. Since `self.counter` is
   monotonically incremented before every insert, the new entry's `last_used` is
   always the largest.

   **Important:** This axiom references `CacheEntry.last_used` (crate-private), which
   is appropriate since it's within the crate's trust boundary. The axiom must have a
   precondition requiring the inserted entry's `last_used` to be greater than all
   existing entries' `last_used`, since we can express this via the entries map.

4. **Add a proof lemma `lemma_put_view`** proving the postconditions for all branches:
   - Zero-capacity: no-op, trivial
   - Key existed (remove returned Some): `remove(key).insert(key, new_value)` matches
     `spec_put`'s overwrite case. LRU: `axiom_cache_lru_of_remove` filters out key,
     then `axiom_cache_lru_of_insert` appends key — net effect is `move_to_mru`.
   - New key at capacity: `evict` + `insert` matches `spec_put`'s eviction case
   - New key below capacity: `insert` matches `spec_put`'s simple insert case

   Use `reveal(<Cache<_, _> as View>::view)`, `reveal(cache_contents_of)`,
   `reveal(cache_lru_of)`, and the existing invariant lemmas.

5. **Run `make verify-cache`** and report the **exact** result (verification count,
   error count, exit code, cheating summary).

### Evidence

**Commands run by reviewer (2026-04-23):**

1. Baseline verification:
```
$ make verify-cache
cheating: assume=0 external_body=8 admit=0 trusted=0 no_decreases=0 cfg_gate=0
coverage: 9/10 exec functions have contracts
status: CHEATING_DETECTED
```

2. Removed `external_body` from `Cache::put`, ran verification:
```
$ make verify-cache
error: The verifier does not yet support the following Rust feature:
       &mut types, except in special cases
   --> src/libs/cache/src/lib.rs:245:30
    |
245 |         if let Some(entry) = self.entries.get_mut(&key) {
    |                              ^^^^^^^^^^^^^^^^^^^^^^^^^^
```

3. Removed `external_body` from `Cache::get`, ran verification:
```
$ make verify-cache
error: The verifier does not yet support the following Rust feature:
       &mut types, except in special cases
   --> src/libs/cache/src/lib.rs:209:30
    |
209 |         if let Some(entry) = self.entries.get_mut(key) {
    |                              ^^^^^^^^^^^^^^^^^^^^^^^^^
```

4. Removed `external_body` from `CacheGuard::deref`, ran verification:
```
$ make verify-cache
error: disallowed: field expression for an opaque datatype
  --> src/libs/cache/src/lib.rs:98:9
   |
98 |         self.value
   |         ^^^^^^^^^^ this field expression is disallowed
```

5. Confirmed no vstd spec for `get_mut`:
```
$ grep -c 'get_mut' ~/.cargo/registry/src/*/vstd-0.0.0-2026-04-12-0118/std_specs/btree.rs
0
```

6. Confirmed `BTreeMap::insert` has vstd spec (lib.vstd_btree.rs:108-122):
```rust
pub assume_specification<Key: Ord, Value, A: Allocator + Clone>[
    alloc::collections::BTreeMap::<Key, Value, A>::insert
](m: &mut BTreeMap<Key, Value, A>, k: Key, v: Value) -> (result: Option<Value>)
    ensures
        btreemap_view_spec(*m) == btreemap_view_spec(*old(m)).insert(k, v),
        ...
```

7. Confirmed `btreemap_remove` wrapper exists (lib.rs:114-123) with full specs.
