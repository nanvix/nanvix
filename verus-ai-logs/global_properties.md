# Global Properties — Nanvix Kernel Verification

Cross-module invariants identified during property analysis. These properties
span multiple modules and must be maintained as a system-level concern.

---

## GLOBAL-1: Heap memory region exclusivity

**Modules**: `mm::kheap`, linker script, kernel memory map

The memory region `[HEAP_STORAGE base, base + MIN_HEAP_SIZE)` must not overlap
with any other memory region used by the kernel (stack, BSS globals, MMIO regions,
page tables). This is an architectural property depending on linker script and
memory map — cannot be verified within any single module.

**Source**: kheap property analysis

---

## GLOBAL-2: Single initialization / boot ordering

**Modules**: `mm::kheap`, `kmain`

`kheap::init()` is called exactly once, before any allocation. The caller (`kmain`)
ensures this sequencing. After `init()` returns `Ok`, `HEAP` is `Some` for all
subsequent `alloc`/`dealloc` calls.

**Source**: kheap property analysis (FN-7a)

---

## GLOBAL-3: No concurrent access to kernel heap

**Modules**: `mm::kheap`, kernel scheduler/interrupt subsystem

`HEAP` and `HEAP_STORAGE` are `static mut`. The kernel must ensure no concurrent
access to the allocator (single-threaded initialization, interrupt masking during
allocation, or proper synchronization for multi-core). This is an architectural
invariant outside kheap's scope.

**Source**: kheap property analysis (BUG-5)

---

## GLOBAL-4: Layout consistency across alloc/dealloc

**Modules**: `mm::kheap`, all kernel callers of `alloc`/`dealloc`

The `GlobalAlloc` contract requires that `dealloc` is called with the same `Layout`
that was passed to `alloc`. If violated, `layout_to_allocator` may route to a
different slab, causing deallocation failure (state preserved, but memory leaked).

**Source**: kheap property analysis

---

## GLOBAL-5: Architecture-constant coupling

**Modules**: `mm::kheap`, `arch::mem`, `config::constants`

Correctness of the kernel heap depends on:
- `PAGE_SIZE == 4096`
- All slab block sizes (8, 16, ..., 512/4096) are powers of two that divide `PAGE_SIZE`
- `MIN_SLAB_SIZE == 32 * PAGE_SIZE`

Partition and alignment guarantees are tied to these constants.

**Source**: kheap property analysis (TYPE-3, TYPE-6, LIVE-1)

---

## Slab-level properties (from slab crate verification)

These were verified during the slab crate analysis and are referenced by kheap:

- **Slab invariant preservation**: `allocate` and `deallocate` preserve `slab.inv()`.
- **Slab allocation disjointness**: Within a single slab, `allocated_addrs` and
  `free_addrs` are always disjoint.
- **Slab address bounds**: All addresses in a slab's sets lie within
  `[start_addr, end_addr)` and are block-aligned.
- **Slab state transitions**: `allocate` moves one address free→allocated;
  `deallocate` moves one address allocated→free. No other changes.

**Source**: `src/libs/slab/src/lib.spec.rs`, `src/libs/slab/src/lib.rs`
