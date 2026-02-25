# Verus Bitmap Verification Integration Report

## Overview

Successfully integrated Verus formal verification for the `bitmap` crate from the
monolithic verus split (`~/nanvix/verus/split/libs/bitmap/`) into the nanvix-dev project
(`~/nanvix-dev/src/libs/bitmap/`). The `raw-array` dependency was also augmented with
Verus specifications to support bitmap's verification.

## Verification Results

```
$ make verify
verification results:: 11 verified, 0 errors   (raw-array)
verification results:: 71 verified, 0 errors   (bitmap)
```

**Total: 82 properties verified, 0 errors.**

## Changes Summary

### Files Modified

| File | Lines Changed | Description |
|------|--------------|-------------|
| `src/libs/bitmap/src/lib.rs` | +511 / -66 | Replaced exec code with Verus-annotated version |
| `src/libs/raw-array/Cargo.toml` | +4 | Added `vstd` dependency, `[package.metadata.verus]` |
| `src/libs/raw-array/src/lib.rs` | +102 / -28 | Added Verus annotations, `set()` method, external type specs |

### Files Added

| File | Lines | Description |
|------|-------|-------------|
| `src/libs/bitmap/src/lib.spec.rs` | 147 | Bitmap spec: `BitmapView`, `View` trait, invariants, helper specs |
| `src/libs/bitmap/src/lib.proof.rs` | 1157 | Bitmap proofs: 52 lemmas for correctness of alloc, set, clear, etc. |
| `src/libs/raw-array/src/lib.spec.rs` | 112 | RawArray spec: `View` trait, `is_zero`, `RawArrayView`, invariants |
| `src/libs/raw-array/src/lib.proof.rs` | 131 | RawArray proofs: lemmas for update, equality, frame conditions |

### Cargo.toml Version Fix

- `Cargo.toml`: vstd version pinned to `=0.0.0-2026-02-22-0103` (was `0.0.0-2026-02-08-0120`)
  to match the installed Verus binary (v0.2026.02.22). `Cargo.lock` regenerated.

## Key Integration Decisions

### 1. Error Crate: No Modification Required

The `error` crate was **not** modified. Instead:
- `raw-array` provides `#[verifier::external_type_specification]` for `Error` and `ErrorCode`.
- `bitmap` provides `#[verifier::external_fn_specification]` for `Error::new()` with
  ensures contracts (`result.code == code, result.reason == reason`).

**Justification:** The error crate has a `rustc-dep-of-std` feature which makes adding
`vstd` as a dependency problematic. Using external specifications avoids modifying error
while providing all necessary verification contracts.

### 2. RawArray: Verus Metadata Required

Added `[package.metadata.verus] verify = true` to raw-array's Cargo.toml. This causes the
Verus compiler to set `verus_keep_ghost` for raw-array when verifying bitmap, which is
required for cross-crate spec visibility.

### 3. Bitmap Exec Code Differences from Original

The integrated `lib.rs` has these **necessary** differences from the original exec code:

| Change | Reason |
|--------|--------|
| `self.bits.set(w, self.bits[w] \| (1u8 << b))` instead of `self.bits[w] \|= 1 << b` | Verus does not support mutable indexing |
| `self.usage = self.usage + size` instead of `self.usage += size` | Verus does not support compound assignment on struct fields |
| `#[cfg(not(verus_keep_ghost))] debug_assert_eq!(...)` | Verus does not support `debug_assert_eq!` macro |
| `from_raw_array`: manual overflow check instead of `checked_mul` + closure | Verus does not support `checked_mul` or closures |
| `for` loops replaced with `while` loops with invariants | Verus does not support `for` range loops |
| `#[cfg_attr(not(verus_keep_ghost), derive(Debug))]` | Verus cannot derive `Debug` |
| Added `ex_error_new` external fn specification | Provides Verus ensures contract for `Error::new()` |

All changes are semantically equivalent to the original code.

### 4. Verified Properties (bitmap, 71 items)

The following properties are formally verified for the bitmap allocator:
- **Invariant preservation**: All public methods preserve `bitmap.inv()`.
- **Allocation correctness**: `alloc`/`alloc_range` return indices within bounds.
- **Frame conditions**: Operations only modify the targeted bit(s); all other bits unchanged.
- **Set-based reasoning**: Uses `Set<int>` to track which bits are set, with union/insert/remove.
- **Liveness**: If free bits exist, `alloc` succeeds.
- **Usage tracking**: `usage` count always equals `set_bits.len()`.
- **Zero-initialization**: New bitmaps have all bits unset.

### 5. Verified Properties (raw-array, 11 items)

- View equality lemmas (reflexive, symmetric, transitive).
- Update frame conditions (only changed index is modified).
- Length preservation across updates.
- Invariant validation.
