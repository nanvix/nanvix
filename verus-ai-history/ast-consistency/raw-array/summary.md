# Exec Diff: raw_array_lib_original

**Source:** `/tmp/ast-check-originals/raw_array_lib_original.rs`
**Verus:** `src/libs/raw-array/src/lib.rs`

## Full Diffs (source vs Verus with spec/proof)

Directory: `full/`

| Function | Status | Files |
|----------|--------|-------|
| `RawArray::set` | EXTRA_IN_VERUS | RawArray__set_verus.rs (EXTRA) |

## Exec-Only Diffs (source vs Verus stripped of ghost/proof)

Directory: `exec-only/`

These diffs show only the executable code differences, with all Verus
annotations (requires/ensures, proof blocks, ghost variables, invariants)
removed. This makes it easier to spot real exec logic changes.

| Function | Status | Files |
|----------|--------|-------|
| `RawArray::set` | EXTRA_IN_VERUS | RawArray__set_verus.rs (EXTRA) |

## Struct Issues

| Struct | Status | Files |
|--------|--------|-------|
| `ExError` | EXTRA_IN_VERUS | struct_ExError_verus.rs (EXTRA) |
| `ExErrorCode` | EXTRA_IN_VERUS | struct_ExErrorCode_verus.rs (EXTRA) |
| `ExRawArrayStorage` | EXTRA_IN_VERUS | struct_ExRawArrayStorage_verus.rs (EXTRA) |
