# Exec Fidelity Report: kheap

## AST Consistency Results
- Matched: 6 (3 functions: `ArenaAllocator::alloc`, `ArenaAllocator::dealloc`, `init`; 3 structs: `ArenaAllocator`, `HeapStorage`, `Kheap`)
- Mismatched: 4 (listed below with ast_diff output)
- Missing: 0
- Extra: 0

### Mismatch 1: `Kheap::layout_to_allocator`

**Change**: Named return `-> Result<SlabSize, AllocError>` → `-> (result: Result<SlabSize, AllocError>)`

**Classification**: Pre-approved deviation — `-> T` → `-> (ret: T)` for `ensures` clauses.

**Function body**: Identical. No exec code changes.

### Mismatch 2: `Kheap::from_raw_parts`

**Changes** (all legitimate):

1. **Named return**: `-> Result<Kheap, Error>` → `-> (result: Result<Kheap, Error>)` — Pre-approved deviation.

2. **`mem::PAGE_SIZE` cfg-gated**: Original `mem::PAGE_SIZE` preserved under `#[cfg(not(verus_keep_ghost))]`; Verus-compatible `PAGE_SIZE` (constant duplicated inside `verus!{}`) added under `#[cfg(verus_keep_ghost)]`. Verus cannot resolve module-path constants defined outside `verus!{}` blocks.

3. **`addr as *mut u8` cfg-gated**: Original cast preserved under `#[cfg(not(verus_keep_ghost))]`; Verus-compatible `usize_to_mut_ptr(addr)` added under `#[cfg(verus_keep_ghost)]`. Verus lacks support for `usize`-to-pointer casts.

4. **`info!()` macros cfg-gated**: `#[cfg(not(verus_keep_ghost))]` added to two `info!()` calls. Verus cannot parse format macros.

**Classification**: All changes use cfg-gating to preserve the original exec code path in non-Verus builds while providing Verus-compatible alternatives. This is the prescribed approach per verus-constraints.

### Mismatch 3: `Kheap::allocate`

**Changes**:

1. **Named return**: Pre-approved deviation.

2. **`|_|` → `|_e|` in closure params** (10 occurrences): Verus does not support `_` (wildcard) patterns in closure parameters — it requires named variables. The change is semantically identical (both ignore the argument). This is a confirmed Verus limitation documented with `VERUS DEVIATION` comment.

**Classification**: Named return is pre-approved. Closure param rename is a documented Verus limitation — minimal change, semantically identical, not cfg-gatable at expression level.

### Mismatch 4: `Kheap::deallocate`

**Changes**: Identical pattern to `Kheap::allocate` — named return (pre-approved) and `|_|` → `|_e|` (10 occurrences, documented Verus limitation).

## Fixes Applied

None required. All 4 mismatches fall into one of:
- **Pre-approved deviation**: Named returns `-> (ret: T)` for ensures clauses (all 4 functions)
- **Legitimate cfg-gating**: `mem::PAGE_SIZE`, `addr as *mut u8`, `info!()` — original exec preserved under `#[cfg(not(verus_keep_ghost))]` (`from_raw_parts`)
- **Documented Verus limitation**: `|_|` → `|_e|` in closures — Verus requires named closure params (`allocate`, `deallocate`)

No accidental exec code modifications were found.

## Verification: PASS

```
make verify-kernel MODULE=mm::kheap → 19 verified, 0 errors
make verify (all crates)            → 19 verified, 0 errors (kernel) + 35 verified (slab) + 0 errors (bitmap)
```
