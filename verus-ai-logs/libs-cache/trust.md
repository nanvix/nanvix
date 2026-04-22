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
  type). Rewriting to avoid `get_mut` would change the existing-key path from
  in-place mutation to a `remove` + `insert` sequence — a structural exec code
  modification violating source integrity. The `contains_key` alternative also
  has the `Borrow<Q>` blocker (same as `remove`).
- **Spec:** `self@ == old(self)@.spec_put(key, value)`, invariant preserved.

### Cache::find_lru_victim — `lib.rs:315-331`
- **Trust item:** `external_body`
- **Classification:** `VERUS_LIMITATION`
- **Reason:** Uses `entries.iter().min_by_key(|(_, e)| e.last_used).map(|(k, _)| k.clone())`
  iterator chain with closure. `min_by_key` has no vstd spec. vstd does have
  `BTreeMap::iter` and `Iter::next` specs (with ForLoopGhostIteratorNew) but gated
  behind `cfg(std)`, unavailable on this no\_std target. Even with iterator specs,
  `min_by_key` would still require a manual loop rewrite.
- **Spec:** Returns the key with the smallest `last_used` counter (= `cache_lru_of(*entries)[0]`).
  Returns `None` iff entries is empty.

### Cache::evict — `lib.rs:338-360`
- **Trust item:** None (body-verified)
- **Note:** `evict` itself is NOT external_body. Its body is verified using
  `find_lru_victim` (external_body) and `btreemap_remove` (external_body) as
  trusted helpers. The eviction logic (call find_lru_victim, remove victim) is
  proven correct.

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

## assume_specification Items (lib.vstd_btree.rs)

These are adapted from upstream vstd specs for `alloc::collections::BTreeMap`.
They exist because vstd gates its btree specs behind `cfg(std)` which is unavailable
on this no\_std kernel target. The import path (`alloc::` vs `std::`) and type
parameters (`A: Allocator + Clone` exposed) differ from upstream.

**Fidelity deviation:** Two local specs drop upstream guards, making them
unconditionally stronger:

| Function | Line | Classification | Upstream Guard | Local |
|---|---|---|---|---|
| `BTreeMap::new` | lib.vstd\_btree.rs:69-73 | `EXTERNAL_BOTTOM` | none | none |
| `BTreeMap::len` | lib.vstd\_btree.rs:88-95 | `EXTERNAL_BOTTOM` | `key_obeys_cmp_spec::<Key>()` on axiom | **dropped** |
| `BTreeMap::is_empty` | lib.vstd\_btree.rs:98-105 | `EXTERNAL_BOTTOM` | none | none |
| `BTreeMap::insert` | lib.vstd\_btree.rs:108-122 | `EXTERNAL_BOTTOM` | `obeys_cmp_spec::<Key>()` | **dropped** |
| `BTreeMap::clear` | lib.vstd\_btree.rs:130-137 | `EXTERNAL_BOTTOM` | none | none |

The dropped guards (`obeys_cmp_spec` / `key_obeys_cmp_spec`) ensure the `Ord`
implementation is well-formed (antisymmetric, transitive, total). The local specs
unconditionally assume `K: Ord` is correctly implemented. Practical risk is low —
all standard types satisfy this — but this is an additional trust assumption beyond
upstream vstd. The upstream guards exist because vstd is maximally conservative;
this crate trades that conservatism for simpler proofs.

### broadcast axiom Declarations

| Axiom | Line | Purpose |
|---|---|---|
| `axiom_btree_map_view_finite_dom` | lib.vstd\_btree.rs:56-61 | BTreeMap view domain is finite |
| `axiom_spec_btree_map_len` | lib.vstd\_btree.rs:80-85 | Connects `spec_btree_map_len` to `btreemap_view_spec.len()` |

Source: `~/.cargo/registry/src/.../vstd-0.0.0-2026-04-12-0118/std_specs/btree.rs`

## Integrity Audit

Re-audited 2026-04-23. All 8 external_body and 5 assume_specification items
challenged against verus-constraints escalation ladder (verify as-is → search
vstd → minimal rewrite → stdlib wrapper). None eliminated — all are genuine
trust boundaries. Verified against vstd 0.0.0-2026-04-12-0118 (btree specs
still gated behind `cfg(all(feature="alloc", feature="std"))`; no get_mut spec
in any version). Two assume_specification items (insert, len axiom) are stronger
than upstream vstd due to dropped `obeys_cmp_spec` guards. AST consistency:
15 matched, 3 mismatched (all pre-approved deviations or justified VERUS
REWRITEs), 0 missing, 2 extra (new wrapper/helper functions). See
`integrity-audit/fix_report.md` for full challenge analysis.
