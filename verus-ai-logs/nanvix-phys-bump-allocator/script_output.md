# Caller Analysis (LSP): lib.rs

- **Source file:** `/home/ruize/nanvix-phy/src/libs/bump_allocator/src/lib.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP
- **Crate:** `bump-allocator`
- **Depended on by:** `kernel`

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 12 |
| Public / trait-pub | 9 |
| Private | 3 |
| Types | 8 |

## Public API — External Callers

### `new` (impl `FixedSizeBumpAllocator<N, A, S>`) [pub] — 1 external caller(s)
```
pub const unsafe fn new() -> Self
```
> 
# Description

Creates a new fixed-size bump allocator.

# Returns

Returns a new allocator.

# Safety

The caller must ensure that only **one** `FixedSizeBumpAllocator` instance exists for a
given `S: BssStorage` backend at any time. Creating multiple allocators over the same
backend causes independent bump counters, which leads to overlapping slot reservations
and undefined behavior (multiple `&'static mut` references to the same memory).



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/page_table_allocator.rs` | 103 | `> = unsafe { FixedSizeBumpAllocator::new() };` |

*Internal callers (4):*
- **FixedSizeBumpAllocator<N, A, S>::default** (L327): `unsafe { Self::new() }`
- ? (L365): `unsafe { FixedSizeBumpAllocator::new() };`
- ? (L393): `unsafe { FixedSizeBumpAllocator::new() };`
- ? (L424): `unsafe { FixedSizeBumpAllocator::new() };`

### `alloc_as` (impl `FixedSizeBumpAllocator<N, A, S>`) [pub] — 4 external caller(s)
```
pub unsafe fn alloc_as<T>(&self) -> Result<&'static mut MaybeUninit<T>, BumpAllocError>
```
> 
# Description

Allocates the next fixed-size slot and reinterprets it as `MaybeUninit<T>`.

# Returns

On success, returns a mutable reference to an uninitialized `T`.

# Errors

- [`BumpAllocError::SizeMismatch`] if `size_of::<T>() != N`.
- [`BumpAllocError::AlignmentMismatch`] if `align_of::<T>() > A`.
- Any error from [`alloc()`](Self::alloc).

# Safety

The caller must initialise the returned `MaybeUninit<T>` before reading through it
and ensure exclusive use of the returned reference.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/vmem.rs` | 111 | `.alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()` |
| `src/kernel/src/mm/virt/identity_map.rs` | 518 | `.alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()` |
| `src/kernel/src/mm/virt/boot_init.rs` | 129 | `.alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()` |
| `src/kernel/src/mm/virt/boot_init.rs` | 161 | `.alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()` |

*Internal callers (1):*
- **alloc_as_allows_typed_access** (L398): `unsafe { ALLOC_B.alloc_as::<[u32; 2]>().expect("alloc_as failed") };`

### `alloc` (impl `FixedSizeBumpAllocator<N, A, S>`) [pub] — **0 external callers**
```
pub fn alloc(&self) -> Result<&'static mut [u8; N], BumpAllocError>
```
> 
# Description

Allocates the next fixed-size slot as a raw byte array.

# Returns

On success, returns a mutable reference to a unit-sized byte array.

# Errors

- [`BumpAllocError::Exhausted`] if storage capacity is exhausted.
- [`BumpAllocError::Overflow`] if internal arithmetic overflows.
- [`BumpAllocError::OutOfBounds`] if computed slot exceeds storage bounds.
- [`BumpAllocError::Misaligned`] if computed slot is not properly aligned.


*Internal callers (5):*
- **FixedSizeBumpAllocator<N, A, S>::alloc_as** (L318): `let slot: &'static mut [u8; N] = self.alloc()?;`
- **alloc_returns_distinct_slots** (L369): `let first: *mut u8 = ALLOC_A.alloc().expect("first alloc failed").as_mut_ptr();`
- **alloc_returns_distinct_slots** (L370): `let second: *mut u8 = ALLOC_A.alloc().expect("second alloc failed").as_mut_ptr()`
- **alloc_returns_exhausted_error** (L428): `let _ = ALLOC_C.alloc().expect("first alloc failed");`
- **alloc_returns_exhausted_error** (L429): `let result = ALLOC_C.alloc();`

### `as_mut_ptr` (trait `BssStorage` for `BackendB`) [trait-pub] — **0 external callers**
```
fn as_mut_ptr() -> *mut u8
```

### `default` (trait `Default` for `FixedSizeBumpAllocator<N, A, S>`) [trait-pub] — **0 external callers**
```
fn default() -> Self
```

### `as_mut_ptr` (trait `BssStorage` for `BackendA`) [trait-pub] — **0 external callers**
```
fn as_mut_ptr() -> *mut u8
```

### `as_mut_ptr` (trait `BssStorage` for `BackendC`) [trait-pub] — **0 external callers**
```
fn as_mut_ptr() -> *mut u8
```

### `fmt` (trait `fmt::Display` for `BumpAllocError`) [trait-pub] — **0 external callers**
```
fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
```

### `align_up` [pub] — 2 external caller(s)
```
pub const fn align_up(value: usize, alignment: usize) -> Option<usize>
```
> 
# Description

Aligns `value` up to the next multiple of `alignment`.

# Parameters

- `value`: Value to align.
- `alignment`: Alignment boundary.

# Returns

Returns the aligned value, or `None` if `alignment` is zero or the computation overflows.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/page_table_allocator.rs` | 24 | `align_up,` |
| `src/kernel/src/mm/virt/page_table_allocator.rs` | 49 | `const PAGE_TABLE_SLOT_STRIDE: usize = match align_up(PAGE_TABLE_SLOT_SIZE, PAGE_` |

*Internal callers (1):*
- **FixedSizeBumpAllocator<N, A, S>::alloc** (L269): `let stride: usize = align_up(N, A).ok_or(BumpAllocError::Overflow)?;`

## Private Functions — Internal Call Graph

These are implementation details. Listed to show which public functions depend on them.

### `alloc_returns_distinct_slots` [private]
```
fn alloc_returns_distinct_slots()
```
*No internal callers found (may be called via macro, closure, or conditional compilation).*

### `alloc_as_allows_typed_access` [private]
```
fn alloc_as_allows_typed_access()
```
*No internal callers found (may be called via macro, closure, or conditional compilation).*

### `alloc_returns_exhausted_error` [private]
```
fn alloc_returns_exhausted_error()
```
*No internal callers found (may be called via macro, closure, or conditional compilation).*

## Type References

### `FixedSizeBumpAllocator` [pub] — 3 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/page_table_allocator.rs` | 26 | `FixedSizeBumpAllocator,` |
| `src/kernel/src/mm/virt/page_table_allocator.rs` | 99 | `pub static PAGE_TABLE_ALLOCATOR: FixedSizeBumpAllocator<` |
| `src/kernel/src/mm/virt/page_table_allocator.rs` | 103 | `> = unsafe { FixedSizeBumpAllocator::new() };` |

### `BackendA` [private] — 0 external reference(s)

### `BackendB` [private] — 0 external reference(s)

### `BackendC` [private] — 0 external reference(s)

### `StorageB` [private] — 0 external reference(s)

### `StorageA` [private] — 0 external reference(s)

### `StorageC` [private] — 0 external reference(s)

### `BumpAllocError` [pub] — 0 external reference(s)

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `alloc`
- `as_mut_ptr`
- `default`
- `as_mut_ptr`
- `as_mut_ptr`
- `fmt`
