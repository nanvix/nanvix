# Trust Boundary Audit — libs/cache

This file documents all trust items (external_body, admit, external_type_specification)
used in the cache crate verification.

## Root Cause

vstd provides BTreeMap specifications in `vstd::std_specs::btree` (View trait,
`assume_specification` for `new`, `insert`, `get`, `remove`, `clear`, `len`, `iter`,
`contains_key`, `keys`, `values`). However, the btree module is gated behind
`cfg(all(feature = "alloc", feature = "std"))` and internally imports from
`std::collections`, making it structurally incompatible with this crate's `no_std`
kernel target (`-Z build-std=core,alloc,compiler_builtins`). Enabling the `std`
feature is not possible because the `std` crate does not exist on the i686-nanvix
kernel target.

To work around this, `lib.vstd_btree.rs` provides equivalent
`assume_specification` declarations for `alloc::collections::BTreeMap` methods
(`new`, `insert`, `len`, `is_empty`, `clear`) and an uninterpreted
`btreemap_view_spec` function instead of `impl View for BTreeMap`.

Additionally, `BTreeMap::get_mut` (used by `Cache::get` and `Cache::put`) has no
vstd spec even in the `std` btree module, and returns `Option<&mut V>` — a Verus
limitation on `&mut` return types.

`CacheGuard` contains `&'a mut V` which Verus cannot handle in struct
fields ("The verifier does not yet support &mut types, except in special cases").

## External Type Specifications

### ExBTreeMap — `lib.vstd_btree.rs:31-38`
- **Trust item:** `external_type_specification` + `external_body`
- **Classification:** `EXTERNAL_TYPE`
- **Reason:** `alloc::collections::BTreeMap` is not in vstd on no_std targets.
  Verus requires a type declaration to reference it in verified structs.
  `external_body` is needed because BTreeMap has private fields.
- **Reproducer:** Any `#[verus_verify]` struct containing `BTreeMap<K, V>` fails with
  "cannot use type `alloc::collections::BTreeMap` which is ignored because it is either
  declared outside the verus! macro or it is marked as `external`."

### ExGlobal — `lib.vstd_btree.rs:40-41`
- **Trust item:** `external_type_specification`
- **Classification:** `EXTERNAL_TYPE`
- **Reason:** `alloc::alloc::Global` is the default allocator for `BTreeMap<K, V>`.
  Declaring BTreeMap requires this type to be visible to Verus.

### ExCacheEntry — `lib.spec.rs:17-18`
- **Trust item:** `external_type_specification`
- **Classification:** `EXTERNAL_TYPE`
- **Reason:** `CacheEntry<V>` is a private struct used as the value type in
  `BTreeMap<K, CacheEntry<V>>`. Verus needs to see it to verify Cache.

### ExCacheGuard — `lib.spec.rs:23-25`
- **Trust item:** `external_type_specification` + `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** `CacheGuard<'a, V>` has field `value: &'a mut V`. Verus does not
  support `&mut` in struct field types. `external_body` hides the struct fields.
- **Reproducer:** Adding `#[verus_verify]` to `CacheGuard` produces
  "The verifier does not yet support &mut types, except in special cases".

## external_body Functions

### btreemap_remove — `lib.rs:114-123`
- **Trust item:** `external_body`
- **Classification:** `STDLIB_WRAPPER`
- **Reason:** `BTreeMap::remove()` has a `Borrow<Q>` generic parameter that
  cannot be monomorphized in `assume_specification` for
  `alloc::collections::BTreeMap`. This thin wrapper fixes Q=K and provides
  pre/post conditions. Body is a single stdlib call.
- **Spec:** `btreemap_view_spec(*m) == old(*m).remove(*k)`, returns the removed
  value if present.

### CacheGuard::deref — `lib.rs:93-99`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** CacheGuard is `external_body` (Verus cannot see `&mut V` field).
  The body `self.value` accesses the opaque field.
- **Spec:** `*ret == self@` — dereferencing yields the guard's abstract value.

### Cache::get — `lib.rs:190-218`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Body calls `BTreeMap::get_mut()` which (a) has no vstd spec even
  in the `std` btree module, and (b) returns `Option<&mut V>` — a Verus `&mut`
  return type limitation. Also constructs `CacheGuard` with `&mut`.
- **Spec:** On hit: `result is Some`, guard view equals `spec_get(*key).1.unwrap()`,
  state transitions via `spec_get`. On miss: `result is None`, view unchanged.

### Cache::put — `lib.rs:230-265`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Same `get_mut` blockers as Cache::get (no vstd spec, `&mut` return
  type). Rewriting to avoid `get_mut` (using `contains_key` + `remove` +
  `insert`) would require a new `axiom_cache_lru_of_insert` trust item plus a
  concrete counter invariant, increasing total trust items without reducing
  the trust boundary.
- **Spec:** `self@ == old(self)@.spec_put(key, value)`, invariant preserved.

### Cache::evict — `lib.rs:321-344`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Uses `self.entries.iter().min_by_key(|(_, e)| e.last_used)`
  iterator chain with closure, plus `BTreeMap::remove()`. Iterator combinators
  (`iter`, `min_by_key`, `map`) have no vstd specs.
- **Spec:** LRU victim (`lru_order[0]`) evicted, contents/order updated accordingly.

### axiom_cache_lru_of_remove — `lib.proof.rs:408-418`
- **Trust item:** `external_body` (on proof fn)
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Axiom relating the uninterpreted `cache_lru_of` function to
  BTreeMap removal. `cache_lru_of` is partially uninterpreted (uses
  `cache_lru_of_nonempty` for non-empty maps) because the concrete LRU
  ordering depends on `CacheEntry::last_used` counter values, which cannot
  be expressed as a closed-form spec function over `BTreeMap` entries.
  Sound because `BTreeMap::remove` does not change `last_used` counters of
  remaining entries, so their relative sort order is preserved.
- **Spec:** `cache_lru_of(new) == cache_lru_of(old).filter(|k| k != key)`.

## Body-Verified Functions

The following functions are verified without `external_body`:

| Function | Status |
|---|---|
| `Cache::new` | ✅ Body verified (calls `BTreeMap::new` with local `assume_specification`) |
| `Cache::remove` | ✅ Body verified (uses `btreemap_remove` wrapper + `axiom_cache_lru_of_remove`) |
| `Cache::clear` | ✅ Body verified (calls `BTreeMap::clear` with local `assume_specification`) |

## Unverifiable Functions

### CacheGuard::deref_mut — `lib.rs:102-105`
- **Trust item:** No spec (function excluded from verification)
- **Classification:** `VERUS_LIMITATION`
- **Reason:** `deref_mut` returns `&mut V`. Verus error: "The verifier does not
  yet support &mut types, except in special cases" (on the function signature).
  The function cannot have any spec annotation at all.
- **Reproducer:** Adding `#[verus_verify(external_body)]` to `deref_mut` produces
  the `&mut` type error.
- **Impact:** Mutation-through-guard semantics are unmodeled. Callers who mutate
  a value through `*guard = new_value` have no formal guarantee the change
  persists in the cache. This is documented but cannot be resolved until Verus
  adds `&mut` return type support.

## Trust Assumptions

### Counter overflow — `lib.rs:210,246`
- **Assumption:** `self.counter` (type `u64`) never overflows during the
  lifetime of a Cache instance.
- **Classification:** `VERUS_LIMITATION` (precondition omitted)
- **Justification:** At 10 billion ops/sec, overflow requires ~58 years.
  Physically unreachable. Adding `requires self.counter < u64::MAX` would
  burden every caller with a practically-impossible proof obligation.
- **See also:** `bugs.md` BUG-1.

## admit() in Proof Stubs

All five admit() placeholders have been eliminated. The invariant preservation
lemmas are now fully proven:

| Lemma | Status |
|---|---|
| `lemma_spec_new_inv` | ✅ Proven |
| `lemma_spec_get_inv` | ✅ Proven |
| `lemma_spec_put_inv` | ✅ Proven |
| `lemma_spec_remove_inv` | ✅ Proven |
| `lemma_spec_clear_inv` | ✅ Proven |
