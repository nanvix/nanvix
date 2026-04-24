# Cheating Elimination Report: frame

## Cheating Counts (before → after)

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 11 | 11 | 0 |
| assume_specification | 2 | 2 | 0 |
| cfg-gated exec | 4 | 4 | 0 |

**Note on counting:**
- external_body: 10 in frame.rs + 1 in frame.spec.rs (ExFrameNumber)
- assume_specification: 2 in frame.spec.rs (FRAME_SIZE, init)
- cfg-gated exec: 4 diagnostic computation `let` bindings (lines 893, 955, 958, 960);
  7 additional logging macro instances (error!, info!) are allowed per verus-constraints

## Spec Improvement

One spec was tightened during this analysis:

- **`bitmap_index_to_frame_addr`** (frame.rs:76): Added `requires frame_addr_of(index as int) <= usize::MAX as int`.
  The original spec had no precondition but ensured `ret.is_ok()` unconditionally.
  Since the body calls `FrameNumber::from_raw_value(index)` which can return `None`,
  the unconditional `is_ok()` postcondition was stronger than the implementation
  justifies. The caller (`Inner::alloc`) already proves this condition in its proof
  block. This tightening reduces the trusted assumption surface.

## Items Eliminated

None. All cheating items are either external-bottom trust boundaries (wrapping
arch/HAL types and std functions) or proof gaps (singleton pattern using
`static mut` + `MaybeUninit` + `AtomicBool` that Verus cannot handle).

## Trust Boundaries (trust.md)

8 items recorded as external-bottom trust boundaries:

| # | Function | File | Type | Classification |
|---|----------|------|------|----------------|
| 1 | `ExFrameNumber` | frame.spec.rs:10-12 | external_type_specification + external_body | EXTERNAL_TYPE |
| 2 | `FRAME_SIZE` | frame.spec.rs:18-23 | assume_specification | EXTERNAL_CONST |
| 3 | `frame_addr_to_bitmap_index` | frame.rs:63-70 | external_body | STDLIB_WRAPPER |
| 4 | `bitmap_index_to_frame_addr` | frame.rs:74-89 | external_body | STDLIB_WRAPPER |
| 5 | `page_aligned_pa_to_bitmap_index` | frame.rs:93-100 | external_body | STDLIB_WRAPPER |
| 6 | `region_start_frame_number` | frame.rs:104-111 | external_body | STDLIB_WRAPPER |
| 7 | `region_size_raw` | frame.rs:115-121 | external_body | STDLIB_WRAPPER |
| 8 | `region_start_raw` | frame.rs:125-132 | external_body | STDLIB_WRAPPER |

All conversion wrappers (3-8) exist because Verus cannot express
`assume_specification` on generic trait methods (e.g., `Deref::deref` for
`PageAligned<T>`). Each wrapper body is a single expression.

## Verification TODOs (verification_todo.md)

5 items recorded as proof gaps:

| # | Function | File | Type | Blocker |
|---|----------|------|------|---------|
| 1 | `init` | frame.spec.rs:38-42 | assume_specification | MaybeUninit::write(), static mut, unsafe |
| 2 | `pub alloc()` | frame.rs:1184-1195 | external_body | singleton pattern (static mut) |
| 3 | `pub free()` | frame.rs:1199-1213 | external_body | singleton pattern (static mut) |
| 4 | `pub book()` | frame.rs:1217-1227 | external_body | singleton pattern (static mut) |
| 5 | `pub alloc_range()` | frame.rs:1231-1241 | external_body | singleton pattern (static mut) |

All share a common root cause: Verus cannot reason about the singleton
pattern (`static mut` + `MaybeUninit` + `AtomicBool`). The inner methods
(`Inner::alloc/free/book/alloc_range`) are fully body-verified.

## AST Consistency

### Summary
- Zero mismatches confirmed: NO

The AST consistency checker reports 4 MISMATCHes, 6 MISSING_IN_VERUS, and
8 EXTRA_IN_VERUS. All are documented and justified below.

### MISMATCHes (4)

All 4 mismatches are exec code changes required by Verus limitations.
Each is documented with a `VERUS REWRITE` or `VERUS DEVIATION` comment.

#### 1. Inner::alloc — conversion chain → wrapper call

**Original:**
```rust
let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) { ... };
match FrameAddress::from_frame_number(frame_number) { ... }
```

**Verus:**
```rust
let result = bitmap_index_to_frame_addr(index);
result
```

**Evidence:** Verus cannot express `assume_specification` on
`FrameNumber::from_raw_value` or `FrameAddress::from_frame_number` because
they involve generic `PageAligned<T>` construction with `Deref` dispatch.
The wrapper `bitmap_index_to_frame_addr` encapsulates the same conversion
with an external_body spec.

**Semantics preserved:** Same conversion (bitmap index → frame address), same
error behavior (Err propagated). Time/space complexity identical.

#### 2. Inner::free — method chain → wrapper call

**Original:** `frame.into_frame_number().into_raw_value()`
**Verus:** `frame_addr_to_bitmap_index(frame)`

**Evidence:** Generic `Deref::deref` for `PageAligned<T>` cannot have
`assume_specification`. Wrapper isolates the conversion chain.

**Semantics preserved:** Identical computation. Comment at frame.rs:373-377.

#### 3. Inner::book — method chain → wrapper call

**Original:** `phys_addr.into_frame_number().into_raw_value()`
**Verus:** `page_aligned_pa_to_bitmap_index(phys_addr)`

**Evidence:** Same as #2.

**Semantics preserved:** Identical computation.

#### 4. Inner::alloc_range — multiple changes

**4a. Method chains → wrapper calls:**
- `region.start().into_frame_number().into_raw_value()` → `region_start_frame_number(region)`
- `region.size()` → `region_size_raw(region)`
- `region.start().into_raw_value()` → `region_start_raw(region)`

Same justification as #2.

**4b. Inclusive range → exclusive range:**
- Original: `start + size/FRAME_SIZE - 1` with `for i in start..=end`
- Verus: `size/FRAME_SIZE` with `start + num_frames` and `for i in start..end`

**Evidence (reproducer):**
```
$ verus /tmp/test_inclusive_range.rs
error[E0277]: the trait bound `RangeInclusive<usize>: ForLoopGhostIteratorNew`
is not satisfied
```
`RangeInclusive<usize>` does not implement `ForLoopGhostIteratorNew` in Verus.
The exclusive range `Range<usize>` does. The two formulations iterate over
the same set of indices: `[sfn, sfn+1, ..., sfn+n-1]`.

**4c. `continue` → empty body:**
- Original: `Ok(false) => continue,`
- Verus: `Ok(false) => { /* Frame is free — nothing to do. */ },`

This is a trivial syntactic equivalence (empty match arm body = continue in
a for loop).

**4d. cfg-gated diagnostic computations:**
- `index * mem::FRAME_SIZE` → cfg-gated to avoid usize overflow in Verus mode
- `region.start().into_raw_value()` → cfg-gated wrapper call (only for error message)

These computations produce values used exclusively in the immediately-following
cfg-gated `error!()` macro. They are cfg-gated because:
(a) `index * mem::FRAME_SIZE` can overflow usize for large indices
(b) the wrapper calls are only needed for diagnostic output

### MISSING_IN_VERUS (6)

Functions `alloc`, `alloc_range`, `book`, `free`, `init`, `instance` are
reported as missing because:
- `instance()` and `init()` are not annotated with `#[verus_verify]` (they
  use `unsafe`, `static mut`, `MaybeUninit` that Verus cannot parse)
- The pub functions (`alloc`, `free`, `book`, `alloc_range`) have `external_body`,
  which the AST tool considers as "not containing verifiable exec code"

All 6 functions exist in both the original and verus versions with identical
exec code. They are NOT missing — the AST tool's name-matching heuristic
does not find them because they lack verification annotations.

### EXTRA_IN_VERUS (8)

- 6 conversion wrapper functions (`frame_addr_to_bitmap_index`, etc.): New
  functions added to isolate the conversion trust boundary. Justified as
  STDLIB_WRAPPER entries in trust.md.
- `Inner::init` and `Inner::instance`: These are tool artifacts — the free
  functions `init()` and `instance()` are misidentified as `Inner` methods
  by the AST tool's name resolution.

## Verification Results

```
make verify-kernel MODULE=mm::phys::frame
  9 verified, 0 errors

make verify-kernel (full crate)
  22 verified, 0 errors
```

## Result: PASS

All cheating items are classified:
- 8 external-bottom trust boundaries (documented in trust.md)
- 5 proof gaps (documented in verification_todo.md)
- 0 admit / 0 assume
- No eliminable items remain — all are genuine Verus limitations
- Full crate verification passes with 0 errors
