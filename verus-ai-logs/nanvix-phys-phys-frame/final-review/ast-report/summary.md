# AST Consistency Report: ast_orig_b2ee0yup

**Source:** `/tmp/ast_orig_b2ee0yup.rs`
**Verus:** `src/kernel/src/mm/phys/frame.rs`

## Summary

- Functions matched: 10/19
- Functions mismatched: 9
- Missing in Verus: 0
- Extra in Verus: 0
- **Consistent: NO**

## Inconsistent Functions

| Function | Status | Source Lines | Verus Lines |
|----------|--------|-------------|-------------|
| `Inner::alloc` [Inner__alloc.diff](full/Inner__alloc.diff) [src](full/Inner__alloc_source.rs) [verus](full/Inner__alloc_verus.rs) | MISMATCH | 137-165 | 136-211 |
| `Inner::alloc_contiguous` [Inner__alloc_contiguous.diff](full/Inner__alloc_contiguous.diff) [src](full/Inner__alloc_contiguous_source.rs) [verus](full/Inner__alloc_contiguous_verus.rs) | MISMATCH | 210-239 | 255-406 |
| `Inner::alloc_range` [Inner__alloc_range.diff](full/Inner__alloc_range.diff) [src](full/Inner__alloc_range_source.rs) [verus](full/Inner__alloc_range_verus.rs) | MISMATCH | 565-613 | 939-1205 |
| `Inner::book` [Inner__book.diff](full/Inner__book.diff) [src](full/Inner__book_source.rs) [verus](full/Inner__book_verus.rs) | MISMATCH | 481-494 | 806-855 |
| `Inner::free` [Inner__free.diff](full/Inner__free.diff) [src](full/Inner__free_source.rs) [verus](full/Inner__free_verus.rs) | MISMATCH | 290-320 | 456-557 |
| `Inner::is_covered` [Inner__is_covered.diff](full/Inner__is_covered.diff) [src](full/Inner__is_covered_source.rs) [verus](full/Inner__is_covered_verus.rs) | MISMATCH | 517-520 | 877-895 |
| `Inner::refcount` [Inner__refcount.diff](full/Inner__refcount.diff) [src](full/Inner__refcount_source.rs) [verus](full/Inner__refcount_verus.rs) | MISMATCH | 428-444 | 712-770 |
| `Inner::share` [Inner__share.diff](full/Inner__share.diff) [src](full/Inner__share_source.rs) [verus](full/Inner__share_verus.rs) | MISMATCH | 368-395 | 604-680 |
| `free_count` [free_count.diff](full/free_count.diff) [src](full/free_count_source.rs) [verus](full/free_count_verus.rs) | MISMATCH | 723-726 | 1389-1401 |

## Full Diffs (source vs Verus with spec/proof)

Directory: `full/`

| Function | Status | Files |
|----------|--------|-------|
| `Inner::alloc` | MISMATCH | Inner__alloc_source.rs, Inner__alloc_verus.rs, Inner__alloc.diff |
| `Inner::alloc_contiguous` | MISMATCH | Inner__alloc_contiguous_source.rs, Inner__alloc_contiguous_verus.rs, Inner__alloc_contiguous.diff |
| `Inner::alloc_range` | MISMATCH | Inner__alloc_range_source.rs, Inner__alloc_range_verus.rs, Inner__alloc_range.diff |
| `Inner::book` | MISMATCH | Inner__book_source.rs, Inner__book_verus.rs, Inner__book.diff |
| `Inner::free` | MISMATCH | Inner__free_source.rs, Inner__free_verus.rs, Inner__free.diff |
| `Inner::is_covered` | MISMATCH | Inner__is_covered_source.rs, Inner__is_covered_verus.rs, Inner__is_covered.diff |
| `Inner::refcount` | MISMATCH | Inner__refcount_source.rs, Inner__refcount_verus.rs, Inner__refcount.diff |
| `Inner::share` | MISMATCH | Inner__share_source.rs, Inner__share_verus.rs, Inner__share.diff |
| `free_count` | MISMATCH | free_count_source.rs, free_count_verus.rs, free_count.diff |

## Exec-Only Diffs (source vs Verus stripped of ghost/proof)

Directory: `exec-only/`

These diffs show only the executable code differences, with all Verus
annotations (requires/ensures, proof blocks, ghost variables, invariants)
removed. This makes it easier to spot real exec logic changes.

| Function | Status | Files |
|----------|--------|-------|
| `Inner::alloc` | MISMATCH | Inner__alloc_source_stripped.rs, Inner__alloc_verus_stripped.rs, Inner__alloc.diff |
| `Inner::alloc_contiguous` | MISMATCH | Inner__alloc_contiguous_source_stripped.rs, Inner__alloc_contiguous_verus_stripped.rs, Inner__alloc_contiguous.diff |
| `Inner::alloc_range` | MISMATCH | Inner__alloc_range_source_stripped.rs, Inner__alloc_range_verus_stripped.rs, Inner__alloc_range.diff |
| `Inner::book` | MISMATCH | Inner__book_source_stripped.rs, Inner__book_verus_stripped.rs, Inner__book.diff |
| `Inner::free` | MISMATCH | Inner__free_source_stripped.rs, Inner__free_verus_stripped.rs, Inner__free.diff |
| `Inner::is_covered` | MISMATCH | Inner__is_covered_source_stripped.rs, Inner__is_covered_verus_stripped.rs, Inner__is_covered.diff |
| `Inner::refcount` | MISMATCH | Inner__refcount_source_stripped.rs, Inner__refcount_verus_stripped.rs, Inner__refcount.diff |
| `Inner::share` | MISMATCH | Inner__share_source_stripped.rs, Inner__share_verus_stripped.rs, Inner__share.diff |
| `free_count` | MISMATCH | free_count_source_stripped.rs, free_count_verus_stripped.rs, free_count.diff |

## All Functions

| Function | Status | Hash Match | Verification |
|----------|--------|------------|--------------|
| `Inner::alloc` | MISMATCH | ❌ |  |
| `Inner::alloc_contiguous` | MISMATCH | ❌ |  |
| `Inner::alloc_range` | MISMATCH | ❌ |  |
| `Inner::book` | MISMATCH | ❌ |  |
| `Inner::free` | MISMATCH | ❌ |  |
| `Inner::is_covered` | MISMATCH | ❌ |  |
| `Inner::refcount` | MISMATCH | ❌ |  |
| `Inner::share` | MISMATCH | ❌ |  |
| `alloc` | MATCH | ✅ |  |
| `alloc_contiguous` | MATCH | ✅ |  |
| `alloc_range` | MATCH | ✅ |  |
| `book` | MATCH | ✅ |  |
| `free` | MATCH | ✅ |  |
| `free_count` | MISMATCH | ❌ |  |
| `init` | MATCH | ✅ |  |
| `instance` | MATCH | ✅ |  |
| `is_covered` | MATCH | ✅ |  |
| `refcount` | MATCH | ✅ |  |
| `share` | MATCH | ✅ |  |

