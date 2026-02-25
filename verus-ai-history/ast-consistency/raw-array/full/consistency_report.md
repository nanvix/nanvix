# Exec Consistency Report

**Source:** `/tmp/ast-check-originals/raw_array_lib_original.rs`
**Verus:** `src/libs/raw-array/src/lib.rs`

## Summary

- Functions matched: 9/9
- Functions mismatched: 0
- Missing in Verus: 0
- Extra in Verus: 1
- **Consistent: YES**

## Inconsistent Functions

| Function | Status | Source Lines | Verus Lines |
|----------|--------|-------------|-------------|
| `RawArray::set` [RawArray__set_verus.rs](RawArray__set_verus.rs) | EXTRA_IN_VERUS |  | 320-330 |

## All Functions

| Function | Status | Hash Match | Verification |
|----------|--------|------------|--------------|
| `RawArray::deref` | MATCH | ✅ | 🔒 external_body |
| `RawArray::deref_mut` | MATCH | ✅ | ⚠️ UNVERIFIED |
| `RawArray::drop` | MATCH | ✅ | ⚠️ UNVERIFIED |
| `RawArray::from_raw_parts` | MATCH | ✅ | 🔒 external_body |
| `RawArray::new` | MATCH | ✅ | 🔒 external_body |
| `RawArrayStorage::get` | MATCH | ✅ | ⚠️ UNVERIFIED |
| `RawArrayStorage::get_mut` | MATCH | ✅ | ⚠️ UNVERIFIED |
| `RawArrayStorage::new_managed` | MATCH | ✅ | ⚠️ UNVERIFIED |
| `RawArrayStorage::new_unmanaged` | MATCH | ✅ | ⚠️ UNVERIFIED |
| `RawArray::set` [RawArray__set_verus.rs](RawArray__set_verus.rs) | EXTRA_IN_VERUS | ❌ | 🔒 external_body |

## Verification Coverage

**⚠️ 6 function(s) are UNVERIFIED** (outside `verus!` block):

- `RawArray::deref_mut` (lines 353-355)
- `RawArray::drop` (lines 359-372)
- `RawArrayStorage::get` (lines 186-195)
- `RawArrayStorage::get_mut` (lines 166-175)
- `RawArrayStorage::new_managed` (lines 83-109)
- `RawArrayStorage::new_unmanaged` (lines 134-155)

These functions are not checked by Verus at all. Justify why each
cannot be verified, or move them inside `verus!` with proper contracts.

**🔒 4 function(s) use `external_body`** (body not verified):

- `RawArray::deref` (lines 341-346)
- `RawArray::from_raw_parts` (lines 293-308)
- `RawArray::new` (lines 252-267)
- `RawArray::set` (lines 320-330)

These functions have requires/ensures contracts but the body is trusted.
Justify why `external_body` is necessary for each.

## Inconsistent Structs

| Struct | Status | Source Lines | Verus Lines |
|--------|--------|-------------|-------------|
| `ExError` | EXTRA_IN_VERUS |  | 214-214 |
| `ExErrorCode` | EXTRA_IN_VERUS |  | 218-218 |
| `ExRawArrayStorage` | EXTRA_IN_VERUS |  | 209-209 |
