# TCB Allowed List — Nanvix phys-mm

Any `external_body` outside this list must be removed.

## Allowed `external_body`

- `src/kernel/src/mm/phys/mod.rs::book_physical_memory_regions` — iterates a std
  `alloc::collections::LinkedList` via `for region in list.iter()`. The build
  toolchain's `vstd` (`/mnt/toolchain/verus/vstd`) ships ghost-iterator specs only
  for `slice`/`vec`/`VecDeque`/`BTree*`/`Hash*` iterators, **not** `LinkedList`, and
  the orphan rule forbids supplying `View`/`ForLoopGhostIterator` impls for the
  foreign `linked_list::Iter` from the `kernel` crate (E0117). Genuine Verus
  front-end limitation, not a proof gap or code bug — full analysis in
  `verus-unsupported.md`. The abstract booking effect is specified over
  `PhysMemView` and discharged (no `admit`) by
  `lemma_book_region_reserves_region_frames` in `mod.proof.rs`. `ensures` keeps the
  caller-relevant guarantee: allocator stays initialized and well-formed.
- `src/kernel/src/mm/phys/mod.rs::book_mmio_regions` — same std-`LinkedList`
  iteration limitation as above. Abstract effect (skip-if-not-covered; book tracked
  frames) discharged by `lemma_book_mmio_skip_untracked` /
  `lemma_book_mmio_books_tracked`. `ensures` keeps allocator initialized and
  well-formed across booking.
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
