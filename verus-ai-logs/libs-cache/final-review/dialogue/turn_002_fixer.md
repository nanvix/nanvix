## Response to Turn 2

### Attempt 1: assume_specification for BTreeMap::remove (simplified K=Q)
- Changes: Added `assume_specification` for `BTreeMap::remove` in lib.spec.rs with simplified signature fixing Q=K.
- Result: FAILED — Verus requires exact generic signature match:
  ```
  error: assume_specification requires function type signature to match exactly
  ```
  The full BTreeMap::remove signature has `A: Allocator + Clone` and `Q: ?Sized + Ord` where `K: Borrow<Q>`. Cannot simplify Q=K without signature mismatch.

### Attempt 2: Stdlib wrapper pattern (btreemap_remove)
- Changes:
  - Created `btreemap_remove<K: Ord, V>` wrapper function in lib.rs (lines 106–127) with `external_body` that wraps `m.remove(k)`. Specs use `btreemap_view_spec` directly:
    ```rust
    #[verus_verify(external_body)]
    #[verus_spec(ret =>
        requires btreemap_view_spec::<K, CacheEntry<V>>(*m).dom().contains(*k) ==> ret is Some,
        ensures
            btreemap_view_spec(*m) == btreemap_view_spec(*old(m)).remove(*k),
            ...
    )]
    fn btreemap_remove<K: Ord, V>(m: &mut BTreeMap<K, CacheEntry<V>>, k: &K) -> Option<CacheEntry<V>>
    ```
  - Changed `Cache::remove` body from `self.entries.remove(key)` to `btreemap_remove(&mut self.entries, key)` (VERUS REWRITE).
  - Removed `#[verus_verify(external_body)]` from `Cache::remove`.
  - Added `proof!` block calling `Self::lemma_remove_view(self, *key, old(self).entries, old(self).capacity)`.
  - Added `axiom_cache_lru_of_remove` proof axiom in lib.proof.rs (external_body proof fn) connecting `cache_lru_of` across BTreeMap remove operations.
  - Added `lemma_remove_view` proof function proving `new_self@ == old_view.spec_remove(key)` and `new_self@.inv()`.
- Result: Initial attempt FAILED — postcondition failures because:
  1. Key-absent case had no proof code (solver couldn't prove filter identity or contents equality)
  2. Key-present case needed explicit field-by-field `=~=` assertions

### Attempt 3: Added lemma_filter_neq_absent + explicit field assertions
- Changes:
  - Added `lemma_filter_neq_absent<K>(s, key)` helper in lib.proof.rs proving `s.filter(|k| k != key) =~= s` when `!s.contains(key)`, by induction on `s.len()`.
  - Restructured `lemma_remove_view` with explicit proofs for both branches:
    - **Key-present**: contents `=~=` + axiom for LRU + filter lemmas for inv + `new_self@ =~= old_view.spec_remove(key)`.
    - **Key-absent**: domain `=~=` to show BTreeMap unchanged → contents unchanged; `!lru.contains(key)` + filter identity → LRU unchanged; `new_self@ =~= old_view =~= old_view.spec_remove(key)`.
- Result: **SUCCESS** — 16 verified, 0 errors

### Final Result
- Status: FIXED
- external_body count change: user functions was 6 → now 5
  - Removed from: `Cache::remove` (lib.rs:282)
  - Remaining user-function external_body: `deref`, `get`, `put`, `clear`, `evict`
  - New infrastructure external_body: `btreemap_remove` wrapper (lib.rs:113), `axiom_cache_lru_of_remove` (lib.proof.rs:373)
- Verification: 16 verified, 0 errors (exit 0)

```
$ make verify-cache
verification results:: 16 verified, 0 errors
cheating: assume=0 external_body=9 admit=0 trusted=0 no_decreases=0 cfg_gate=0
coverage: 8/9 exec functions have contracts
```

### Changes Summary

**lib.rs:**
- Lines 106–127: New `btreemap_remove` stdlib wrapper (external_body, escalation ladder step 4)
- Lines 282–293: `Cache::remove` — removed external_body, VERUS REWRITE to call wrapper, proof! block

**lib.proof.rs:**
- Lines 111–125: New `lemma_filter_neq_absent` — filter identity for absent key
- Lines 373–383: New `axiom_cache_lru_of_remove` — external_body proof fn axiomatizing LRU across remove
- Lines 391–460: New `lemma_remove_view` — proves Cache::remove postconditions

### Trust Entries (to add to trust.md)

1. **btreemap_remove** (lib.rs:113): `STDLIB_WRAPPER` — thin wrapper fixing BTreeMap::remove Q=K, A=Global. Trust: specs match std library contract for Map::remove.
2. **axiom_cache_lru_of_remove** (lib.proof.rs:373): `VERUS_LIMITATION` — axiomatizes that removing a key from BTreeMap preserves relative LRU ordering of remaining entries (filter(!=key)). Sound because BTreeMap::remove doesn't change last_used counters of other entries.
