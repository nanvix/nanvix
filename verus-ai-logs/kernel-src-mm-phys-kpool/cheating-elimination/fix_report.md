# Cheating Elimination Report: kpool

## Cheating Counts (before → after)

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 4 | 0 | 4 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 1* | 1* | 0 |

\* The 1 cfg-gated item is a **false positive**: `#[cfg(verus_keep_ghost)] verus! { impl View for KernelFrame ... }` (line 807). This is spec-only code (a `View` trait impl), not gated exec code. It existed identically in the base branch (`verus-ai/upool`, line 435). All 10 `#[cfg(not(verus_keep_ghost))]` items are on logging macros (`error!`, `info!`), which are explicitly allowed per verus-constraints.

## Items Eliminated

All 4 `external_body` markers were eliminated by the proving phase (prior to this audit). The bodies of all four `Inner` methods are now fully verified:

1. **`Inner::new`** (`kpool.rs:96`): Was `#[verus_verify(external_body)]`. Now body-verified with proof blocks establishing `internal_inv` from `is_valid_physical_region` postconditions and arithmetic lemmas.

2. **`Inner::alloc`** (`kpool.rs:166`): Was `#[verus_verify(external_body)]`. Now body-verified with proof blocks relating bitmap index to page address via multiplication/division lemmas from `vstd::arithmetic`.

3. **`Inner::alloc_range`** (`kpool.rs:295`): Was `#[verus_verify(external_body)]`. Now body-verified with a loop invariant proving contiguous frame construction, plus proof blocks establishing the union of new page indices.

4. **`Inner::free`** (`kpool.rs:592`): Was `#[verus_verify(external_body)]`. Now body-verified with proof blocks establishing page_index validity and relating bitmap clear to `used_page_indices.remove`.

## Trust Boundaries (trust.md)

- **`pa_into_raw`** (`hal/mem/types/address/frame.rs:94`): `external_body` STDLIB_WRAPPER. A single-line wrapper for `PageAligned<PhysicalAddress>::into_raw_value()` needed because Verus cannot resolve the generic trait method chain. This is a dependency trust boundary, not kpool-specific. Spec: `ensures ret as int == pa@`.

No kpool-specific trust boundaries remain.

## Verification TODOs (verification_todo.md)

None. All proof gaps have been resolved.

## AST Consistency

4 functions report MISMATCH. All are justified:

### 1. `Inner::new` — Two exec changes

- **`base.into_raw_value()` → `pa_into_raw(base)`**: Stdlib wrapper pattern (escalation ladder step 4). Verus cannot resolve `.into_raw_value()` on `PageAligned<PhysicalAddress>`. Semantics identical; `pa_into_raw` body is `pa.into_raw_value()`.
- **`Ok(Inner { base, bitmap })` → `let inner = Inner { base, bitmap }; Ok(inner)`**: Pre-approved deviation (intermediate value for ensures reference).

### 2. `Inner::alloc` — Two exec changes

- **`self.base.into_raw_value()` → `pa_into_raw(self.base)`**: Same stdlib wrapper as above.
- **`FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(addr)?)?))` → `FrameAddress::from_raw_value(addr)?`**: Equivalent rewrite. `FrameAddress::from_raw_value` (frame.rs:123) body is `Ok(Self(PageAligned::from_address(PhysicalAddress::from_raw_value(raw_addr)?)?))` — identical to the original inline chain. Same semantics, same error behavior.

### 3. `Inner::alloc_range` — Three exec changes

- **`self.base.into_raw_value()` → `pa_into_raw(self.base)`**: Same stdlib wrapper.
- **`FrameAddress::new(...)` → `FrameAddress::from_raw_value(addr)?`**: Same equivalent rewrite as `alloc`.
- **Added `count == 0` and `count > num_pages` guards** (lines 310–329): VERUS REWRITE. `Bitmap::alloc_range` has `requires size > 0, size <= num_bits`. Without guards, calling it with invalid `count` would violate the precondition. Both guards return `Err(InvalidArgument)` — the original would also error (from bitmap), just with a different error code. Semantics preserved (error on invalid input).

### 4. `Inner::free` — Two exec changes

- **`self.base.into_raw_value()` → `pa_into_raw(self.base)`**: Same stdlib wrapper.
- **Added underflow guard** `if addr.into_raw_value() < pa_into_raw(self.base)` (line 597): VERUS REWRITE. The original `addr.into_raw_value() - self.base.into_raw_value()` would cause usize underflow when `addr < base` (wrapping in release, panic in debug). The guard returns `Err(BadAddress)` instead. This is also a correctness improvement — it prevents undefined/wrapping behavior.

Zero mismatches confirmed: **NO** (4 justified deviations remain — all documented with VERUS REWRITE comments or matching pre-approved patterns)

## Result: PASS

All cheating items in kpool have been eliminated:
- 4 `external_body` markers removed, bodies fully verified
- 0 admit, 0 assume, 0 assume_specification
- cfg-gated items are all on allowed patterns (logging macros, spec-only View impl)
- AST mismatches are justified (stdlib wrappers, pre-approved deviations, documented VERUS REWRITEs)
- Full crate verification passes with 0 errors
