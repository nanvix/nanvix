# Unified Physical Memory Allocator

## Motivation

The kernel currently partitions physical memory into two independent pools:

- **`kpool`** — A fixed-size reserved region for kernel-internal allocations (page tables,
  kernel stacks). Backed by a `Bitmap` with capacity determined at boot.
- **`upool`** — A thin facade over the global frame allocator (`SparseBitmap`) that serves
  user-space page requests from the remaining physical memory.

This split creates an artificial ceiling: the `kpool` bitmap has a fixed capacity, and workloads
that create many threads or page tables can exhaust it even when the system has abundant free
frames in the `upool`. This is the root cause of [#1270], where 64 concurrent threads exhaust the
kernel pool bitmap while the user pool remains largely unused.

**Goal:** Eliminate the fixed `kpool` partition by routing all frame allocations — kernel and
user — through a single `SparseBitmap`-backed frame allocator.

[#1270]: https://github.com/nanvix/nanvix/issues/1270

---

## Architecture

### Current Design

```text
┌──────────────────────────────────────────────────────────────┐
│                    PhysMemoryManager                          │
│                                                              │
│  ┌────────────────────┐       ┌────────────────────────────┐ │
│  │       Kpool        │       │          Upool             │ │
│  │  (own Bitmap)      │       │  (facade → frame::alloc)   │ │
│  │                    │       │                            │ │
│  │  Fixed region:     │       │  All remaining RAM:        │ │
│  │  KPOOL_BASE..+SIZE │       │  SparseBitmap              │ │
│  │                    │       │                            │ │
│  │  alloc()           │       │  alloc()                   │ │
│  │  alloc_many()      │       │                            │ │
│  │  free()            │       │                            │ │
│  └────────────────────┘       └────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

**Problem:** `Kpool`'s bitmap capacity is fixed at `KPOOL_SIZE / PAGE_SIZE`. Once full, kernel
allocations fail with `OutOfMemory` even though the frame allocator (`SparseBitmap`) may have
thousands of free frames available.

### Proposed Design

```text
┌──────────────────────────────────────────────────────────────┐
│                    PhysMemoryManager                          │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Unified Frame Allocator                    │  │
│  │              (SparseBitmap singleton)                   │  │
│  │                                                        │  │
│  │  alloc()            → single frame                     │  │
│  │  alloc_contiguous() → N contiguous frames              │  │
│  │  free()             → return frame                     │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────┐          ┌────────────────────────┐    │
│  │   KernelFrame    │          │     UserFrame          │    │
│  │   (newtype)      │          │     (newtype)          │    │
│  │   + Deref<[u8]>  │          │     (opaque addr)      │    │
│  │   + clear()      │          │                        │    │
│  └──────────────────┘          └────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

**Key change:** Both `KernelFrame` and `UserFrame` are allocated from the same underlying
`SparseBitmap`. The type distinction is preserved for API safety — `KernelFrame` retains
`Deref`/`DerefMut` for direct kernel access, while `UserFrame` remains an opaque handle.

---

## Design Principles

1. **Single source of truth.** One allocator owns all physical frames. No partitioning at boot.
2. **Type safety preserved.** `KernelFrame` and `UserFrame` remain distinct types with different
   capabilities. The allocator does not conflate them.
3. **Contiguous allocation supported.** `SparseBitmap::alloc_range(count)` already provides
   O(n) contiguous-bit scanning with chunk-level hints — identical semantics to what `Kpool`
   offers today via `Bitmap::alloc_range()`. This is required for kernel stack allocation:
   `KernelStack::new()` uses `alloc_many_kernel_frames()` because the kernel uses identity
   mapping and the hardware stack pointer traverses the region linearly. Page tables, in
   contrast, are allocated one frame at a time via `alloc_kernel_frame()`.
4. **Minimal API surface change.** Callers of `PhysMemoryManager` continue to call
   `alloc_kernel_frame()`, `alloc_many_kernel_frames()`, and `alloc_many_user_frames()`.
   Internal routing changes; external contracts remain stable.
5. **No boot-time reservation.** The `KPOOL_BASE`/`KPOOL_SIZE` constants and the dedicated
   physical region are eliminated. Frames that were previously reserved for `kpool` become
   available to the general pool.
6. **Demand-side kernel watermark.** A low-water mark gates user allocations — not kernel ones.
   User requests are rejected when free frames would drop below `KERNEL_WATERMARK`. Kernel
   allocations are never gated, so there is no artificial ceiling. This avoids kpool's
   fundamental flaw (supply-side cap) while still protecting the kernel from user exhaustion.

---

## Component Changes

### `frame.rs` — Frame Allocator Singleton

Add `alloc_contiguous(count)` and `free_count()` functions:

```rust
/// Allocates `count` physically contiguous frames.
///
/// Returns the base `FrameAddress` of the contiguous range.
pub(super) fn alloc_contiguous(count: usize) -> Result<FrameAddress, Error> {
    instance().alloc_contiguous(count)
}

/// Returns the number of free frames in the system.
///
/// Safe without locking: Nanvix is single-core with interrupts disabled,
/// so no TOCTOU race is possible.
pub(super) fn free_count() -> usize {
    let inner = instance();
    inner.bitmap.capacity() - inner.bitmap.usage()
}
```

**Prerequisite:** `SparseBitmap` needs a `usage()` method that sums per-chunk `Bitmap::usage`
fields. `Bitmap` already maintains `usage` internally — exposing it is trivial.

Backed by `SparseBitmap::alloc_range(count)`, which already:

- Scans chunks with a `next_chunk_hint` for amortized O(1) best-case.
- Delegates to `Bitmap::alloc_range()` within each chunk (fast-skip of full bytes, wrap-around).
- Returns the starting bit index of the contiguous range.

### `manager.rs` — PhysMemoryManager

Remove the `kpool: Kpool` field. Route kernel frame allocation through `frame`:

```rust
pub struct PhysMemoryManager {
    // kpool removed
    _private: (),
}

impl PhysMemoryManager {
    /// Used by page-table allocation and single-frame kernel requests.
    /// Kernel allocations bypass the watermark — no artificial ceiling.
    pub fn alloc_kernel_frame(&mut self) -> Result<KernelFrame, Error> {
        let addr = frame::alloc()?;
        Ok(KernelFrame::new(addr))
    }

    /// Used by KernelStack::new() — kernel stacks require physically contiguous
    /// frames because the kernel uses identity mapping and the hardware SP
    /// traverses the region linearly.
    /// Kernel allocations bypass the watermark — no artificial ceiling.
    pub fn alloc_many_kernel_frames(
        &mut self,
        count: usize,
        frames: &mut Vec<KernelFrame>,
    ) -> Result<(), Error> {
        let base_addr = frame::alloc_contiguous(count)?;
        for i in 0..count {
            let addr = base_addr.offset(i * PAGE_SIZE);
            frames.push(KernelFrame::new(addr));
        }
        Ok(())
    }

    /// User allocations are gated by the kernel watermark: if fulfilling
    /// the request would leave fewer than KERNEL_WATERMARK free frames,
    /// the allocation is rejected with OutOfMemory.
    pub fn alloc_many_user_frames(
        &mut self,
        count: usize,
        frames: &mut Vec<UserFrame>,
    ) -> Result<(), Error> {
        if frame::free_count() < constants::KERNEL_WATERMARK + count {
            return Err(Error::new(
                ErrorCode::OutOfMemory,
                "would breach kernel watermark",
            ));
        }
        // ... existing per-frame allocation loop
    }
}
```

### `kpool.rs` — Kernel Frame Pool

**Deleted.** The `Kpool` struct, its `Bitmap`, and the singleton are removed entirely.

The `KernelFrame` type is **retained** (moved to its own file or into `manager.rs`) because it
provides `Deref<Target = [u8; PAGE_SIZE]>` for kernel page-table manipulation and
`Drop`-based freeing through `frame::free()`.

### `upool.rs` — User Frame Pool

Unchanged. It is already a zero-state facade over `frame::alloc()`.

### `phys/mod.rs` — Initialization

The `init()` function simplifies:

```rust
pub fn init(
    physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
    mmio_regions: &LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    physical_memory_layout: SparseBitmap,
) -> Result<(), Error> {
    // Initialize the single frame allocator.
    unsafe { frame::init(physical_memory_layout)? };
    book_physical_memory_regions(physical_memory_regions)?;
    book_mmio_regions(mmio_regions)?;

    // No kpool initialization.
    // No kpool region booking (those frames are now available to everyone).

    PhysMemoryManager::init()?;
    Ok(())
}
```

Parameters removed: `kpool_base`, `kpool_bitmap`.

### Boot / Platform Layer (`hal/platform/microvm/mod.rs`, `hal/platform/hyperlight/mod.rs`)

- Remove `KPOOL_BASE` / `KPOOL_SIZE` from memory layout calculations.
- Remove kpool bitmap construction and the `book` call that reserves kpool frames.
- The physical memory formerly reserved for kpool becomes available in the `SparseBitmap`.

### Config (`libs/config`)

- Remove `KPOOL_BASE`, `KPOOL_SIZE`, and related compile-time constants from `build.rs` and
  `lib.rs`.
- Remove the build-time assertion `kpool_end <= memory_size`.
- Add `KERNEL_WATERMARK` constant (in frames) to `kernel_config.toml`. Recommended value:
  32–64 frames (128–256 KB), sized to cover worst-case kernel cleanup operations (freeing page
  tables, reclaiming stacks). The watermark is a soft reservation — no memory is locked away;
  it merely gates user-side allocation requests.

---

## Contiguous Allocation: Why It Works

The `SparseBitmap` is composed of chunks, each backed by a `Bitmap`. The existing
`Bitmap::alloc_range(count)` algorithm:

1. Starts from `next_free` hint (amortized skip of allocated prefix).
2. Fast-skips fully-occupied bytes (`byte == 0xFF`).
3. Scans for `count` consecutive zero bits.
4. Wraps around once if needed.
5. Returns starting index or `OutOfMemory`.

`SparseBitmap::alloc_range(count)` iterates chunks starting from `next_chunk_hint`, trying each
chunk's bitmap. A contiguous allocation cannot span chunks (by design), which mirrors the
current kpool behavior where all frames come from a single contiguous region.

**Important constraint:** If a caller needs more contiguous frames than fit in a single chunk,
the allocation will fail. In practice, Nanvix's chunk size equals the physical memory extent
(one chunk per memory region), so this is not a limitation for typical deployments.

---

## Migration Strategy

### Phase 1: Add `frame::alloc_contiguous()`

- Add the `alloc_contiguous(count)` function to `frame.rs`.
- Add unit tests verifying contiguous allocation and OOM behavior.
- No callers changed yet — pure addition.

### Phase 2: Route Kernel Allocations Through Frame Allocator

- Modify `PhysMemoryManager::alloc_kernel_frame()` to call `frame::alloc()`.
- Modify `PhysMemoryManager::alloc_many_kernel_frames()` to call `frame::alloc_contiguous()`.
- Move `KernelFrame` out of `kpool.rs` (keep type, `Deref`, and `Drop` intact).
- Update `KernelFrame::Drop` to free via `frame::free()` instead of `kpool::free()`.
- **Verify:** All existing tests pass with no behavioral change.

### Phase 3: Remove `kpool` Module and Boot Reservation

- Delete `kpool.rs`.
- Remove `kpool_base` and `kpool_bitmap` parameters from `phys::init()`.
- Remove kpool region setup from platform boot code (microvm, hyperlight).
- Remove `KPOOL_BASE` / `KPOOL_SIZE` from config crate.
- Remove the frame-allocator `book()` call that reserved the kpool region.

### Phase 4: Validate Under Stress

- Re-run the reproduction case (`SCOREBOARD_SLOT_HINT = 32`, 64 workers) and confirm the test
  passes without OOM.
- Run the full test suite (`test/test-standalone.toml`) to verify no regressions.
- Run integration tests for both microvm and hyperlight platforms to confirm correct behavior
  across all supported backends.
- Benchmark boot time and steady-state allocation latency (should be negligible difference since
  `SparseBitmap` has the same O(n) scan as `Bitmap`).

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
| ------ | -------- | ------------ |
| Fragmentation under mixed kernel+user allocation | Contiguous allocation may fail when memory is fragmented | Monitor with stress tests; kernel frames are small (1 page) so single-frame alloc is unaffected; contiguous alloc is only used for kernel stack allocation which typically requests `KSTACK_SIZE / PAGE_SIZE` frames (currently 2–4) |
| Hyperlight platform assumptions | Hyperlight models kpool as a distinct guest-memory region in the PEB layout | Update Hyperlight layout code to remove the kpool region; reclaim those bytes into general guest memory |
| Verified (Verus) proofs on `Bitmap` | `alloc_range` in the bitmap crate has formal proofs | No change to the bitmap crate itself — proofs remain valid since we're using the same `SparseBitmap::alloc_range()` that already delegates to the proven `Bitmap::alloc_range()` |
| `alloc_many_kernel_frames` callers assume contiguity | Kernel stacks depend on physical contiguity | Audited: the sole caller is `KernelStack::new()` (via `VirtMemoryManager::alloc_kpages`). Kernel stacks require contiguous frames because the kernel uses identity mapping and the hardware SP traverses the region linearly. Contiguous allocation semantics must be preserved. |
| Watermark too high wastes user-visible memory | User workloads receive premature OOM | Size `KERNEL_WATERMARK` to worst-case kernel cleanup (thread teardown + page-table walk). 32–64 frames (128–256 KB) is conservative for current workloads. Make it a `kernel_config.toml` constant so it can be tuned per-deployment without code changes. |

---

## Known Limitations

1. **Chunk-spanning contiguous allocation.** The current `SparseBitmap` does not support
   contiguous allocation across chunk boundaries. A contiguous allocation request must be
   satisfiable within a single chunk. This is acceptable for all current deployments because
   physical memory is presented as a single chunk (microvm) or a single guest memory region
   (Hyperlight). Multi-region platforms that expose disjoint physical memory ranges as separate
   chunks would need to either (a) ensure each region is large enough to satisfy the largest
   contiguous request (currently `KSTACK_SIZE / PAGE_SIZE` frames, i.e. 2–4 frames), or
   (b) extend `SparseBitmap` to scan across chunk boundaries. This limitation is intentionally
   not addressed in the current design — it can be revisited if Nanvix gains support for
   platforms with many small disjoint memory regions.
