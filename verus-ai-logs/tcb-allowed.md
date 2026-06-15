# TCB Allowed List — Nanvix phys-mm

This file is a machine-readable policy hint for AI proof phases.

Only the concrete functions listed here may keep or introduce `external_body`
in the phys-mm verification scope. Any `external_body` outside this list must
be removed or explicitly approved before continuing.

## A. Allowed TCB / `external_body` functions

| Function | File | Scope | Reason |
|---|---|---|---|
| `KernelFrame::deref()` | `src/kernel/src/mm/phys/kframe.rs` | Entire function | Converts a physical frame address into an immutable byte slice with `core::slice::from_raw_parts`; raw memory / identity-map boundary. |
| `KernelFrame::deref_mut()` | `src/kernel/src/mm/phys/kframe.rs` | Entire function | Converts a physical frame address into a mutable byte slice with `core::slice::from_raw_parts_mut`; raw memory / identity-map / aliasing boundary. |
| `KernelFrame::clear()` | `src/kernel/src/mm/phys/kframe.rs` | Entire function, for now | Performs raw physical memory effect via identity-map `memset`. A precise postcondition would require modeling page contents with a `PointsTo` owned by `KernelFrame` or passed as an explicit parameter. |
| `frame::install_global()` or equivalent wrapper | `src/kernel/src/mm/phys/frame.rs` | Only the global singleton store | Trusted wrapper should only write an already-constructed `Inner` into `INSTANCE` and set `INSTANCE_INIT`. If this wrapper does not exist yet, refactor `frame::init()` to introduce it. |

## B. Functions to skip / exclude from current proof target

These are singleton/global plumbing. Do not spend proof effort on them in the
current phys-mm pipeline. If they currently contain other logic, split that
logic into a lower-level function and prove the lower-level function instead.

| Function | File | Required handling |
|---|---|---|
| `PhysMemoryManager::get_mut()` | `src/kernel/src/mm/phys/manager.rs` | Exclude from proof target or allow `external_body`; it returns `&mut` from global `PHYS_MEMORY_MANAGER`. |
| `frame::init()` | `src/kernel/src/mm/phys/frame.rs` | Do **not** permanently trust the whole function. Split into verified `Inner::new(bitmap, refcount)` plus trusted `frame::install_global(inner)` (listed in section A). Until split, only the global-store part is intended to be trusted. |

## C. Must not be skipped by default

Do **not** add `external_body` merely because these are hard to prove. They are
phys-mm proof targets unless separately approved.

- `frame::Inner::{alloc, alloc_contiguous, free, share, refcount, book, is_covered, alloc_range}`
- public `frame::{alloc, alloc_contiguous, free, share, refcount, book, is_covered, alloc_range, free_count}` wrappers, except where they only delegate to approved singleton plumbing
- `Upool::{new, alloc}`
- `UserFrame::{new, address, leak, share, refcount, drop}`
- `PhysMemoryManager::{init, alloc_user_frame, alloc_many_user_frames, alloc_kernel_frame, alloc_many_kernel_frames, check_user_watermark}` except for the separately listed singleton accessor `get_mut()`
- `raw-array` and `bitmap` dependencies; `Bitmap` already has specs and is not a phys-mm TCB boundary

## D. Refactoring rule for global singletons

For global singleton code such as `INSTANCE.write(...)`, `INSTANCE.assume_init_mut()`,
`PHYS_MEMORY_MANAGER.write(...)`, or `PHYS_MEMORY_MANAGER.assume_init_mut()`:

1. Do not prove the global memory access itself in the current pipeline.
2. Move any meaningful allocator/state construction into an inner function or
   struct method that can be specified and proved.
3. Keep only the minimal global store/access wrapper in TCB.

