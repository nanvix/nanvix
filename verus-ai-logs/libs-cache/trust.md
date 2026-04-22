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

Additionally, `BTreeMap::get_mut` (used by `Cache::get` and `Cache::put`) has no
vstd spec even in the `std` btree module, and returns `Option<&mut V>` — a Verus
limitation on `&mut` return types.

`CacheGuard` contains `&'a mut V` which Verus cannot handle in struct
fields ("The verifier does not yet support &mut types, except in special cases").

## External Type Specifications

### ExBTreeMap — `lib.spec.rs:20-23`
- **Trust item:** `external_type_specification` + `external_body`
- **Classification:** `EXTERNAL_TYPE`
- **Reason:** `alloc::collections::BTreeMap` is not in vstd. Verus requires a type
  declaration to reference it in verified structs. `external_body` is needed because
  BTreeMap has private fields.
- **Reproducer:** Any `#[verus_verify]` struct containing `BTreeMap<K, V>` fails with
  "cannot use type `alloc::collections::BTreeMap` which is ignored because it is either
  declared outside the verus! macro or it is marked as `external`."

### ExGlobal — `lib.spec.rs:25-26`
- **Trust item:** `external_type_specification`
- **Classification:** `EXTERNAL_TYPE`
- **Reason:** `alloc::alloc::Global` is the default allocator for `BTreeMap<K, V>`.
  Declaring BTreeMap requires this type to be visible to Verus.

### ExCacheEntry — `lib.spec.rs:29-31`
- **Trust item:** `external_type_specification`
- **Classification:** `EXTERNAL_TYPE`
- **Reason:** `CacheEntry<V>` is a private struct used as the value type in
  `BTreeMap<K, CacheEntry<V>>`. Verus needs to see it to verify Cache.

### ExCacheGuard — `lib.spec.rs:35-38`
- **Trust item:** `external_type_specification` + `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** `CacheGuard<'a, V>` has field `value: &'a mut V`. Verus does not
  support `&mut` in struct field types. `external_body` hides the struct fields.
- **Reproducer:** Adding `#[verus_verify]` to `CacheGuard` produces
  "The verifier does not yet support &mut types, except in special cases".

## external_body Functions

### Cache::new — `lib.rs:147`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Body calls `BTreeMap::new()`. vstd btree specs unavailable on
  `no_std` target (see Root Cause). Cache::view() is uninterp, so body cannot
  be verified against abstract state even with specs.
- **Spec:** `result@ == CacheView::spec_new(capacity as nat)`

### Cache::get — `lib.rs:186`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Body calls `BTreeMap::get_mut()` which (a) has no vstd spec even
  in the `std` btree module, and (b) returns `Option<&mut V>` — a Verus `&mut`
  return type limitation. Also constructs `CacheGuard` with `&mut`.
- **Spec:** On hit: `result is Some`, guard view equals `spec_get(*key).1.unwrap()`,
  state transitions via `spec_get`. On miss: `result is None`, view unchanged.

### CacheGuard::deref — `lib.rs:95`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** CacheGuard is `external_body` (Verus cannot see `&mut V` field).
  The body `self.value` accesses the opaque field.
- **Spec:** `*ret == self@` — dereferencing yields the guard's abstract value.

### Cache::put — `lib.rs:216`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Same `get_mut` blockers as Cache::get (no vstd spec, `&mut` return
  type), plus calls `self.evict()`. vstd btree specs unavailable on `no_std` target.
- **Spec:** `self@ == old(self)@.spec_put(key, value)`, invariant preserved.

### Cache::remove — `lib.rs:262`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Calls `BTreeMap::remove()`. vstd btree specs unavailable on
  `no_std` target (see Root Cause).
- **Spec:** `self@ == old(self)@.spec_remove(*key)`, invariant preserved.

### Cache::clear — `lib.rs:279`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Calls `BTreeMap::clear()`. vstd btree specs unavailable on
  `no_std` target (see Root Cause).
- **Spec:** `self@ == old(self)@.spec_clear()`, invariant preserved.

### Cache::evict — `lib.rs:303`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Uses `self.entries.iter().min_by_key(|(_, e)| e.last_used)`
  iterator chain with closure, plus `BTreeMap::remove()`. Iterator combinators
  lack vstd specs. vstd btree specs unavailable on `no_std` target.
- **Spec:** LRU victim (`lru_order[0]`) evicted, contents/order updated accordingly.

## Unverifiable Functions

### CacheGuard::deref_mut — `lib.rs:101`
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

### Counter overflow — `lib.rs:192,224`
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
