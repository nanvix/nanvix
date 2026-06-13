# Caller Analysis: `mm::phys` (`mod.rs`)

## Script Output

Ran:
```bash
python scripts/find_callers_lsp.py src/kernel/src/mm/phys/mod.rs --project-dir /home/ruize/nanvix-phy
```

Summary (rust-analyzer LSP, intra-crate only; crate `kernel`, no external dependents):

| Category | Count |
|----------|------:|
| Total exec functions | 4 |
| Public / trait-pub | 2 (`init`, `test`) |
| Private | 2 (`book_physical_memory_regions`, `book_mmio_regions`) |
| Types | 0 |

Call sites found:

- `init` [pub] — **1 external caller**
  - `src/kernel/src/mm/kernel_vas.rs:120` — `phys::init(physical_memory_regions, &mmio_regions, physical_memory_layout)?;`
- `test` [pub] — 0 external callers reported (see note below; actually called at `kernel_vas.rs:123` under `#[cfg(feature = "test")]`).
- `book_physical_memory_regions` [private] — called by `init` (L128).
- `book_mmio_regions` [private] — called by `init` (L130).

### Validation / corrections to script output

- The two in-scope helpers `book_physical_memory_regions` and `book_mmio_regions`
  are **private** and reachable only through `init`. They have exactly one caller
  each (`init`), confirmed by reading the code. They are not part of the public API
  surface — their contracts only need to support `init`'s post-state.
- The script lists `test` as having "0 external callers", but `test` is **out of
  scope** for this module's verification (target functions are `init`,
  `book_mmio_regions`, `book_physical_memory_regions`). It is invoked at
  `kernel_vas.rs:123` behind `#[cfg(feature = "test")]`. No action needed.
- No callers via function pointers, closures, generics, or macros were found. `init`
  is a plain free function called by name. No implicit/trait-dispatched callers exist
  for the in-scope functions (no `Drop`/`GlobalAlloc`/`Iterator` impls involved).

## Caller Context (read at the call site)

`kernel_vas::init` (the root-VAS bring-up path) is the sole caller. It:

1. Parses memory regions into `(other_virtual, virtual, physical)` region lists.
2. Calls `phys::init(physical_memory_regions, &mmio_regions, physical_memory_layout)?`
   **first**, before any virtual-memory setup (`virt::init`, `VirtMemoryManager::init`).
3. Propagates errors with `?` — on failure, boot aborts; no partial-state recovery.
4. After `phys::init` returns `Ok`, it proceeds to build page tables and the root
   address space, which depend on the physical frame allocator + `PhysMemoryManager`
   singleton being live and consistent.

So the caller treats `phys::init` as a **one-shot, boot-time initialization barrier**:
after `Ok(())`, the global physical-memory subsystem (frame allocator singleton +
`PhysMemoryManager` singleton + user page pool) must be fully initialized and all
non-usable / reserved / MMIO frames must be booked so subsequent `alloc()` never
hands them out.

## Trait Obligations

None. The in-scope functions implement no trait; they are free functions. No
`Drop`/`Iterator`/`GlobalAlloc` contracts apply.

## Caller Expectations

### `init` (pub) — the only externally-observed function

Inputs:
- `physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>>` —
  physical regions that are *not usable* and must be booked (removed from the free pool).
- `mmio_regions: &LinkedList<TruncatedMemoryRegion<VirtualAddress>>` — MMIO regions
  (by GVA) to book where they intersect tracked RAM.
- `physical_memory_layout: Bitmap` — the frame bitmap that seeds the frame allocator
  (defines which frames exist / are tracked).

Callers assume after `Ok(())`:
- The frame allocator singleton is initialized exactly once (via `frame::init`) from
  `physical_memory_layout`, and is now the live global instance.
- Every frame in every `physical_memory_regions` entry has been booked
  (`frame::alloc_range`), i.e. moved from `free_frames` to `allocated_frames` so that
  later `frame::alloc()` will never return them. Booking a region that is not entirely
  free is an error (returns `Err`, see failure case).
- Every MMIO frame that the allocator actually tracks (`frame::is_covered`) has been
  booked (`frame::book`). MMIO frames **above** tracked RAM (e.g. LAPIC at
  `0xFEE0_0000`) are silently skipped — callers must NOT assume those are booked.
- A fresh user page pool (`Upool::new()`) exists and the `PhysMemoryManager` singleton
  has been initialized with it (`PhysMemoryManager::init`).
- The whole subsystem is internally consistent (frame allocator `inv()` holds: alloc/free
  disjoint, page-aligned, refcount/allocated consistency per `FrameAllocView::wf`).

Callers assume after `Err(_)`:
- Boot aborts (caller uses `?`). The caller does not attempt to use the physical-memory
  subsystem afterward and does not expect any particular partial state to be safe; it
  only relies on the error being surfaced. (Internally an error can leave the singleton
  initialized but with incomplete booking — but no in-scope caller observes this, since
  it stops on error.)

What would break the caller if internals changed:
- If a frame belonging to a booked physical/MMIO region could still be returned by a
  later `alloc()` (booking not effective) — this is the core safety property the caller
  depends on.
- If `init` could be called and silently leave the global singletons uninitialized while
  returning `Ok` — downstream `virt::init` / page-table setup would touch an
  uninitialized allocator.
- If the success/failure of booking an already-allocated or out-of-coverage region were
  reported inconsistently (e.g. an overlapping region silently ignored instead of
  erroring) — the caller relies on conflicts being surfaced as `Err`.

What would NOT break the caller:
- The internal representation of the free/allocated set (bitmap vs. anything else),
  the refcount storage layout, the ordering in which regions are iterated, or the exact
  `Error` variant returned. Callers only observe `Ok(())` vs `Err`.
- Whether MMIO booking walks GVA→GPA per-page or in some other way, as long as covered
  MMIO frames end up booked and uncovered ones are skipped.
- Logging / `info!` output.

### `book_physical_memory_regions` (private helper)

Only `init` calls it. Caller (`init`) assumes:
- On `Ok(())`: every frame of every region in the list is now allocated/booked (delegates
  to `frame::alloc_range` per region, which requires each range to be fully free).
- On `Err(_)`: propagated up via `?`; `init` aborts. Booking stops at the first failing
  region (regions iterated in list order); already-booked earlier regions stay booked.
- Consumes the list by value (`LinkedList<...>` taken by move) — caller does not reuse it.

### `book_mmio_regions` (private helper)

Only `init` calls it. Caller (`init`) assumes:
- On `Ok(())`: for every MMIO region, each page-aligned GPA frame that
  `frame::is_covered` reports as tracked has been booked via `frame::book`; tracked
  frames within MMIO ranges are removed from the free pool. Frames outside tracked RAM
  are intentionally skipped (no error).
- On `Err(_)`: propagated via `?` (`init` aborts). Errors can come from
  `PhysicalAddress::from_mmio_address`, `PageAligned::from_address`, or `frame::book`
  (e.g. booking an already-allocated covered frame).
- Borrows the list (`&LinkedList<...>`) — does not consume it.

## Abstract Resource

`mm::phys` manages the **global physical-memory allocation state of the machine**: a
finite set of physical frames partitioned into *free* and *allocated/reserved*, plus the
`PhysMemoryManager` / user-page-pool singletons layered on top. `init` is the boot-time
constructor that establishes this state from the firmware-provided bitmap and reserves
(books) all frames that must never be handed out (non-usable RAM regions and tracked MMIO
frames).

The relevant abstract view is `FrameAllocView { allocated_frames, free_frames, refcounts }`
(already defined): `init` must end with all booked regions' frames in `allocated_frames`
(each with refcount 1) and removed from `free_frames`, while preserving `FrameAllocView::wf`.

## Key Invariants (caller perspective)

- **One-shot init**: `init` initializes the global frame allocator exactly once
  (`frame::init` rejects a second call); callers rely on it being live and consistent
  afterward.
- **Booked ⇒ never allocated**: any frame in a booked physical region, or any covered
  frame in an MMIO region, is in `allocated_frames` (not `free_frames`) on success, so
  later `alloc()` cannot return it.
- **Coverage-gated MMIO booking**: only frames the allocator tracks are booked; MMIO
  frames above RAM are skipped without error.
- **Free/allocated disjoint, page-aligned, refcount-consistent** (`FrameAllocView::wf`)
  holds across `init` and is what downstream virtual-memory setup depends on.
- **Fail-fast**: any booking conflict (overlap / already-allocated / out-of-coverage
  range) surfaces as `Err`, aborting boot rather than silently corrupting the free pool.
