# Exec Consistency Report

**Source:** `/tmp/ast-check-originals/bitmap_lib_original.rs`
**Verus:** `src/libs/bitmap/src/lib.rs`

## Summary

- Functions matched: 6/11
- Functions mismatched: 5
- Missing in Verus: 0
- Extra in Verus: 1
- **Consistent: NO**

## Inconsistent Functions

| Function | Status | Source Lines | Verus Lines |
|----------|--------|-------------|-------------|
| `Bitmap::alloc_range` [Bitmap__alloc_range.diff](Bitmap__alloc_range.diff) [Bitmap__alloc_range_source.rs](Bitmap__alloc_range_source.rs) [Bitmap__alloc_range_verus_stripped.rs](Bitmap__alloc_range_verus_stripped.rs) | MISMATCH | 161-221 | 271-499 |
| `Bitmap::clear` [Bitmap__clear.diff](Bitmap__clear.diff) [Bitmap__clear_source.rs](Bitmap__clear_source.rs) [Bitmap__clear_verus_stripped.rs](Bitmap__clear_verus_stripped.rs) | MISMATCH | 261-271 | 579-629 |
| `Bitmap::from_raw_array` [Bitmap__from_raw_array.diff](Bitmap__from_raw_array.diff) [Bitmap__from_raw_array_source.rs](Bitmap__from_raw_array_source.rs) [Bitmap__from_raw_array_verus_stripped.rs](Bitmap__from_raw_array_verus_stripped.rs) | MISMATCH | 105-118 | 159-195 |
| `Bitmap::new` [Bitmap__new.diff](Bitmap__new.diff) [Bitmap__new_source.rs](Bitmap__new_source.rs) [Bitmap__new_verus_stripped.rs](Bitmap__new_verus_stripped.rs) | MISMATCH | 63-85 | 99-139 |
| `Bitmap::set` [Bitmap__set.diff](Bitmap__set.diff) [Bitmap__set_source.rs](Bitmap__set_source.rs) [Bitmap__set_verus_stripped.rs](Bitmap__set_verus_stripped.rs) | MISMATCH | 236-246 | 514-564 |
| `ex_error_new` [ex_error_new_verus_stripped.rs](ex_error_new_verus_stripped.rs) | EXTRA_IN_VERUS |  | 45-51 |

## All Functions

| Function | Status | Hash Match | Verification |
|----------|--------|------------|--------------|
| `Bitmap::alloc` | MATCH | ✅ | ✅ verified |
| `Bitmap::alloc_range` | MISMATCH | ❌ | ✅ verified |
| `Bitmap::clear` | MISMATCH | ❌ | ✅ verified |
| `Bitmap::deref` | MATCH | ✅ | ⚠️ UNVERIFIED |
| `Bitmap::from_raw_array` | MISMATCH | ❌ | ✅ verified |
| `Bitmap::index` | MATCH | ✅ | ✅ verified |
| `Bitmap::index_unchecked` | MATCH | ✅ | ✅ verified |
| `Bitmap::new` | MISMATCH | ❌ | ✅ verified |
| `Bitmap::number_of_bits` | MATCH | ✅ | ✅ verified |
| `Bitmap::set` | MISMATCH | ❌ | ✅ verified |
| `Bitmap::test` | MATCH | ✅ | ✅ verified |
| `ex_error_new` [ex_error_new_verus_stripped.rs](ex_error_new_verus_stripped.rs) | EXTRA_IN_VERUS | ❌ | ✅ verified |

## Verification Coverage

**⚠️ 1 function(s) are UNVERIFIED** (outside `verus!` block):

- `Bitmap::deref` (lines 736-738)

These functions are not checked by Verus at all. Justify why each
cannot be verified, or move them inside `verus!` with proper contracts.
