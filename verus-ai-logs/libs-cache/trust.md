# Trust Boundary Audit — libs/cache

This file documents all trust items (external_body, admit, external_type_specification)
used in the cache crate verification.

## Root Cause

`BTreeMap` (from `alloc::collections`) has no vstd specifications — no View trait,
no `assume_specification` for any method (`new`, `get_mut`, `insert`, `remove`, `clear`,
`len`, `iter`, `min_by_key`). This makes body verification of any Cache method impossible.
All methods are marked `external_body` with comprehensive pre/post conditions.

Additionally, `CacheGuard` contains `&'a mut V` which Verus cannot handle in struct
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

### Cache::new — `lib.rs:136`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Body calls `BTreeMap::new()` which has no vstd spec.
- **Spec:** `result@ == CacheView::spec_new(capacity as nat)`

### Cache::get — `lib.rs:166`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Body calls `BTreeMap::get_mut()` which has no vstd spec. Also
  constructs `CacheGuard` with `&mut` which Verus cannot handle.
- **Spec:** On hit: `result is Some`, view transitions via `spec_get`, contents
  and capacity preserved. On miss: `result is None`, view unchanged.

### Cache::put — `lib.rs:207`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Body calls `BTreeMap::get_mut()`, `BTreeMap::len()`,
  `BTreeMap::insert()`, and `self.evict()`. None have vstd specs.
- **Spec:** `self@ == old(self)@.spec_put(key, value)`, invariant preserved,
  put-get round-trip for capacity > 0, zero-capacity no-op.

### Cache::remove — `lib.rs:261`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Body calls `BTreeMap::remove()` which has no vstd spec.
- **Spec:** `self@ == old(self)@.spec_remove(*key)`, key no longer present.

### Cache::clear — `lib.rs:283`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Body calls `BTreeMap::clear()` which has no vstd spec.
- **Spec:** `self@ == old(self)@.spec_clear()`, empty contents and order.

### Cache::evict — `lib.rs:304`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Body calls `BTreeMap::iter()`, `Iterator::min_by_key()` with a
  closure, and `BTreeMap::remove()`. None have vstd specs. The iterator chain
  with closure is also problematic for Verus.
- **Spec:** LRU victim (`lru_order[0]`) evicted, contents/order updated accordingly.

## admit() in Proof Stubs

All five lemmas in `lib.proof.rs` use `admit()`. These are specification-phase
placeholders to be filled during the proving phase.

| Lemma | Line | Classification |
|---|---|---|
| `lemma_spec_new_inv` | proof.rs:17 | `TEMPORARY` |
| `lemma_spec_get_inv` | proof.rs:25 | `TEMPORARY` |
| `lemma_spec_put_inv` | proof.rs:35 | `TEMPORARY` |
| `lemma_spec_remove_inv` | proof.rs:45 | `TEMPORARY` |
| `lemma_spec_clear_inv` | proof.rs:55 | `TEMPORARY` |
