# Integrity Audit Report: cache

Audited: 2026-04-23. All items challenged against verus-constraints escalation
ladder. No items eliminated — all are genuine trust boundaries.

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

## Items Eliminated

None. All 8 external_body items and 5 assume_specification items survive the
challenge. Each was systematically tested against the verus-constraints
escalation ladder (verify as-is → search vstd → minimal rewrite → stdlib
wrapper → external_body).

## Challenge Log

### 1. ExBTreeMap (lib.vstd_btree.rs:32) — external_type_specification + external_body
- **Classification:** EXTERNAL_TYPE
- **Challenge:** Can alloc::collections::BTreeMap be declared without external_body?
- **Result:** No. BTreeMap has private fields. Verus requires external_body to
  hide struct internals for external types. vstd uses the same pattern.
- **Verdict:** KEEP

### 2. ExCacheGuard (lib.spec.rs:23-25) — external_type_specification + external_body
- **Classification:** VERUS_LIMITATION
- **Challenge:** Can CacheGuard be declared without external_body?
- **Result:** No. CacheGuard has field `value: &'a mut V`. Verus error: "The
  verifier does not yet support &mut types, except in special cases."
- **Reproducer:** Adding `#[verus_verify]` to the CacheGuard struct definition
  produces the &mut type error.
- **Verdict:** KEEP

### 3. btreemap_remove (lib.rs:114-123) — stdlib wrapper
- **Classification:** STDLIB_WRAPPER
- **Challenge:** Can BTreeMap::remove be given an assume_specification directly?
- **Result:** No. BTreeMap::remove has signature `fn remove<Q: ?Sized>(&mut self,
  key: &Q) -> Option<V> where K: Borrow<Q>, Q: Ord`. The `Borrow<Q>` bound
  cannot be expressed in assume_specification for alloc::collections::BTreeMap
  (unlike std::collections where vstd handles it). The wrapper fixes Q=K.
- **vstd check:** vstd's btree specs for remove are gated behind cfg(std),
  unavailable on this no_std target.
- **Verdict:** KEEP

### 4. CacheGuard::deref (lib.rs:93-99) — external_body
- **Classification:** VERUS_LIMITATION
- **Challenge:** Can deref body `self.value` be verified?
- **Result:** No. CacheGuard is external_body (item 2), so field `value` is
  opaque to the verifier. The body accesses an invisible field.
- **Dependency:** Blocked by item 2 (ExCacheGuard).
- **Verdict:** KEEP

### 5. Cache::get (lib.rs:190-218) — external_body
- **Classification:** VERUS_LIMITATION
- **Challenge:** Can Cache::get be rewritten to avoid &mut returns?
- **Result:** No. Two independent blockers:
  (a) `self.entries.get_mut(key)` returns `Option<&mut CacheEntry<V>>` — Verus
      does not support &mut return types.
  (b) Constructs `CacheGuard { value: &mut entry.value }` — CacheGuard
      contains &mut V (item 2).
  Even if get_mut were replaced (e.g., remove + re-insert), blocker (b) remains.
  Rewriting the public API to avoid CacheGuard would change the module's
  interface — forbidden by source integrity.
- **Verdict:** KEEP

### 6. Cache::put (lib.rs:230-265) — external_body
- **Classification:** VERUS_LIMITATION
- **Challenge:** Can Cache::put be rewritten to avoid get_mut?
- **Result:** Theoretically possible (remove + re-insert), but:
  (a) Changes exec code significantly (in-place update → remove + insert).
  (b) Requires a new axiom `axiom_cache_lru_of_insert` linking fresh counter
      values to MRU position — counter monotonicity invariant does not exist
      in the current spec architecture.
  (c) Net trust does NOT decrease: replaces 1 external_body with changed exec
      code + 1 new axiom.
  The architecture is fundamentally blocked without a counter monotonicity
  invariant linking concrete last_used values to abstract lru_order position.
- **Verdict:** KEEP

### 7. find_lru_victim (lib.rs:315-331) — external_body
- **Classification:** VERUS_LIMITATION
- **Challenge:** Can the iterator chain be verified or rewritten as a loop?
- **Result:** No.
  (a) `min_by_key` has no vstd spec.
  (b) `BTreeMap::iter` has no vstd spec on no_std (gated behind cfg(std)).
  (c) Even with a manual loop rewrite, iteration over BTreeMap entries requires
      vstd's ForLoopGhostIteratorNew for BTreeMap::Iter, which is unavailable.
- **vstd check:** `grep -r "min_by_key" ~/.cargo/registry/src/*/vstd-*/` — no
  results. `grep -r "ForLoopGhostIteratorNew.*BTreeMap"` — only in
  std_specs/btree.rs behind cfg(std).
- **Verdict:** KEEP

### 8. axiom_cache_lru_of_remove (lib.proof.rs:401-411) — external_body on proof fn
- **Classification:** VERUS_LIMITATION
- **Challenge:** Can this axiom be proven instead of assumed?
- **Result:** No. `cache_lru_of` is partially uninterpreted: for non-empty maps
  it delegates to `cache_lru_of_nonempty` (uninterp spec fn). The ordering
  depends on CacheEntry::last_used counter values inside the BTreeMap, which
  are inaccessible through the uninterpreted btreemap_view_spec.
- **Soundness argument:** BTreeMap::remove preserves all other entries' fields
  (including last_used), so the sorted order excluding the removed key equals
  the original order filtered by != key.
- **Verdict:** KEEP

### assume_specification items (lib.vstd_btree.rs)

All 5 items are adapted from upstream vstd (v0.0.0-2026-04-05) for
alloc::collections::BTreeMap. They exist because vstd gates btree specs behind
cfg(std), unavailable on this no_std kernel target.

| Function | Line | Classification | Fidelity |
|----------|------|----------------|----------|
| BTreeMap::new | 69-73 | EXTERNAL_BOTTOM | Matches upstream |
| BTreeMap::len | 88-95 | EXTERNAL_BOTTOM | Drops `key_obeys_cmp_spec` guard (stronger) |
| BTreeMap::is_empty | 98-105 | EXTERNAL_BOTTOM | Matches upstream |
| BTreeMap::insert | 108-122 | EXTERNAL_BOTTOM | Drops `obeys_cmp_spec` guard (stronger) |
| BTreeMap::clear | 130-137 | EXTERNAL_BOTTOM | Matches upstream |

**Fidelity note:** `len` and `insert` drop upstream ordering guards
(`obeys_cmp_spec` / `key_obeys_cmp_spec`). These guards ensure the `Ord`
implementation is well-formed (antisymmetric, transitive, total). The local
specs unconditionally assume `K: Ord` is correctly implemented. Practical risk
is low (all standard types satisfy this), but is an additional trust assumption.

## Items Remaining in trust.md

All items from the original trust.md survive. See trust.md for the complete
list with function names, locations, classifications, and reproducers.

Summary:
- 2 external_type_specification + external_body (ExBTreeMap, ExCacheGuard)
- 1 external_type_specification only (ExGlobal, ExCacheEntry)
- 5 external_body on exec functions (btreemap_remove, deref, get, put, find_lru_victim)
- 1 external_body on proof fn (axiom_cache_lru_of_remove)
- 5 assume_specification (BTreeMap methods)
- 2 broadcast axiom (finite domain, len spec)
- 1 counter overflow trust assumption
- 1 unverified function (deref_mut — no spec possible)

## AST Consistency

- **Matched:** 15 (3 structs: Cache, CacheEntry, CacheGuard; 12 functions:
  Cache::clear, Cache::get, Cache::put, CacheGuard::deref, CacheGuard::deref_mut,
  plus 7 test functions)
- **Mismatched:** 3 (all pre-approved deviations — see analysis below)
- **Missing:** 0
- **Extra in verus:** 2 (find_lru_victim, btreemap_remove — both documented
  stdlib wrappers / function extractions)

### Mismatch Analysis

**Cache::new (MISMATCH) — Pre-approved deviation**
- Change: `Self { ... }` → `let result = Self { ... }; result`
- Reason: Named return binding for ensures clause reference.
- Pre-approved pattern: `Ok(Self { .. })` → `let result = Self { .. }; Ok(result)`
- Ghost code: `proof! { Self::lemma_new_view(&result, capacity); }` (erased)
- Semantics: Identical. The let binding is a no-op in Rust.

**Cache::remove (MISMATCH) — Stdlib wrapper substitution**
- Change: `self.entries.remove(key)` → `btreemap_remove(&mut self.entries, key)`
- Reason: BTreeMap::remove's Borrow<Q> generic can't be monomorphized in
  assume_specification. Wrapper fixes Q=K and body is `m.remove(k)` — identical
  stdlib call.
- Ghost code: `proof! { Self::lemma_remove_view(...); }` (erased)
- Semantics: Identical. Wrapper is inline-equivalent.

**Cache::evict (MISMATCH) — Function extraction + stdlib wrapper**
- Change: Inline iterator chain extracted to `find_lru_victim`; `self.entries.remove(&key)` → `btreemap_remove(&mut self.entries, &key)`.
- Reason: Iterator chain uses min_by_key (no vstd spec) — isolated in
  external_body helper. BTreeMap::remove → btreemap_remove (same as Cache::remove).
- Ghost code: `proof! { Self::lemma_evict_view(...); }` (erased)
- Semantics: Identical. find_lru_victim body contains the exact original
  iterator chain.

### Extra Function Analysis

**find_lru_victim (EXTRA_IN_VERUS):** Extracted from evict to isolate
unverifiable iterator chain. Body is the original code. Marked external_body
with spec linking result to cache_lru_of.

**btreemap_remove (EXTRA_IN_VERUS):** Stdlib wrapper for BTreeMap::remove.
Body is `m.remove(k)`. Marked external_body with pre/post conditions matching
BTreeMap::remove documentation.

## Result: PASS

All cheating items are genuine trust boundaries that cannot be eliminated within
current Verus capabilities. No code changes were made. The verification passes
with 0 errors. All AST mismatches are pre-approved deviations or documented
stdlib wrapper substitutions.
