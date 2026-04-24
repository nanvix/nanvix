# Caller Analysis: `mm::phys::frame`

**Source**: `src/kernel/src/mm/phys/frame.rs`
**Visibility**: All public functions are `pub(super)` — callers are confined to the `mm::phys` module.
**Module type**: Module-level singleton (no handle passed around; free functions over static state).

## Script Output

The automated `find_callers_lsp.py` script failed due to a rust-analyzer daemon
connection error. Analysis was performed manually using exhaustive `grep` over
the kernel crate.

## Module Summary

| Category | Count |
|----------|-------|
| Total functions | 6 (4 on `Inner`, 1 private helper `instance()`, 5 public free functions) |
| `pub(super)` free functions | 5: `init`, `alloc`, `free`, `book`, `alloc_range` |
| Private | 5: `Inner::alloc`, `Inner::free`, `Inner::book`, `Inner::alloc_range`, `instance()` |
| External callers (outside `mm::phys`) | 0 |

## Public API and Callers

### `unsafe fn init(bitmap: SparseBitmap) -> Result<(), Error>` — `pub(super)`

**Callers:**
| File | Line | Context |
|------|------|---------|
| `mm/phys/mod.rs` | 138 | `unsafe { frame::init(physical_memory_layout)? }` — called inside `phys::init()` |

**Caller expectations:**
- Called **exactly once** during single-threaded boot, before any other `frame::` function.
- After success, the frame allocator singleton is ready for use.
- The passed `SparseBitmap` defines the full physical memory layout; all frames start as free.
- Double-init returns an error (enforced by `INSTANCE_INIT` guard).

---

### `fn alloc() -> Result<FrameAddress, Error>` — `pub(super)`

**Callers:**
| File | Line | Context |
|------|------|---------|
| `mm/phys/upool.rs` | 165 | `let addr: FrameAddress = frame::alloc()?;` — inside `Upool::alloc()` |
| `mm/phys/test.rs` | 58 | `match frame::alloc()` — test: batch allocation rounds |
| `mm/phys/test.rs` | 88 | `match frame::alloc()` — test: leak-prevents-drop |
| `mm/phys/test.rs` | 115 | `match frame::alloc()` — test: drop-frees-frame |

**Caller expectations:**
- On success: returns a valid `FrameAddress` that was previously free.
- The returned frame is now **owned** by the caller — the allocator will not hand it out again until freed.
- On failure (`Err`): no state change; the pool is exhausted (`OutOfMemory`).
- The returned `FrameAddress` satisfies `inv()` (page-aligned, valid physical address).
- `Upool::alloc` wraps the result in a `UserFrame` (RAII wrapper with `Drop` that calls `frame::free`).

---

### `fn free(frame: FrameAddress) -> Result<(), Error>` — `pub(super)`

**Callers:**
| File | Line | Context |
|------|------|---------|
| `mm/phys/upool.rs` | 107 | `frame::free(self.addr)` — inside `UserFrame::Drop::drop()` |
| `mm/phys/test.rs` | 99 | `frame::free(leaked_addr)` — test: manually free a leaked frame |
| `mm/phys/test.rs` | 128 | `frame::free(addr)` — test: double-free detection |

**Caller expectations:**
- On success: the frame was allocated (owned) and is now returned to the free pool.
- On failure: the frame was **not** currently allocated (double-free or invalid); no state change.
- `UserFrame::Drop` is the primary caller — it expects that freeing a previously allocated frame always succeeds. Errors are logged but cannot be propagated (inside `drop`).
- Tests rely on free-after-alloc succeeding, and double-free (free-after-drop) failing.
- Declared with `no_unwind` / `opens_invariants none` — required by Verus for `Drop` compatibility.

---

### `fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>` — `pub(super)`

**Callers:**
| File | Line | Context |
|------|------|---------|
| `mm/phys/mod.rs` | 93 | `frame::book(phys_addr)` — inside `book_mmio_regions()` |

**Caller expectations:**
- Marks a single frame as allocated so `alloc()` will never hand it out.
- Used during boot to reserve MMIO-mapped frames.
- On success: the frame was free and is now reserved.
- On failure with `ErrorCode::InvalidArgument`: the frame lies outside addressable physical memory — **caller silently ignores** this (line 97: `Err(e) if e.code == ErrorCode::InvalidArgument => {}`).
- Other errors propagate upward.

---

### `fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error>` — `pub(super)`

**Callers:**
| File | Line | Context |
|------|------|---------|
| `mm/phys/mod.rs` | 70 | `frame::alloc_range(region)?` — inside `book_physical_memory_regions()` |

**Caller expectations:**
- Atomically reserves **all** frames in a contiguous physical memory region.
- Used during boot to mark non-usable physical memory regions as allocated.
- On success: every frame in the region was free and is now reserved.
- On failure: **no partial reservation** — state is unchanged (the spec ensures `self@ == old(self)@` on error).
- Region parameter satisfies `inv()` (page-aligned start, valid size).

## Implicit Callers / Trait Obligations

### `Drop for UserFrame` (in `upool.rs:100-113`)

- The `Drop` impl calls `frame::free(self.addr)`.
- This is runtime-dispatched whenever a `UserFrame` goes out of scope.
- **Semantics**: dropping a `UserFrame` must return the frame to the free pool.
- Drop errors are logged but swallowed (cannot propagate from `drop`).

## Internal Call Graph

All public free functions delegate to the `Inner` singleton via `instance()`:

```
alloc()       → instance().alloc()
free(frame)   → instance().free(frame)
book(addr)    → instance().book(addr)
alloc_range() → instance().alloc_range()
```

`instance()` returns `&'static mut Inner` after checking `INSTANCE_INIT`.
Panics if called before `init()`.

`Inner` methods delegate to `SparseBitmap` operations:
- `alloc` → `bitmap.alloc()` → `FrameNumber::from_raw_value()` → `FrameAddress::from_frame_number()`
- `free`  → `bitmap.clear(frame_number)`
- `book`  → `bitmap.set(frame_number)`
- `alloc_range` → loop of `bitmap.test(i)` then `bitmap.set(i)`

## Type References

| Type | Used by | Role |
|------|---------|------|
| `FrameAddress` | `upool.rs`, `kpool.rs`, `kpage.rs`, `vmem.rs`, etc. | Opaque handle for an allocated frame |
| `PageAligned<PhysicalAddress>` | `mod.rs` (book_mmio_regions) | Input to `book()` |
| `TruncatedMemoryRegion<PhysicalAddress>` | `mod.rs` (book_physical_memory_regions) | Input to `alloc_range()` |
| `SparseBitmap` | Only `frame.rs` internally | Backing allocator (not exposed to callers) |

## Pre-existing Specs (from upstream verification)

- **Source**: specs exist directly on `Inner` methods in `frame.rs` and on free functions.
- **View type**: `UpoolView` (defined in `mod.spec.rs:50-64`), used as `Inner`'s `View` type.
- **Functions with specs**:
  - `Inner::alloc` — full alloc/error spec referencing `UpoolView`
  - `Inner::free` — full free/error spec referencing `UpoolView`
  - `Inner::book` — full book/error spec referencing `UpoolView`
  - `Inner::alloc_range` — full range-book spec referencing `UpoolView`
  - `alloc()` (free fn) — partial spec (only ensures `frame.inv()` on success)
  - `free()` (free fn) — uses `verus!{}` syntax with `external_body`; no ensures beyond `no_unwind`
- **Functions WITHOUT specs**: `init()`, `book()` (free fn), `alloc_range()` (free fn), `instance()`

### Assessment

- **Coverage**: Partial — `Inner` methods have rich specs but the public free functions
  have weak or missing specs. The free functions are thin wrappers, so the `Inner` specs
  capture most of the logic, but the public API layer lacks forwarding specs.
- **Strength**: `Inner` specs are strong (track allocated/free set transitions, error-path
  ensures of no state change). Free-function `alloc()` spec is weak (only `frame.inv()`,
  no set membership). Free-function `free()` has no spec body.
- **View design (`UpoolView`)**: Models state as two disjoint `Set<int>`:
  `allocated_frames` and `free_frames`. Well-formedness requires page-alignment and
  disjointness. This is caller-abstract — callers don't know about the bitmap. The name
  `UpoolView` is slightly misleading since it's used for the frame allocator (`Inner`),
  not just the user pool (`Upool`). The same View could serve both since `Upool` is a
  thin facade.
- **`init()` gap**: No spec captures the post-initialization state (all frames free,
  allocated set empty). This would be needed for full verification.

## Abstract Resource

The frame allocator manages a **pool of physical memory frames** (page-sized, page-aligned
blocks of physical memory). It tracks which frames are free and which are allocated,
enforcing mutual exclusion: a frame is either free or allocated, never both.

## Key Invariants (caller perspective)

1. **Disjointness**: The set of allocated frames and the set of free frames are disjoint — a frame cannot be both allocated and free.
2. **Alloc ownership**: A frame returned by `alloc()` is removed from the free set and added to the allocated set. It will not be returned by a subsequent `alloc()` until freed.
3. **Free returns**: `free(frame)` moves the frame from allocated back to free. It fails if the frame is not currently allocated.
4. **Book = permanent alloc**: `book()` and `alloc_range()` move frames from free to allocated. They are used during boot to reserve regions that must never be allocated to user code.
5. **Init-once**: `init()` must be called exactly once before any other operation. It defines the universe of frames.
6. **Page alignment**: All frame addresses are page-aligned (enforced by `FrameAddress` type).
7. **Error-path no-change**: All operations preserve state on error — callers can safely retry or handle failures knowing no partial mutation occurred.
8. **Drop-safety**: `UserFrame::Drop` calls `free()`, which must succeed for a validly-held frame. This ensures RAII-style automatic reclamation.
