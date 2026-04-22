# AST Consistency Report: ast_orig_g0os8_a8

**Source:** `/tmp/ast_orig_g0os8_a8.rs`
**Verus:** `src/libs/cache/src/lib.rs`

## Summary

- Functions matched: 18/18
- Functions mismatched: 0
- Missing in Verus: 0
- Extra in Verus: 0
- **Consistent: YES**

## All Functions

| Function | Status | Hash Match | Verification |
|----------|--------|------------|--------------|
| `Cache::clear` | MATCH | ✅ |  |
| `Cache::evict` | MATCH | ✅ |  |
| `Cache::get` | MATCH | ✅ |  |
| `Cache::new` | MATCH | ✅ |  |
| `Cache::put` | MATCH | ✅ |  |
| `Cache::remove` | MATCH | ✅ |  |
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

