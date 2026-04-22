# Integrity Audit Report: cache

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

**Additional tracking (not cheating, but trust surface):**
- broadcast axiom: 2 (axiom_btree_map_view_finite_dom, axiom_spec_btree_map_len)
- uninterp spec fn: 4 (btreemap_view_spec, spec_btree_map_len, cache_lru_of_nonempty, CacheGuard::view)
- Implicitly unverified function: 1 (CacheGuard::deref_mut — no `#[verus_verify]`, &mut return type)

## Items Eliminated

None. All 8 external_body items survived challenge per the verus-constraints
escalation ladder. Each was tested:
1. **Verify as-is** — not possible (specific Verus limitation for each)
2. **Search vstd** — vstd btree specs gated behind `cfg(all(feature="alloc", feature="std"))`,
   unavailable on no_std target; BTreeMap::get_mut has no vstd spec in any version
3. **Minimal equivalent rewrite** — not applicable (rewrites cannot overcome &mut
   return type or iterator chain limitations)
4. **Stdlib wrapper** — already used where possible (btreemap_remove)

### Challenge details per item

| # | Item | Classification | Challenge Result |
|---|------|---------------|-----------------|
| 1 | ExBTreeMap (vstd_btree.rs:32) | EXTERNAL_TYPE | BTreeMap is external; external_type_specification required |
| 2 | ExCacheGuard (spec.rs:24) | VERUS_LIMITATION | `&'a mut V` field; Verus error: "does not yet support &mut types" |
| 3 | btreemap_remove (lib.rs:114) | STDLIB_WRAPPER | BTreeMap::remove has `Borrow<Q>` generic; vstd's remove spec uses uninterpreted helpers tied to `std::collections::BTreeMap`. Single stdlib call wrapper. |
| 4 | CacheGuard::deref (lib.rs:93) | VERUS_LIMITATION | Cascading: CacheGuard is external_body, field access opaque |
| 5 | Cache::get (lib.rs:190) | VERUS_LIMITATION | Uses `get_mut()`: (a) no vstd spec, (b) returns `Option<&mut V>`. Alternatives would change public API or exec semantics. |
| 6 | Cache::put (lib.rs:230) | VERUS_LIMITATION | Same get_mut blockers as Cache::get |
| 7 | find_lru_victim (lib.rs:315) | VERUS_LIMITATION | Iterator chain `iter().min_by_key(...)`. No vstd specs for `iter()` on no_std; `min_by_key` has no vstd spec. Manual loop also requires unavailable iterator specs. |
| 8 | axiom_cache_lru_of_remove (proof.rs:401) | VERUS_LIMITATION | Axiom on uninterpreted `cache_lru_of`. Cannot be proven without entry iteration specs. |

## Items Remaining in trust.md

### external_body functions (6)

1. **btreemap_remove** — lib.rs:114-123. STDLIB_WRAPPER. Body is single `m.remove(k)` call.
2. **CacheGuard::deref** — lib.rs:93-99. VERUS_LIMITATION. CacheGuard is external_body.
3. **Cache::get** — lib.rs:190-218. VERUS_LIMITATION. get_mut returns &mut.
4. **Cache::put** — lib.rs:230-265. VERUS_LIMITATION. get_mut returns &mut.
5. **find_lru_victim** — lib.rs:315-331. VERUS_LIMITATION. Iterator chain.
6. **axiom_cache_lru_of_remove** — lib.proof.rs:401-411. VERUS_LIMITATION. Uninterpreted function axiom.

### external_type_specification + external_body (2)

7. **ExBTreeMap** — lib.vstd_btree.rs:31-38. EXTERNAL_TYPE.
8. **ExCacheGuard** — lib.spec.rs:22-25. VERUS_LIMITATION (&mut in struct fields).

### assume_specification (5, in lib.vstd_btree.rs)

- BTreeMap::new (line 69-73) — EXTERNAL_BOTTOM
- BTreeMap::len (line 88-95) — EXTERNAL_BOTTOM
- BTreeMap::is_empty (line 98-105) — EXTERNAL_BOTTOM
- BTreeMap::insert (line 108-122) — EXTERNAL_BOTTOM (drops upstream `obeys_cmp_spec` guard)
- BTreeMap::clear (line 130-137) — EXTERNAL_BOTTOM

### Implicitly unverified (1)

- **CacheGuard::deref_mut** — lib.rs:102-105. VERUS_LIMITATION. Returns `&mut V`.

## AST Consistency

- Matched: 15
- Mismatched: 3 (all justified — see below)
- Missing: 0
- Extra: 2 (justified new functions — see below)

### Mismatches

1. **Cache::new** — `Self { ... }` → `let result = Self { ... }; proof!{...} result`.
   **Pre-approved deviation**: "Ensures needs to reference return value" + proof block erased at compile time. Semantics identical.

2. **Cache::remove** — `self.entries.remove(key)` → `btreemap_remove(&mut self.entries, key)` + proof block.
   **VERUS REWRITE** (escalation ladder step 4: stdlib wrapper). Needed because BTreeMap::remove's `Borrow<Q>` generic cannot be expressed with `assume_specification` on `alloc::collections::BTreeMap`. Proof block erased at compile time. Single stdlib call; semantics identical.

3. **Cache::evict** — Inline iterator chain → `Self::find_lru_victim(...)` + `btreemap_remove(...)` + proof block.
   **VERUS REWRITE** (escalation ladder steps 3-4). Iterator chain `iter().min_by_key(...)` is unverifiable (no vstd specs for min_by_key or no_std BTreeMap iter). Extracted into find_lru_victim (external_body) to isolate the unverifiable code. btreemap_remove replaces self.entries.remove for the same Borrow<Q> reason as Cache::remove. Semantics preserved: same victim selection, same key removal.

### Extra functions

1. **Cache::find_lru_victim** — Extracted from evict's iterator chain. Isolates unverifiable code as external_body helper.
2. **btreemap_remove** — Stdlib wrapper for BTreeMap::remove. Fixes Q=K to bypass Borrow<Q> limitation.

## Body-Verified Functions

| Function | Proof Strategy |
|----------|---------------|
| Cache::new | BTreeMap::new assume_spec + lemma_new_view |
| Cache::remove | btreemap_remove wrapper + axiom_cache_lru_of_remove + lemma_remove_view |
| Cache::clear | BTreeMap::clear assume_spec + lemma_clear_view |
| Cache::evict | find_lru_victim external_body + btreemap_remove wrapper + lemma_evict_view |

## vstd Version

- Project uses vstd 0.0.0-2026-04-12-0118
- trust.md previously referenced 0.0.0-2026-04-05-0114 (outdated, updated)
- In both versions, `vstd::std_specs::btree` is gated behind `cfg(all(feature = "alloc", feature = "std"))`
- No get_mut spec in any vstd version

## Result: PASS

All external_body items are genuine trust boundaries that survived challenge.
No admit, assume, trusted, or cfg-gated exec code found. All AST mismatches
are pre-approved deviations or justified VERUS REWRITEs. The verification
passes with 0 errors.
