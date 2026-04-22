# Caller Analysis: `cache` (Bounded LRU Cache)

> 0 external callers — analysis based on API design intent and tests.

## Script Output

**Parser:** tree-sitter only (single-module, no reverse deps)

No crate in the workspace depends on `cache`. The `nanvix-sandbox-cache`
crate is an unrelated module (sandbox pool management) and does not depend on
`cache`. The `cache` crate appears in the workspace `[dependencies]` table
but is never consumed by any other crate.

All call sites are within the module's own `#[cfg(test)]` block (10 test
functions exercising the full public API).

## Trait Obligations

### `Deref for CacheGuard<'_, V>`
- **Target:** `V`
- **Expected semantics:** `*guard` yields `&V` — an immutable reference to
  the cached value. The compiler inserts `deref()` for `*` and auto-deref
  coercions.

### `DerefMut for CacheGuard<'_, V>`
- **Expected semantics:** `*guard` yields `&mut V` — a mutable reference to
  the cached value, allowing in-place updates. The compiler inserts
  `deref_mut()` for `*` on `&mut` contexts.

### `Clone for CacheEntry<V>` (derived, private)
- Internal only. Required by `BTreeMap` iteration during eviction, which
  clones the victim key (`K: Clone`).

## Caller Expectations

### `Cache::new(capacity: usize) -> Self`
- **Callers assume:** Returns an empty cache that can hold up to `capacity`
  entries. Subsequent `get` on any key returns `None`. The cache is
  immediately usable.
- **Callers don't care about:** Internal counter initialization value, choice
  of backing data structure, or memory pre-allocation strategy.

### `Cache::get(&mut self, key: &K) -> Option<CacheGuard<'_, V>>`
- **Callers assume:**
  - Returns `Some(guard)` if the key is present, `None` otherwise.
  - The guard dereferences to the stored value (immutable or mutable access).
  - A successful `get` refreshes the entry's recency — the entry is no longer
    the LRU candidate (tests: `get_refreshes_lru_order`).
  - The cache size does not change.
  - The guard borrows the cache mutably; the caller cannot hold multiple
    guards or call other cache methods while a guard is live.
- **Callers don't care about:** The counter value, how recency is tracked
  internally, or the exact eviction ordering among entries with similar
  access patterns.

### `Cache::put(&mut self, key: K, value: V)`
- **Callers assume:**
  - **New key, below capacity:** the entry is inserted and immediately
    retrievable via `get`.
  - **New key, at capacity:** the least-recently-used entry is evicted first,
    then the new entry is inserted. Size stays at capacity
    (tests: `evicts_lru_entry_when_full`, `capacity_one`).
  - **Existing key (overwrite):** the value is replaced in-place; no eviction
    occurs; cache size does not change (tests: `put_overwrites_existing_key`,
    `overwrite_does_not_evict`).
  - **Zero-capacity cache:** `put` is a no-op (defensive guard in code).
  - After `put`, `get(key)` returns `Some` with the new value.
  - An overwrite refreshes the entry's recency.
- **Callers don't care about:** Which specific entry is evicted (only that it
  is the *least recently used* one), internal counter mechanics, or the
  eviction algorithm's time complexity.

### `Cache::remove(&mut self, key: &K)`
- **Callers assume:**
  - If the key exists, it is removed; subsequent `get` returns `None`.
  - If the key does not exist, the call is a no-op — no panic, no error
    (tests: `remove_nonexistent_key_is_noop`).
  - Cache size decreases by one (if key was present).
- **Callers don't care about:** Whether removed entries affect counter state
  or LRU ordering of remaining entries.

### `Cache::clear(&mut self)`
- **Callers assume:**
  - All entries are removed; any subsequent `get` returns `None`
    (tests: `clear_removes_all_entries`).
  - The cache is usable again (can `put` new entries after clearing).
- **Callers don't care about:** Whether the counter resets or the backing
  map's memory is freed or retained.

### `CacheGuard::deref(&self) -> &V`
- **Callers assume:** Yields an immutable reference to the value that was
  stored in the cache. The reference is valid for the guard's lifetime.
- **Callers don't care about:** Guard internals.

### `CacheGuard::deref_mut(&mut self) -> &mut V`
- **Callers assume:** Yields a mutable reference to the cached value,
  allowing in-place modification. Changes persist in the cache after the
  guard is dropped.
- **Callers don't care about:** Guard internals.

## Private Helper: `evict(&mut self)`
- Called only by `put` when `entries.len() >= capacity`.
- Removes the entry with the smallest `last_used` counter (the LRU victim).
- Callers of `put` expect exactly one entry to be evicted per over-capacity
  insertion.

## Abstract Resource

From the caller's perspective, this module manages a **bounded key-value
map with LRU eviction**. The cache maps keys (`K: Ord + Clone`) to values
(`V`) with a fixed maximum size. When the cache is full and a new key is
inserted, the least-recently-accessed entry is automatically evicted to make
room.

## Key Invariants (caller perspective)

1. **Capacity bound:** The number of entries never exceeds the capacity given
   at construction (`entries.len() <= capacity`).
2. **Key uniqueness:** Each key maps to at most one value.
3. **Lookup consistency:** After `put(k, v)` without an intervening `remove(k)`
   or eviction of `k`, `get(k)` returns `Some` containing `v` (or a later
   overwrite).
4. **LRU eviction correctness:** When eviction occurs, the evicted entry is the
   one that was accessed (via `get` or `put`) least recently — i.e., the entry
   whose last access is oldest among all entries.
5. **Overwrite preserves size:** Inserting a key that already exists replaces
   the value without changing the number of entries and without triggering
   eviction.
6. **Remove is idempotent:** Removing a non-existent key is a no-op.
7. **Clear resets state:** After `clear`, the cache behaves as if freshly
   constructed (empty, same capacity).
8. **Guard consistency:** A `CacheGuard` obtained from `get` provides a valid
   mutable reference to the entry's value; modifications through the guard
   are reflected in subsequent `get` calls.

## Pre-existing Specs (from upstream verification)

- **Source:** `src/libs/cache/src/lib.spec.rs`
- **Functions with specs:** *(none — the spec file contains an empty `verus!{}` block)*
- **Functions WITHOUT specs:** `new`, `get`, `put`, `remove`, `clear`, `evict`,
  `deref`, `deref_mut`
- **View type:** does not exist
- **Proof file:** `src/libs/cache/src/lib.proof.rs` — empty `verus!{}` block

### Assessment
- **Coverage:** none — empty stub files, no specifications written yet.
- **Strength:** n/a
- **View design:** does not exist — needs to be designed from scratch.

## Test Coverage Summary

The `#[cfg(test)]` module contains 10 tests covering:
- Miss on empty cache
- Put/get round-trip
- Overwrite semantics
- LRU eviction when full
- `get` refreshes LRU order
- Remove (existing and non-existent keys)
- Clear
- Capacity-one edge case
- Overwrite does not trigger eviction

These tests serve as the primary callers and encode the behavioral
expectations listed above.
