# AST Consistency Report: ast_orig_qe376xq9

**Source:** `/tmp/ast_orig_qe376xq9.rs`
**Verus:** `src/libs/cache/src/lib.rs`

## Summary

- Functions matched: 15/18
- Functions mismatched: 3
- Missing in Verus: 0
- Extra in Verus: 2
- **Consistent: NO**

## Inconsistent Functions

| Function | Status | Source Lines | Verus Lines |
|----------|--------|-------------|-------------|
| `Cache::evict` [Cache__evict.diff](full/Cache__evict.diff) [src](full/Cache__evict_source.rs) [verus](full/Cache__evict_verus.rs) | MISMATCH | 224-233 | 351-360 |
| `Cache::new` [Cache__new.diff](full/Cache__new.diff) [src](full/Cache__new_source.rs) [verus](full/Cache__new_verus.rs) | MISMATCH | 124-130 | 165-175 |
| `Cache::remove` [Cache__remove.diff](full/Cache__remove.diff) [src](full/Cache__remove_source.rs) [verus](full/Cache__remove_verus.rs) | MISMATCH | 205-207 | 283-291 |
| `Cache::find_lru_victim` [verus](full/Cache__find_lru_victim_verus.rs) | EXTRA_IN_VERUS |  | 325-331 |
| `btreemap_remove` [verus](full/btreemap_remove_verus.rs) | EXTRA_IN_VERUS |  | 121-123 |

## Full Diffs (source vs Verus with spec/proof)

Directory: `full/`

| Function | Status | Files |
|----------|--------|-------|
| `Cache::evict` | MISMATCH | Cache__evict_source.rs, Cache__evict_verus.rs, Cache__evict.diff |
| `Cache::new` | MISMATCH | Cache__new_source.rs, Cache__new_verus.rs, Cache__new.diff |
| `Cache::remove` | MISMATCH | Cache__remove_source.rs, Cache__remove_verus.rs, Cache__remove.diff |
| `Cache::find_lru_victim` | EXTRA_IN_VERUS | Cache__find_lru_victim_verus.rs (EXTRA) |
| `btreemap_remove` | EXTRA_IN_VERUS | btreemap_remove_verus.rs (EXTRA) |

## Exec-Only Diffs (source vs Verus stripped of ghost/proof)

Directory: `exec-only/`

These diffs show only the executable code differences, with all Verus
annotations (requires/ensures, proof blocks, ghost variables, invariants)
removed. This makes it easier to spot real exec logic changes.

| Function | Status | Files |
|----------|--------|-------|
| `Cache::evict` | MISMATCH | Cache__evict_source.rs, Cache__evict_verus_stripped.rs, Cache__evict.diff |
| `Cache::new` | MISMATCH | Cache__new_source.rs, Cache__new_verus_stripped.rs, Cache__new.diff |
| `Cache::remove` | MISMATCH | Cache__remove_source.rs, Cache__remove_verus_stripped.rs, Cache__remove.diff |
| `Cache::find_lru_victim` | EXTRA_IN_VERUS | Cache__find_lru_victim_verus.rs (EXTRA) |
| `btreemap_remove` | EXTRA_IN_VERUS | btreemap_remove_verus.rs (EXTRA) |

## All Functions

| Function | Status | Hash Match | Verification |
|----------|--------|------------|--------------|
| `Cache::clear` | MATCH | ✅ |  |
| `Cache::evict` | MISMATCH | ❌ |  |
| `Cache::get` | MATCH | ✅ |  |
| `Cache::new` | MISMATCH | ❌ |  |
| `Cache::put` | MATCH | ✅ |  |
| `Cache::remove` | MISMATCH | ❌ |  |
| `CacheGuard::deref` | MATCH | ✅ |  |
| `CacheGuard::deref_mut` | MATCH | ✅ |  |
| `capacity_one` | MATCH | ✅ |  |
| `clear_removes_all_entries` | MATCH | ✅ |  |
| `evicts_lru_entry_when_full` | MATCH | ✅ |  |
| `get_refreshes_lru_order` | MATCH | ✅ |  |
| `get_returns_none_on_miss` | MATCH | ✅ |  |
| `overwrite_does_not_evict` | MATCH | ✅ |  |
| `put_overwrites_existing_key` | MATCH | ✅ |  |
| `put_then_get` | MATCH | ✅ |  |
| `remove_deletes_key` | MATCH | ✅ |  |
| `remove_nonexistent_key_is_noop` | MATCH | ✅ |  |
| `Cache::find_lru_victim` | EXTRA_IN_VERUS | ❌ |  |
| `btreemap_remove` | EXTRA_IN_VERUS | ❌ |  |

