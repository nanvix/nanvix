# View Design: `cache` (Bounded LRU Cache)

## Abstract Resource

A **bounded key-value map with LRU eviction**: callers store key-value pairs up
to a fixed capacity, and when the cache is full and a new key is inserted, the
least-recently-accessed entry is automatically evicted.

---

## View Struct

```rust
pub struct CacheView<K, V> {
    /// The key-value mapping of all cached entries.
    pub contents: Map<K, V>,
    /// Maximum number of entries the cache can hold.
    pub capacity: nat,
    /// Keys ordered from least-recently-used (index 0) to most-recently-used (last).
    pub lru_order: Seq<K>,
}
```

### Abstraction pattern

**Ghost Struct View + wf()** — the View is a mathematical model of the cache's
caller-visible state, using `Map`, `Seq`, and `nat` instead of machine types.

---

## Well-formedness Invariant

```rust
impl<K, V> CacheView<K, V> {
    pub open spec fn inv(&self) -> bool {
        // 1. Size never exceeds the configured capacity.
        &&& self.contents.dom().len() <= self.capacity
        // 2. LRU order contains exactly the stored keys, without duplicates.
        &&& self.lru_order.no_duplicates()
        &&& self.lru_order.to_set() == self.contents.dom()
        // 3. Explicit cardinality link (derivable from 2, but helps the solver).
        &&& self.lru_order.len() == self.contents.dom().len()
    }
}
```

### Invariant clauses explained

| # | Clause | Purpose |
|---|--------|---------|
| 1 | `contents.dom().len() <= capacity` | Capacity bound — the core size constraint callers depend on. |
| 2 | `no_duplicates() ∧ to_set() == dom()` | The LRU sequence is a permutation of the key set — every key appears exactly once, and every stored key has a position. |
| 3 | `lru_order.len() == contents.dom().len()` | Explicit length link. Derivable from (2), but stated for SMT convenience: needed when proving `lru_order[0]` is safe in the eviction branch. |

---

## Helper Spec Function

```rust
impl<K, V> CacheView<K, V> {
    /// Move `key` to the most-recently-used position (end of the sequence).
    /// Preserves all other elements and their relative order.
    pub open spec fn move_to_mru(self, key: K) -> Seq<K> {
        self.lru_order.filter(|k: K| k != key).push(key)
    }
}
```

This helper isolates the `filter(…).push(…)` pattern so that properties
(set-preservation, no-duplicates preservation, last-element identity) can be
proved once as lemmas and reused across `spec_get` and `spec_put`.

---

## Spec Transition Functions

All transitions have the implicit precondition `recommends self.inv()`.

### `spec_new`

```rust
pub open spec fn spec_new(capacity: nat) -> CacheView<K, V> {
    CacheView {
        contents: Map::empty(),
        capacity,
        lru_order: Seq::empty(),
    }
}
```

### `spec_get`

Returns the updated cache state and the lookup result.

```rust
pub open spec fn spec_get(self, key: K) -> (CacheView<K, V>, Option<V>) {
    if self.contents.dom().contains(key) {
        (CacheView {
            lru_order: self.move_to_mru(key),
            ..self
        }, Some(self.contents[key]))
    } else {
        (self, None)
    }
}
```

- **Hit:** recency is refreshed (key moves to MRU); contents and capacity
  unchanged.  Returns `Some(value)`.
- **Miss:** state unchanged.  Returns `None`.

### `spec_put`

```rust
pub open spec fn spec_put(self, key: K, value: V) -> CacheView<K, V> {
    if self.capacity == 0 {
        // Zero-capacity cache: no-op.
        self
    } else if self.contents.dom().contains(key) {
        // Overwrite existing key: replace value, refresh recency.
        CacheView {
            contents: self.contents.insert(key, value),
            lru_order: self.move_to_mru(key),
            ..self
        }
    } else if self.contents.dom().len() >= self.capacity {
        // At capacity with new key: evict LRU victim, then insert.
        let victim = self.lru_order[0];
        CacheView {
            contents: self.contents.remove(victim).insert(key, value),
            lru_order: self.lru_order.subrange(1, self.lru_order.len() as int).push(key),
            ..self
        }
    } else {
        // Below capacity with new key: insert directly.
        CacheView {
            contents: self.contents.insert(key, value),
            lru_order: self.lru_order.push(key),
            ..self
        }
    }
}
```

- **Eviction safety:** the branch `contents.dom().len() >= capacity` with
  `capacity > 0` plus `inv()` guarantees `lru_order.len() >= 1`, so
  `lru_order[0]` is well-defined.

### `spec_remove`

```rust
pub open spec fn spec_remove(self, key: K) -> CacheView<K, V> {
    if self.contents.dom().contains(key) {
        CacheView {
            contents: self.contents.remove(key),
            lru_order: self.lru_order.filter(|k: K| k != key),
            ..self
        }
    } else {
        // Key absent: no-op.
        self
    }
}
```

### `spec_clear`

```rust
pub open spec fn spec_clear(self) -> CacheView<K, V> {
    CacheView {
        contents: Map::empty(),
        lru_order: Seq::empty(),
        ..self  // capacity preserved
    }
}
```

---

## CacheGuard Modeling

`CacheGuard<'_, V>` implements `Deref<Target=V>` and `DerefMut`.  It is a
transparent borrow wrapper over `&mut V` — the value stored inside the cache.

**Design decision: no separate GuardView type.**

Rationale:

1. **The guard is a thin smart pointer.**  Its only abstract state is the value
   it dereferences to.  A `GuardView<V>` would contain a single field `value: V`
   — isomorphic to `V` itself, adding no information.

2. **Verus handles `&mut` natively.**  When `get(&mut self)` returns a
   `CacheGuard<'_, V>`, Verus tracks the mutable borrow.  While the guard is
   live the cache is exclusively borrowed; when the guard is dropped, any
   mutations to `*guard` are reflected in the cache's post-state.  This is
   standard `&mut` semantics, not a cache-specific concern.

3. **Spec-phase guidance:**
   - The `get` ensures clause will state `*result == old(self)@.contents[key]`
     (value identity on return).
   - The recency update (`move_to_mru`) is captured in `self@` vs `old(self)@`.
   - Value mutations through the guard modify only `contents[key]`; `lru_order`
     and `capacity` are unaffected.

---

## Design Rationale

### Field: `contents: Map<K, V>`

- **What it represents:** the set of key-value pairs currently stored.
- **Substitution test:** ✅ — any cache implementation, regardless of backing
  data structure (hash map, B-tree, array, linked list), maintains a logical
  key-value mapping.
- **Used in:** every spec transition (get, put, remove, clear) and `inv()`.

### Field: `capacity: nat`

- **What it represents:** the upper bound on the number of entries.
- **Substitution test:** ✅ — any bounded cache has a configurable capacity.
  Using `nat` (not `usize`) avoids unnecessary overflow reasoning in specs.
- **Used in:** `inv()` (capacity bound) and `spec_put` (eviction decision).

### Field: `lru_order: Seq<K>`

- **What it represents:** the recency ordering of all cached keys, from
  least-recently-used (index 0) to most-recently-used (last element).
- **Substitution test:** ✅ — any LRU cache must maintain a total recency
  order over its entries, regardless of whether the implementation uses
  counters, linked lists, timestamps, or something else.  The ordering is
  directly observable through eviction behavior: callers can determine which
  entry will be evicted next.
- **Used in:** `spec_get` (recency refresh), `spec_put` (eviction victim
  selection, recency refresh on overwrite), `spec_remove` (key removal from
  order), `spec_clear` (reset), and `inv()`.

---

## Rejected Alternatives

### 1. `recency: Map<K, nat>` instead of `lru_order: Seq<K>`

Map each key to a recency rank (higher = more recent).

**Rejected because:**
- Introduces arbitrary counter values that mirror the implementation's internal
  counter, violating the substitution test.
- Requires additional `inv()` clauses: injectivity (no two keys share a rank),
  density or boundedness of ranks, and well-ordering.
- Eviction becomes `argmin(recency)` rather than the simpler `lru_order[0]`.
- Strictly more complex with no gain in expressiveness.

### 2. `size: nat` as a separate field

Track the current number of entries explicitly.

**Rejected because:**
- Derivable from `contents.dom().len()`.  Adding it violates minimality and
  creates a consistency obligation (`size == contents.dom().len()`) in every
  transition.

### 3. `last_accessed: Option<K>` — most-recently-used key

**Rejected because:**
- Subsume by `lru_order.last()`.  Adding it violates minimality.

### 4. No `lru_order` field (just `contents` + `capacity`)

**Rejected because:**
- Under-specified: without the ordering, we cannot specify *which* entry is
  evicted.  The spec could only say "some entry is removed", which is too weak
  for callers who test specific eviction behavior (e.g.,
  `get_refreshes_lru_order`, `evicts_lru_entry_when_full`).

### 5. `eviction_candidate: Option<K>` — only the LRU victim

**Rejected because:**
- After any recency-refreshing operation (`get` or `put` overwrite), the new
  LRU victim changes.  Tracking only the current victim doesn't let us reason
  about how sequences of operations affect eviction order.  The full ordering
  is needed for compositional reasoning.

### 6. Separate `GuardView<K, V>` type for `CacheGuard`

**Rejected because:**
- The guard is a transparent smart-pointer wrapper over `&mut V`.  Its abstract
  state is just the value itself — a `GuardView` would be isomorphic to `V` and
  add no information.  Verus handles `&mut` borrows natively; the cache's
  post-state after the guard is dropped reflects any mutations.  See the
  "CacheGuard Modeling" section above.

---

## Quality Checklist

| Criterion | Status | Evidence |
|-----------|--------|----------|
| **Substitution** | ✅ | Each field survives a rewrite to a different algorithm (hash map, linked list, array, etc.). |
| **Caller-only** | ✅ | No mention of counters, B-trees, internal entry structs, or memory layout. |
| **Complete** | ✅ | All 8 caller expectations from the analysis are expressible: capacity bound, key uniqueness (Map semantics), lookup consistency, LRU eviction, overwrite semantics, remove idempotency, clear, and guard mutation. |
| **Minimal** | ✅ | Every field appears in at least one spec transition and in `inv()`. |
| **No code-as-spec** | ✅ | Spec transitions describe WHAT changes, not HOW the implementation achieves it. |
