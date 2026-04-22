# Integrity Audit Report: cache

Audited: 2026-04-23

## Cheating Counts (before → after)

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 8 | 8 | 0 |
| trusted | 0 | 0 | 0 |
| no_decreases | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |
| assume_specification | 5 | 5 | 0 |

Additional trust items (not in cheating counter):
- broadcast axiom: 2 (axiom_btree_map_view_finite_dom, axiom_spec_btree_map_len)
- uninterp spec fn: 3 (btreemap_view_spec, spec_btree_map_len, cache_lru_of_nonempty)

### External Body Breakdown (8 total)

| # | Item | File:Line | Type | Classification |
|---|------|-----------|------|----------------|
| 1 | ExBTreeMap | lib.vstd_btree.rs:31-38 | type spec | EXTERNAL_TYPE |
| 2 | ExCacheGuard | lib.spec.rs:23-25 | type spec | VERUS_LIMITATION |
| 3 | CacheGuard::deref | lib.rs:93-99 | exec fn | VERUS_LIMITATION |
| 4 | btreemap_remove | lib.rs:114-123 | exec fn | STDLIB_WRAPPER |
| 5 | Cache::get | lib.rs:190-218 | exec fn | VERUS_LIMITATION |
| 6 | Cache::put | lib.rs:230-265 | exec fn | VERUS_LIMITATION |
| 7 | Cache::find_lru_victim | lib.rs:315-331 | exec fn | VERUS_LIMITATION |
| 8 | axiom_cache_lru_of_remove | lib.proof.rs:401-411 | proof fn | VERUS_LIMITATION |

## Items Eliminated

None. All 8 external_body items are at genuine trust boundaries. Each was
challenged against the verus-constraints escalation ladder (verify as-is →
search vstd → minimal rewrite → stdlib wrapper → external_body). Detailed
challenge analysis follows.

## Detailed Challenge Analysis

### 1. btreemap_remove (lib.rs:114-123) — KEEP

**Classification:** STDLIB_WRAPPER

**Challenge:** Can we use `assume_specification` for `BTreeMap::remove` directly?

**Result:** No. `BTreeMap::remove` has signature `fn remove<Q>(&mut self, key: &Q) -> Option<V>`
where `K: Borrow<Q>` and `Q: Ord`. vstd's upstream spec (btree.rs:776-790) handles
`Borrow<Q>` via uninterpreted helper functions (`borrowed_key_removed`,
`maps_borrowed_key_to_value`) and multiple axioms. Adapting this full infrastructure for
`alloc::collections` would require copying ~40 lines of uninterpreted specs and axioms.
The wrapper fixes `Q=K`, reducing the spec to a simple
`btreemap_view_spec(*m) == old.remove(*k)`. Body is a single stdlib call — the thinnest
possible trust layer.

### 2. CacheGuard::deref (lib.rs:93-99) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can we verify the body?

**Result:** No. `CacheGuard` itself is `external_body` because it contains `&'a mut V` in a
struct field, which Verus does not support ("The verifier does not yet support &mut types,
except in special cases"). Since the struct is opaque, field access `self.value` cannot be
verified.

### 3. Cache::get (lib.rs:190-218) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can we rewrite to avoid `get_mut`?

**vstd search:** Confirmed `BTreeMap::get_mut` has no vstd spec in any version (searched
vstd 0.0.0-2026-03-15 through 0.0.0-2026-04-05). Not even for `std::collections`.

**Result:** No. Three independent blockers: (a) `BTreeMap::get_mut` has no vstd spec,
(b) it returns `Option<&mut V>` — a Verus `&mut` return type limitation, (c) constructs
`CacheGuard { value: &mut entry.value }` requiring `&mut` access into BTreeMap.

**Rewrite considered:** Replace `get_mut` with `remove` + `insert`. Rejected because
even with the round-trip, constructing `CacheGuard` wrapping `&mut V` requires mutable
access into the BTreeMap, which demands `get_mut` or equivalent. The CacheGuard blocker
is fundamental and cannot be worked around by restructuring the map access.

### 4. Cache::put (lib.rs:230-265) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can we rewrite to avoid `get_mut` and body-verify?

**Result:** No. A rewrite using `btreemap_remove` + `insert` instead of `get_mut` would:
(a) require two new axioms — `axiom_cache_lru_of_insert` (how insert affects LRU ordering)
and proof relating fresh counter to MRU position — since `cache_lru_of` is uninterpreted
for non-empty maps; (b) change exec code from in-place mutation to remove/reinsert cycle —
a substantial structural modification; (c) increase net trust count (1 external_body →
2+ axioms + exec rewrite). The current single `external_body` on `put` is the smallest
trust surface.

### 5. Cache::find_lru_victim (lib.rs:315-331) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can we rewrite the iterator chain as a manual `for` loop?

**vstd search:** vstd DOES have `BTreeMap::iter` and `btree_map::Iter::next` specs plus
`ForLoopGhostIteratorNew` ghost iterator infrastructure (btree.rs:289-430). However, these
are gated behind `cfg(std)` and use `std::collections`, making them unavailable on this
no\_std target.

**Result:** Rewrite rejected. Would require: (a) copying ~60 lines of iterator
infrastructure from vstd to lib.vstd_btree.rs (`assume_specification` for `iter`, `next`,
`ForLoopGhostIteratorNew` impl, `MapIterGhostIterator` struct); (b) rewriting the 3-line
iterator chain as a `for` loop with invariants. Net trust: trades 1 external_body for
~3 `assume_specification` items — formal trust count increases. The current 3-line
external_body function is trivially auditable and preferable to spreading trust across
multiple declarations.

### 6. axiom_cache_lru_of_remove (lib.proof.rs:401-411) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can this axiom be proven instead of assumed?

**Result:** No. `cache_lru_of` delegates to the uninterpreted `cache_lru_of_nonempty` for
non-empty maps. There is no definitional body to reason about. Making `cache_lru_of`
concrete would require defining a spec-level sort over `Map<K, CacheEntry<V>>` entries by
`last_used`. vstd's `Map` type has no ordering primitives, so this would need its own
axioms — making it net-neutral. The axiom is sound because `BTreeMap::remove` does not
change `last_used` counters of remaining entries, preserving their relative sort order.

### 7. ExBTreeMap (lib.vstd_btree.rs:31-38) — KEEP

**Classification:** EXTERNAL_TYPE

**Challenge:** Can we use vstd's BTreeMap support directly?

**Result:** No. vstd's btree specs are gated behind `cfg(all(feature = "alloc", feature = "std"))`
and import from `std::collections`. This crate targets `i686-nanvix` (no\_std) where the
`std` crate does not exist.

### 8. ExCacheGuard (lib.spec.rs:23-25) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can we avoid external_body on the type?

**Result:** No. `CacheGuard` has field `value: &'a mut V`. Verus error: "The verifier does not
yet support &mut types, except in special cases" on the struct definition.

## assume_specification Fidelity

All 5 items are adapted from upstream vstd (v0.0.0-2026-04-05-0114, `std_specs/btree.rs`).
Two specs are **stronger** than upstream:

| Function | Upstream Guard | Local | Difference |
|----------|---------------|-------|------------|
| BTreeMap::new | none | none | Identical |
| BTreeMap::len | `key_obeys_cmp_spec::<Key>()` on axiom | none | Stronger |
| BTreeMap::is_empty | none | none | Identical |
| BTreeMap::insert | `obeys_cmp_spec::<Key>()` | none | Stronger |
| BTreeMap::clear | none | none | Identical |

The dropped guards (`obeys_cmp_spec` / `key_obeys_cmp_spec`) ensure the `Ord`
implementation is well-formed (antisymmetric, transitive, total). The local specs
unconditionally assume `K: Ord` is correct. Practical risk is low — all standard types
satisfy this — but this is an additional trust assumption beyond upstream vstd. The
upstream guards exist because vstd is maximally conservative; this crate trades that
conservatism for simpler proofs since the cache only uses well-behaved key types.

## Items Remaining in trust.md

All 8 external_body items remain. See trust.md for full documentation with
function names, locations, classifications, and justifications. Updated to
document the `obeys_cmp_spec` guard deviation.

## AST Consistency

- **Matched:** 15
- **Mismatched:** 3 (Cache::new, Cache::remove, Cache::evict)
- **Missing:** 0
- **Extra in Verus:** 2 (find_lru_victim, btreemap_remove)

### MISMATCH 1: Cache::new — Pre-approved deviation

```diff
--- source
+++ verus
     pub const fn new(capacity: usize) -> Self {
-        Self {
+        let result = Self {
             entries: BTreeMap::new(),
             counter: 0,
             capacity,
+        };
+        proof! {
+            Self::lemma_new_view(&result, capacity);
         }
+        result
     }
```

**Category:** Pre-approved: `Ok(Self { .. })` → `let result = Self { .. }; result`
(intermediate variable for ensures reference). The `proof!{}` block is erased under
normal build.
**Semantics preserved:** Yes — identical observable behavior.
**Action:** ACCEPT.

### MISMATCH 2: Cache::remove — Stdlib wrapper substitution

```diff
--- source
+++ verus
     pub fn remove(&mut self, key: &K) {
-        self.entries.remove(key);
+        btreemap_remove(&mut self.entries, key);
+        proof! {
+            Self::lemma_remove_view(self, *key, old(self).entries, old(self).capacity);
+        }
     }
```

**Category:** Escalation ladder step 4 (stdlib wrapper). `BTreeMap::remove`'s `Borrow<Q>`
generic prevents `assume_specification`. The wrapper `btreemap_remove` fixes `Q=K` and
body is `m.remove(k)` — single stdlib call. The `proof!{}` block is erased.
**Semantics preserved:** Yes — `btreemap_remove` body is `m.remove(k)`.
**Action:** ACCEPT. Documented with `// VERUS REWRITE` comment in source.

### MISMATCH 3: Cache::evict — Iterator extraction + stdlib wrapper

```diff
--- source
+++ verus
     fn evict(&mut self) {
-        let victim: Option<K> = self
-            .entries
-            .iter()
-            .min_by_key(|(_, e)| e.last_used)
-            .map(|(k, _)| k.clone());
-        if let Some(key) = victim {
-            self.entries.remove(&key);
+        if let Some(key) = Self::find_lru_victim(&self.entries) {
+            btreemap_remove(&mut self.entries, &key);
+            proof! {
+                Self::lemma_evict_view(self, key, old(self).entries, old(self).capacity);
+            }
         }
     }
```

**Category:** Escalation ladder step 4 (stdlib wrapper). Iterator chain
(`iter().min_by_key().map()`) has no vstd specs; extracted to `find_lru_victim` to
minimize external_body scope. `self.entries.remove(&key)` replaced with
`btreemap_remove` (same wrapper as Cache::remove). The `proof!{}` block is erased.
**Semantics preserved:** Yes — `find_lru_victim` body is the original iterator chain.
`btreemap_remove` body is `m.remove(k)`.
**Action:** ACCEPT. Documented with `// VERUS REWRITE` comments in source.

### EXTRA 1: Cache::find_lru_victim

New static method extracting the iterator chain from `evict`. Body is identical to the
original inline iterator code. Marked `external_body` with spec connecting result to
`cache_lru_of`. Required to isolate the unverifiable iterator chain into the smallest
possible external_body scope.

**Action:** ACCEPT (stdlib wrapper pattern).

### EXTRA 2: btreemap_remove

New crate-level function wrapping `BTreeMap::remove` with fixed `Q=K` type parameter.
Body is `m.remove(k)` — single stdlib call. Marked `external_body` with pre/post
conditions.

**Action:** ACCEPT (stdlib wrapper pattern, classified STDLIB_WRAPPER in trust.md).

## Result: PASS

All `external_body` items are at genuine trust boundaries that cannot be eliminated
without adding equivalent or greater trust elsewhere. No `admit()`, `assume()`,
`trusted`, or `cfg-gated exec` cheating detected. AST mismatches are pre-approved
deviations or documented Verus rewrites. Verification passes with 0 errors.
No blockers.
