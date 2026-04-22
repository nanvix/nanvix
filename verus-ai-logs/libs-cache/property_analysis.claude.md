# Property Analysis: `cache` (Bounded LRU Cache)

> Module: `src/libs/cache/src/lib.rs`
> View: `CacheView<K, V>` with fields `contents: Map<K, V>`, `capacity: nat`, `lru_order: Seq<K>`

---

## TYPE: Type/Representation Invariants

### TYPE-1: View Invariant (well-formedness)

**Statement:** The abstract state satisfies `CacheView::inv()` at every observable program point — after construction and after every public method returns.

**Applies to:** all public methods (`new`, `get`, `put`, `remove`, `clear`) and private helper (`evict`)

**Formal sketch:**
```
inv(&self) -> bool {
    &&& self.contents.dom().len() <= self.capacity
    &&& self.lru_order.no_duplicates()
    &&& self.lru_order.to_set() == self.contents.dom()
    &&& self.lru_order.len() == self.contents.dom().len()
}
```

**Why needed:** Every spec transition function assumes `inv()` holds on entry. If any operation breaks it, all subsequent reasoning is unsound. This is the foundational invariant.

---

### TYPE-2: Abstraction Function Consistency

**Statement:** The view function `self@` (mapping `Cache<K, V>` to `CacheView<K, V>`) correctly abstracts the concrete state:
- `self@.contents` maps each key `k` in `self.entries` to its `.value` field (stripping the `last_used` counter).
- `self@.capacity == self.capacity as nat`.
- `self@.lru_order` is a sequence of all keys in `self.entries`, sorted by ascending `last_used` values (smallest = LRU at index 0, largest = MRU at end).

**Applies to:** `View` impl for `Cache<K, V>`

**Formal sketch:**
```
ensures
    self@.contents.dom() == self.entries.keys_as_set(),
    forall |k| self.entries.contains_key(k) ==> self@.contents[k] == self.entries[k].value,
    self@.capacity == self.capacity as nat,
    self@.lru_order is entries sorted by ascending last_used,
```

**Why needed:** The abstraction function is the bridge between implementation and specification. Without it, ensures clauses expressed in terms of `CacheView` have no connection to actual runtime behavior.

---

### TYPE-3: Counter Well-formedness

**Statement:** All `last_used` values in `self.entries` are in the range `[1, self.counter]`, and `self.counter >= self.entries.len()` (since each entry was assigned a distinct counter value on insertion/access).

**Applies to:** internal invariant, maintained by `new`, `get`, `put`, `clear`

**Formal sketch:**
```
forall |k| self.entries.contains_key(k)
    ==> 1 <= self.entries[k].last_used <= self.counter,
self.counter >= self.entries.len() as u64,
```

**Why needed:** The correctness of `evict` (finding the minimum `last_used`) and the LRU ordering depend on counter values being sensible. This invariant also supports proving that the `lru_order` sequence is well-defined.

---

### TYPE-4: Counter Injectivity

**Statement:** No two distinct entries share the same `last_used` value:
```
forall |k1, k2| k1 != k2 && entries.contains_key(k1) && entries.contains_key(k2)
    ==> entries[k1].last_used != entries[k2].last_used
```

**Applies to:** internal invariant, maintained by `new`, `get`, `put`, `clear`

**Why needed:** Injectivity ensures that `min_by_key(last_used)` in `evict` produces a unique victim — the LRU entry is deterministic. Without this, the spec-level `lru_order[0]` might not correspond to the implementation's eviction choice.

---

### TYPE-5: LRU Order Matches Counter Order

**Statement:** The `lru_order` sequence in the view is consistent with the `last_used` counter ordering: for any two entries at positions `i < j` in `lru_order`, the entry at position `i` has a strictly smaller `last_used` value than the entry at position `j`.

**Applies to:** internal invariant linking concrete counters to abstract ordering

**Formal sketch:**
```
forall |i: int, j: int|
    0 <= i < j < self@.lru_order.len()
    ==> entries[lru_order[i]].last_used < entries[lru_order[j]].last_used
```

**Why needed:** This is the key linking invariant that connects the counter-based implementation to the sequence-based abstract specification. It ensures that the min-by-key eviction in the implementation corresponds to removing `lru_order[0]` in the spec.

---

## FN: Function-Level Contracts

### FN-1: `Cache::new(capacity)` — Construction

**Preconditions:** None (any `usize` is valid).

**Postconditions:**
```
ensures
    result@ == CacheView::spec_new(capacity as nat),
    result@.inv(),
    result@.contents == Map::empty(),
    result@.capacity == capacity as nat,
    result@.lru_order == Seq::empty(),
```

**Frame:** N/A (fresh object).

---

### FN-2: `Cache::get(&mut self, key)` — Cache Hit

**Preconditions:**
```
requires old(self)@.inv()
```

**Postconditions (hit — key present):**
```
ensures
    old(self)@.contents.dom().contains(*key) ==> (
        result is Some,
        *result.unwrap() == old(self)@.contents[*key],
        self@ == old(self)@.spec_get(*key).0,
        self@.contents == old(self)@.contents,            // contents unchanged
        self@.capacity == old(self)@.capacity,             // capacity unchanged
        self@.lru_order == old(self)@.move_to_mru(*key),   // key moved to MRU
        self@.inv(),
    )
```

**Postconditions (miss — key absent):**
```
ensures
    !old(self)@.contents.dom().contains(*key) ==> (
        result is None,
        self@ == old(self)@,   // state completely unchanged
        self@.inv(),
    )
```

**Frame:** On miss, the entire abstract state is unchanged. On hit, only `lru_order` changes; `contents` and `capacity` are preserved.

---

### FN-3: `Cache::put(&mut self, key, value)` — Zero Capacity

**Preconditions:**
```
requires old(self)@.inv()
```

**Postconditions (capacity == 0):**
```
ensures
    old(self)@.capacity == 0 ==> self@ == old(self)@,
    self@.inv(),
```

**Why separate:** Zero-capacity is a degenerate edge case that shortcuts to a no-op. It must not modify state.

---

### FN-4: `Cache::put(&mut self, key, value)` — Overwrite Existing Key

**Preconditions:**
```
requires
    old(self)@.inv(),
    old(self)@.capacity > 0,
    old(self)@.contents.dom().contains(key),
```

**Postconditions:**
```
ensures
    self@.contents == old(self)@.contents.insert(key, value),
    self@.contents.dom().len() == old(self)@.contents.dom().len(),   // size unchanged
    self@.lru_order == old(self)@.move_to_mru(key),                  // key refreshed to MRU
    self@.capacity == old(self)@.capacity,
    self@.inv(),
```

**Frame:** `capacity` and set of keys are unchanged. Only the value and LRU position of `key` change.

---

### FN-5: `Cache::put(&mut self, key, value)` — Insert New Key, Below Capacity

**Preconditions:**
```
requires
    old(self)@.inv(),
    old(self)@.capacity > 0,
    !old(self)@.contents.dom().contains(key),
    old(self)@.contents.dom().len() < old(self)@.capacity,
```

**Postconditions:**
```
ensures
    self@.contents == old(self)@.contents.insert(key, value),
    self@.contents.dom().len() == old(self)@.contents.dom().len() + 1,
    self@.lru_order == old(self)@.lru_order.push(key),   // new key is MRU
    self@.capacity == old(self)@.capacity,
    self@.inv(),
```

---

### FN-6: `Cache::put(&mut self, key, value)` — Insert New Key, At Capacity (Eviction)

**Preconditions:**
```
requires
    old(self)@.inv(),
    old(self)@.capacity > 0,
    !old(self)@.contents.dom().contains(key),
    old(self)@.contents.dom().len() >= old(self)@.capacity,
```

**Postconditions:**
```
ensures
    // The LRU victim is evicted
    let victim = old(self)@.lru_order[0];
    !self@.contents.dom().contains(victim),
    // New key is inserted
    self@.contents.dom().contains(key),
    self@.contents[key] == value,
    // Size preserved (evict one, insert one)
    self@.contents.dom().len() == old(self)@.contents.dom().len(),
    // All non-victim entries preserved
    forall |k| k != victim && old(self)@.contents.dom().contains(k)
        ==> self@.contents.dom().contains(k)
            && self@.contents[k] == old(self)@.contents[k],
    // LRU order: victim removed, new key at MRU
    self@.lru_order == old(self)@.lru_order.subrange(1, old(self)@.lru_order.len() as int).push(key),
    self@.capacity == old(self)@.capacity,
    self@.inv(),
```

**Why this matters:** This is the most complex transition — it must evict exactly the right entry, preserve all others, insert the new one, and maintain `inv()`.

---

### FN-7: `Cache::put` — Unified Contract

**Statement:** The unified postcondition for `put`, covering all branches:
```
ensures
    self@ == old(self)@.spec_put(key, value),
    self@.inv(),
```

**Why needed:** While FN-3 through FN-6 detail each branch, the implementation must satisfy a single unified spec. This is the contract that callers depend on.

---

### FN-8: `Cache::remove(&mut self, key)` — Key Present

**Preconditions:**
```
requires old(self)@.inv()
```

**Postconditions (key present):**
```
ensures
    old(self)@.contents.dom().contains(*key) ==> (
        !self@.contents.dom().contains(*key),
        self@.contents == old(self)@.contents.remove(*key),
        self@.contents.dom().len() == old(self)@.contents.dom().len() - 1,
        self@.lru_order == old(self)@.lru_order.filter(|k: K| k != *key),
        self@.capacity == old(self)@.capacity,
        self@.inv(),
    )
```

---

### FN-9: `Cache::remove(&mut self, key)` — Key Absent (No-op)

**Postconditions:**
```
ensures
    !old(self)@.contents.dom().contains(*key) ==> (
        self@ == old(self)@,
        self@.inv(),
    )
```

---

### FN-10: `Cache::remove` — Unified Contract

```
ensures
    self@ == old(self)@.spec_remove(*key),
    self@.inv(),
```

---

### FN-11: `Cache::clear(&mut self)`

**Preconditions:**
```
requires old(self)@.inv()
```

**Postconditions:**
```
ensures
    self@.contents == Map::empty(),
    self@.lru_order == Seq::empty(),
    self@.capacity == old(self)@.capacity,   // capacity preserved
    self@ == old(self)@.spec_clear(),
    self@.inv(),
```

**Frame:** Only `capacity` survives; everything else is reset.

---

### FN-12: `Cache::evict(&mut self)` — Private Helper

**Preconditions:**
```
requires
    old(self)@.inv(),
    old(self)@.contents.dom().len() > 0,   // non-empty (called only when at capacity > 0)
```

**Postconditions:**
```
ensures
    let victim = old(self)@.lru_order[0];
    !self@.contents.dom().contains(victim),
    self@.contents == old(self)@.contents.remove(victim),
    self@.contents.dom().len() == old(self)@.contents.dom().len() - 1,
    self@.lru_order == old(self)@.lru_order.subrange(1, old(self)@.lru_order.len() as int),
    self@.capacity == old(self)@.capacity,
    // counter is unchanged by evict
    self@.inv(),
```

**Why needed:** `evict` is a critical internal function. Its correctness directly determines whether `put`'s eviction contract (FN-6) holds. The implementation uses `min_by_key` on `last_used`, which must correspond to `lru_order[0]`.

---

### FN-13: `CacheGuard::deref(&self) -> &V`

**Postconditions:**
```
ensures result == *self.value
```

**Statement:** `Deref` returns a reference to the value that was stored in the cache entry at the time the guard was created. This is a thin wrapper — no transformation.

---

### FN-14: `CacheGuard::deref_mut(&mut self) -> &mut V`

**Postconditions:**
```
ensures result == self.value
```

**Statement:** `DerefMut` returns a mutable reference to the cached value. Mutations through this reference persist in the cache after the guard is dropped (standard `&mut` semantics enforced by Rust's borrow checker; no additional proof obligation).

---

## MOD: Module-Level Safety Properties

### MOD-1: Capacity Bound Maintenance

**Statement:** After every public operation, `self@.contents.dom().len() <= self@.capacity`.

**Applies to:** `new`, `get`, `put`, `remove`, `clear`

**Why needed:** This is the core bounded-cache invariant. `put` is the only operation that adds entries, and it must evict first if at capacity. A violation would mean unbounded memory growth.

---

### MOD-2: Counter Monotonicity

**Statement:** `self.counter` never decreases across any operation. Specifically:
- `get` (hit): `self.counter == old(self).counter + 1`
- `get` (miss): `self.counter == old(self).counter`
- `put` (capacity 0): `self.counter == old(self).counter`
- `put` (otherwise): `self.counter == old(self).counter + 1`
- `remove`: `self.counter == old(self).counter`
- `clear`: `self.counter == 0` (reset, but this is acceptable since entries are also cleared)
- `evict`: `self.counter == old(self).counter`

**Why needed:** Monotonicity of the counter is essential for the LRU ordering to be well-defined. If the counter could decrease or wrap, two entries could share a `last_used` value, breaking TYPE-4 (injectivity) and making eviction non-deterministic.

---

### MOD-3: Key-Value Consistency

**Statement:** For any key `k`:
- If `k ∈ self@.contents.dom()`, then `k` appears exactly once in `self@.lru_order`.
- If `k ∉ self@.contents.dom()`, then `k` does not appear in `self@.lru_order`.

**Applies to:** All operations (part of `inv()` via `lru_order.to_set() == contents.dom()`)

**Why needed:** Ensures the LRU ordering and contents map are always consistent. A key in the map but not the ordering would never be evicted; a key in the ordering but not the map would cause spurious eviction.

---

### MOD-4: Capacity Immutability

**Statement:** `self@.capacity` never changes after construction. For every public method:
```
ensures self@.capacity == old(self)@.capacity
```

**Why needed:** The capacity is set once at construction and should never be modified. If it could change, all capacity-bound reasoning would need to account for arbitrary capacity changes.

---

### MOD-5: LRU Ordering Preservation

**Statement:** Operations that do not access a key preserve the relative ordering of all other keys. Specifically:
- `get(k)` on hit: the relative order of all keys other than `k` is preserved; `k` moves to end.
- `put(k, v)` overwrite: the relative order of all keys other than `k` is preserved; `k` moves to end.
- `put(k, v)` new with eviction: the relative order of all surviving (non-victim) keys is preserved; `k` is appended.
- `remove(k)`: the relative order of all remaining keys is preserved.

**Formal sketch (for get/put overwrite via `move_to_mru`):**
```
forall |i: int, j: int|
    0 <= i < j < old(self)@.lru_order.len()
    && old(self)@.lru_order[i] != key
    && old(self)@.lru_order[j] != key
    ==> // positions of lru_order[i] and lru_order[j] in the new ordering
        // maintain i' < j' (relative order preserved)
```

**Why needed:** Correct LRU semantics require that accessing one key doesn't disturb the relative recency of other keys. Without this, eviction could choose the wrong victim.

---

### MOD-6: Contents Preservation on Non-Mutating Paths

**Statement:** Operations that don't logically modify the contents must leave `self@.contents` unchanged:
- `get` (hit or miss): `self@.contents == old(self)@.contents`
- `remove` (key absent): `self@.contents == old(self)@.contents`
- `put` (capacity 0): `self@.contents == old(self)@.contents`

**Why needed:** Frame conditions prevent accidental corruption. A `get` that silently modified a value would violate caller expectations.

---

## LIVE: Liveness Properties

### LIVE-1: Get Guaranteed Success on Present Key

**Statement:** If `self@.contents.dom().contains(key)` before calling `get`, then `get` returns `Some`.

**Formal:**
```
requires old(self)@.inv(), old(self)@.contents.dom().contains(*key)
ensures result.is_some()
```

**Why needed:** Callers depend on `get` succeeding when the key is known to be present (e.g., after a `put` without intervening eviction or removal).

---

### LIVE-2: Put Guaranteed Insertion (Non-Zero Capacity)

**Statement:** If `capacity > 0`, then after `put(key, value)`, the key is present in the cache:
```
requires old(self)@.inv(), old(self)@.capacity > 0
ensures self@.contents.dom().contains(key), self@.contents[key] == value
```

**Why needed:** A `put` that silently fails to insert (other than in the zero-capacity case) would violate the fundamental contract. Callers rely on `put` followed by `get` succeeding.

---

### LIVE-3: Put-Get Round-Trip

**Statement:** After `put(k, v)` on a cache with `capacity > 0`, an immediate `get(k)` (with no intervening operations) returns `Some` dereferencing to `v`.

**Formal (compositional, from FN-7 and FN-2):**
```
let c1 = old(self)@.spec_put(k, v);   // c1.inv() holds (by FN-7)
c1.contents.dom().contains(k);         // by spec_put definition
c1.contents[k] == v;                   // by spec_put definition
let (c2, result) = c1.spec_get(k);     // result == Some(v) (by FN-2)
```

**Why needed:** This is the most fundamental usability property. It composes FN-7 and FN-2 but is worth stating explicitly because it's the primary caller expectation.

---

### LIVE-4: Eviction Termination

**Statement:** `evict()` always terminates. The iterator over `self.entries` is finite (bounded by `self.entries.len()`), and `min_by_key` consumes the iterator in a single pass.

**Why needed:** Although Rust iterators on `BTreeMap` are finite by construction, this property is relevant because `evict` is called within `put`, and `put` must terminate. In Verus, loop-based operations (iterator internals) may require decreases clauses if expanded.

---

### LIVE-5: Space Available After Eviction

**Statement:** After `evict()` returns (given it was called on a non-empty cache), `self.entries.len() < old_capacity`, ensuring there is room for the subsequent `insert` in `put`.

**Formal:**
```
requires old(self)@.inv(), old(self)@.contents.dom().len() > 0
ensures self@.contents.dom().len() == old(self)@.contents.dom().len() - 1
```

**Why needed:** `put` calls `evict` to make room, then inserts. If `evict` didn't actually remove an entry, the capacity bound (MOD-1) would be violated after insertion.

---

### LIVE-6: Clear Enables Reuse

**Statement:** After `clear()`, the cache is in the same abstract state as `Cache::new(capacity)` (modulo the same capacity), and all subsequent operations behave as on a fresh cache.

**Formal:**
```
ensures self@ == CacheView::spec_new(old(self)@.capacity)
```

**Why needed:** Callers expect `clear` to fully reset the cache. If internal state (like the counter) were not properly handled, subsequent operations could malfunction.

---

## GLOBAL: Cross-Module Properties

### GLOBAL-1: Cache as Bounded Resource Manager

**Statement:** The `cache` module provides a bounded key-value store. Any system component that uses the cache can rely on:
1. Memory usage is bounded by `capacity` entries (no unbounded growth).
2. Lookups are consistent with prior insertions (modulo eviction and removal).
3. The eviction policy is deterministic (LRU).

**Applies to:** Any future caller in the Nanvix workspace that depends on `cache`.

**Why needed:** Currently the crate has zero external callers, but the API is public and designed for reuse. Any future integration with `nanvix-sandbox-cache` or other components must be able to rely on these guarantees.

---

### GLOBAL-2: No Panic Guarantee

**Statement:** None of the public methods (`new`, `get`, `put`, `remove`, `clear`) panic under any input, assuming `inv()` holds. The private `evict` method does not panic when called on a non-empty cache.

**Applies to:** All public methods

**Why needed:** In a systems context (Nanvix is a microkernel), panics in library code can crash the entire system. The cache must be panic-free for all valid inputs.

---

### GLOBAL-3: Guard Borrow Safety

**Statement:** A `CacheGuard` borrows the cache exclusively (`&mut self`). While the guard is live, no other cache operations can be called. Mutations through the guard modify only `self@.contents[key]`; `lru_order` and `capacity` are unaffected.

**Applies to:** `get`, `Deref`, `DerefMut`

**Why needed:** Rust's borrow checker enforces exclusivity statically. From the verification perspective, the key property is that mutations through the guard update only the value, not the cache's structural metadata. This is ensured by Rust's type system and does not require additional proof, but should be documented as a relied-upon guarantee.

---

## Suspected Bugs and Edge Cases

### BUG-1: Counter Overflow (u64 wrapping)

**Severity:** Potential correctness bug (extremely unlikely in practice)

**Description:** `self.counter` is a `u64` that is incremented on every `get` hit and every `put`. After 2^64 operations, the counter wraps to 0 (in release mode) or panics (in debug mode). After wrapping:
- New entries could receive `last_used` values smaller than existing entries.
- `min_by_key` would evict the *most recently* used entry instead of the least.
- TYPE-4 (counter injectivity) would be violated — multiple entries could share `last_used == 0`.

**Impact:** Violates LRU correctness. The victim selected by `evict` would be wrong.

**Mitigation options:**
1. Add a precondition `requires self.counter < u64::MAX` to `get` and `put`.
2. Add overflow checking (return an error or saturate).
3. For verification purposes, prove that the counter cannot overflow given the capacity bound (if `capacity < u64::MAX` and operations are bounded, overflow is impossible in practice, but may need an assumption for the proof).

**Recommendation for verification:** Add `requires self.counter < u64::MAX` as a precondition on `get` and `put`. This is a reasonable assumption since 2^64 operations is physically unreachable, and it makes TYPE-3 and TYPE-4 provable.

---

### BUG-2: `entries.len() >= capacity` with `usize` vs `nat`

**Severity:** Correctness concern in specification

**Description:** The implementation uses `self.entries.len() >= self.capacity` where both are `usize`. The spec uses `self.contents.dom().len() >= self.capacity` where capacity is `nat`. The comparison is correct, but the abstraction function must ensure that `self.entries.len() as nat == self@.contents.dom().len()` — i.e., the BTreeMap length matches the abstract map domain length. This is a trust boundary (see below).

---

### BUG-3: `evict` Called on Empty Cache

**Severity:** Non-issue (guarded by control flow)

**Description:** `evict` is only called when `self.entries.len() >= self.capacity` and `self.capacity > 0`, which implies `self.entries.len() >= 1`. So the iterator is non-empty and `min_by_key` returns `Some`. However, the code uses `if let Some(key) = victim` defensively. The precondition on `evict` (FN-12) should require non-emptiness for clean verification.

---

### BUG-4: `clear` Resets Counter but Not to Initial State Semantics

**Severity:** Design observation, not a bug

**Description:** `clear()` resets `self.counter = 0`, which is correct — it matches the spec `spec_clear` which produces `lru_order: Seq::empty()`. Since all entries are removed, the counter is meaningless and resetting it is safe. However, this means `clear()` produces a state that is `inv()`-equivalent to `new(capacity)` but at the implementation level the `BTreeMap` may retain allocated memory. This is fine — the abstract state is what matters.

---

## Exclusions

### Not Verified: Performance Properties

**Reason:** Time complexity (O(n) eviction, O(log n) lookup) is not a correctness property. Verus verifies functional correctness, not performance.

### Not Verified: Memory Allocation

**Reason:** `BTreeMap` memory management is handled by the allocator. The cache's correctness does not depend on specific allocation behavior.

### Not Verified: Thread Safety

**Reason:** `Cache<K, V>` is not `Sync` or `Send` by design. Concurrent access is the caller's responsibility (e.g., wrapping in a `Mutex`). Single-threaded correctness is what we verify.

### Not Verified: `Clone` Correctness for `CacheEntry`

**Reason:** Derived `Clone` — the compiler generates it. Verifying derived trait impls is out of scope.

---

## Trust Boundaries

### BTreeMap Operations

The implementation relies on `alloc::collections::BTreeMap` for:
- `get_mut(&K) -> Option<&mut V>` — returns `Some` iff key is present
- `insert(K, V)` — inserts or overwrites
- `remove(&K)` — removes if present
- `len() -> usize` — returns the number of entries
- `iter() -> impl Iterator` — iterates over all entries
- `clear()` — removes all entries
- `new() -> BTreeMap` — creates empty map

**vstd coverage:** vstd does not provide specifications for `alloc::collections::BTreeMap`. These operations need `assume_specification` declarations that state their functional behavior in terms of abstract `Map<K, V>` semantics. This is the primary trust boundary for this module.

Specifically, the following BTreeMap behaviors must be assumed:
1. `get_mut` returns `Some(&mut v)` iff the key is in the map, and the returned reference points to the value associated with that key.
2. `insert(k, v)` adds the mapping `k -> v`, replacing any previous value for `k`.
3. `remove(k)` removes key `k` if present; no-op if absent.
4. `len()` returns the number of key-value pairs.
5. `clear()` produces an empty map.
6. `iter()` yields all key-value pairs exactly once.
7. `min_by_key` on the iterator returns the entry with the minimum key value (by the provided closure).

### Iterator / `min_by_key`

`core::iter::Iterator::min_by_key` is used in `evict`. vstd does not spec iterators from `alloc`. An `assume_specification` is needed to state that `min_by_key` returns the element minimizing the key function, and that it returns `Some` when the iterator is non-empty.

---

## Property Summary Table

| ID | Category | Property | Functions |
|----|----------|----------|-----------|
| TYPE-1 | Invariant | View well-formedness (`inv()`) | all |
| TYPE-2 | Invariant | Abstraction function consistency | View impl |
| TYPE-3 | Invariant | Counter range `[1, counter]` | new, get, put, clear |
| TYPE-4 | Invariant | Counter injectivity | new, get, put, clear |
| TYPE-5 | Invariant | LRU order ↔ counter order | all |
| FN-1 | Contract | `new` postcondition | new |
| FN-2 | Contract | `get` hit/miss postcondition | get |
| FN-3 | Contract | `put` zero-capacity no-op | put |
| FN-4 | Contract | `put` overwrite (existing key) | put |
| FN-5 | Contract | `put` insert below capacity | put |
| FN-6 | Contract | `put` insert with eviction | put |
| FN-7 | Contract | `put` unified spec | put |
| FN-8 | Contract | `remove` key present | remove |
| FN-9 | Contract | `remove` key absent (no-op) | remove |
| FN-10 | Contract | `remove` unified spec | remove |
| FN-11 | Contract | `clear` postcondition | clear |
| FN-12 | Contract | `evict` postcondition | evict |
| FN-13 | Contract | `deref` returns value | deref |
| FN-14 | Contract | `deref_mut` returns value | deref_mut |
| MOD-1 | Safety | Capacity bound maintenance | all |
| MOD-2 | Safety | Counter monotonicity | all |
| MOD-3 | Safety | Key-value / LRU consistency | all |
| MOD-4 | Safety | Capacity immutability | all |
| MOD-5 | Safety | LRU relative-order preservation | get, put, remove |
| MOD-6 | Safety | Contents frame on non-mutating paths | get, remove, put(cap=0) |
| LIVE-1 | Liveness | Get succeeds on present key | get |
| LIVE-2 | Liveness | Put inserts (non-zero capacity) | put |
| LIVE-3 | Liveness | Put-get round-trip | put, get |
| LIVE-4 | Liveness | Eviction termination | evict |
| LIVE-5 | Liveness | Space available after eviction | evict |
| LIVE-6 | Liveness | Clear enables reuse | clear |
| GLOBAL-1 | Cross-module | Bounded resource guarantee | all |
| GLOBAL-2 | Cross-module | No-panic guarantee | all |
| GLOBAL-3 | Cross-module | Guard borrow safety | get, deref, deref_mut |
