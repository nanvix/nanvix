# TCB Allowed List — Nanvix phys-mm

Any `external_body` outside this list must be removed.

## Allowed `external_body`

- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::deref`
- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::deref_mut`
- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::clear`
- `src/libs/bump_allocator/src/lib.rs::FixedSizeBumpAllocator::alloc` — materializes a
  `&'static mut [u8; N]` from a backend-provided address (`usize as *mut`); raw-memory
  op Verus cannot verify without a `PointsTo` for the externally-owned `BssStorage`
  region. Mirrors `src/libs/raw-array`. `ensures` states alignment + in-bounds over
  the abstract `bump_view`.
- `src/libs/bump_allocator/src/lib.rs::FixedSizeBumpAllocator::alloc_as` — delegates to
  `alloc` and re-materializes the slot as `&'static mut MaybeUninit<T>`; same rationale.
  `ensures` adds the `size_of::<T>()`/`align_of::<T>()` vs `(N, A)` guard arms.

## Skip / exclude from current proof target

- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::get_mut`
- `src/kernel/src/mm/phys/frame.rs::init`

## `external_body` introduced while speccing `mm::phys`

- `src/kernel/src/mm/phys/mod.rs::book_physical_memory_regions` — iterates an
  `alloc::collections::LinkedList` in a `for` loop. Verus has no `LinkedList` model and the
  orphan rule blocks providing one from the kernel crate (see
  `nanvix-phys-phys-mod/bugs.md`). Body cannot be verified; `ensures` states that, on `Ok`,
  every frame in `phys_regions_frame_set(&physical_memory_regions)` becomes reserved.
- `src/kernel/src/mm/phys/mod.rs::book_mmio_regions` — same `LinkedList` limitation.
  `ensures` states that, on `Ok`, every *covered* frame in `mmio_regions_frame_set(mmio_regions)`
  becomes reserved (uncovered MMIO frames are skipped, matching the `frame::is_covered` gate).

## Cross-module dependencies marked `external_body` (eliminated when their module is verified)

- `src/kernel/src/mm/phys/frame.rs::init` — also listed under skip; callable from verified `init`.
- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::init` — no specs yet; opaque callee.
- `src/kernel/src/mm/phys/upool.rs::Upool` (struct) and `Upool::new` — no specs yet; opaque
  type/callee needed so verified `init` can construct the user page pool.
