# Property Analysis: `mm::kheap` — Kernel Heap Allocator

**Module**: `src/kernel/src/mm/kheap.rs`
**Domain**: Memory management — slab-based kernel heap allocator
**Verified dependency**: `Slab` (with `SlabView` abstract state, fully verified)

---

## 1. Module Overview

The `kheap` module implements the kernel's dynamic memory allocator. It manages a
statically-allocated `HeapStorage` region, partitions it into `NUM_OF_SLABS` equally-sized
arenas, and wraps each in a `Slab` configured for a specific block size (8, 16, 32, …, 512
bytes; extended to 4096 under `hyperlight`).

**Public interface**:
- `init()` — one-time initialization from `kmain`
- `GlobalAlloc` trait impl (`alloc`/`dealloc`) — called by Rust's allocator infrastructure

**Key design properties**:
- Fixed-size slab allocator: each allocation is rounded up to the next power-of-two slab tier.
- Slabs occupy disjoint, contiguous sub-regions of a single page-aligned static buffer.
- `HEAP` is a global `Option<Kheap>` — `None` before `init()`, `Some` after.
- `layout_to_allocator` is the pure routing function from `Layout` → `SlabSize`.

**Verified dependency (`Slab`)**:
- `SlabView` models each slab as `{ block_size, start_addr, end_addr, allocated_addrs, free_addrs }`.
- `SlabView::inv()` guarantees block-alignment, range validity, and allocated/free disjointness.
- `from_raw_parts`, `allocate`, `deallocate` have full bidirectional contracts (see §4 preamble).

---

## 2. Abstract State Design

### `KheapView` — Proposed Abstract State

The abstract state should expose the heap as a collection of typed slab views without
leaking internal field structure. A caller cares about: (a) what addresses are allocated,
(b) what addresses are free, (c) the disjointness of slab regions, and (d) the invariant
that all slabs are well-formed.

```rust
pub struct KheapView {
    /// Abstract state of each slab, indexed by slab tier.
    /// slabs[0] = 8-byte slab, slabs[1] = 16-byte slab, etc.
    pub slabs: Seq<SlabView>,
}
```

**Design rationale**: Using `Seq<SlabView>` (length = `NUM_OF_SLABS`) allows quantified
properties over all slabs without enumerating each field. The index mapping is:

| Index | Block size | Field              |
|------:|-----------:|:-------------------|
|     0 |          8 | `slab_8_bytes`     |
|     1 |         16 | `slab_16_bytes`    |
|     2 |         32 | `slab_32_bytes`    |
|     3 |         64 | `slab_64_bytes`    |
|     4 |        128 | `slab_128_bytes`   |
|     5 |        256 | `slab_256_bytes`   |
|     6 |        512 | `slab_512_bytes`   |
|   7*  |       1024 | `slab_1024_bytes`  |
|   8*  |       2048 | `slab_2048_bytes`  |
|   9*  |       4096 | `slab_4096_bytes`  |

\* hyperlight-only

### Convenience spec functions on `KheapView`

```
spec fn all_allocated(&self) -> Set<usize>
    // Union of allocated_addrs across all slabs
spec fn all_free(&self) -> Set<usize>
    // Union of free_addrs across all slabs
spec fn slab_for_size(&self, size: usize) -> Option<nat>
    // Maps allocation size to slab index (mirrors layout_to_allocator)
```

---

## 3. Type Invariants

### TYPE-1: `KheapView` well-formedness

Each slab in the heap satisfies its own invariant, slab count matches configuration,
and block sizes follow the expected power-of-two sequence.

```
∀ i: 0 <= i < self.slabs.len() ==>
    self.slabs[i].inv()

self.slabs.len() == NUM_OF_SLABS
```

### TYPE-2: Slab region disjointness

No two slabs overlap in their address ranges. This is the **central safety property**
of the slab allocator — without it, one slab's allocation could corrupt another's data.

```
∀ i, j: 0 <= i < j < self.slabs.len() ==>
    self.slabs[i].end_addr <= self.slabs[j].start_addr
    ∨ self.slabs[j].end_addr <= self.slabs[i].start_addr
```

In practice, slabs are laid out contiguously so the stronger form holds:

```
∀ i: 0 <= i < self.slabs.len() - 1 ==>
    self.slabs[i].end_addr <= self.slabs[i + 1].start_addr
```

### TYPE-3: Block-size monotonicity

Slab block sizes follow a strict doubling sequence. This ensures `layout_to_allocator`
routes to the tightest-fitting slab.

```
∀ i: 0 <= i < self.slabs.len() - 1 ==>
    self.slabs[i].block_size < self.slabs[i + 1].block_size
```

Specifically:
```
self.slabs[0].block_size == 8
self.slabs[1].block_size == 16
...
self.slabs[6].block_size == 512
// (hyperlight: self.slabs[7..9] == 1024, 2048, 4096)
```

### TYPE-4: Heap storage containment

All slab regions lie within the `HEAP_STORAGE` buffer.

```
let base = HEAP_STORAGE.memory.as_ptr() as usize;
let bound = base + MIN_HEAP_SIZE;
∀ i: 0 <= i < self.slabs.len() ==>
    base <= self.slabs[i].start_addr
    ∧ self.slabs[i].end_addr <= bound
```

### TYPE-5: `SlabSize` enum value correctness

Each `SlabSize` variant's discriminant equals the block size it represents.
This is used by `layout_to_allocator` and `from_raw_parts` to pass block sizes to `Slab`.

```
SlabSize::Slab8 as usize == 8
SlabSize::Slab16 as usize == 16
...
SlabSize::Slab512 as usize == 512
```

### TYPE-6: `HeapStorage` alignment

`HEAP_STORAGE.memory.as_ptr() as usize` is a multiple of `PAGE_SIZE` (4096).
This follows from `#[repr(align(4096))]` on `HeapStorage` and the static assertion.

```
HEAP_STORAGE.memory.as_ptr() as usize % PAGE_SIZE == 0
```

---

## 4. Function Contracts

### Preamble: Slab dependency contracts (summary)

These are **assumed verified** and referenced throughout:

- **Slab::from_raw_parts(addr, len, block_size)**:
  On `Ok(slab)`: `slab.inv()`, `slab@.block_size == block_size`, `slab@.start_addr >= addr`,
  `slab@.end_addr <= addr + len`, `slab@.allocated_addrs == ∅`.
  On `Err`: `ErrorCode::InvalidArgument` ∧ at least one precondition violated.

- **Slab::allocate(&mut self)**: Requires `self.inv()`.
  On `Ok(ptr)`: addr was free, now allocated; other slab state unchanged.
  On `Err`: free set was empty; state unchanged.

- **Slab::deallocate(&mut self, ptr)**: Requires `self.inv()`.
  On `Ok(())`: ptr was allocated, now free; other slab state unchanged.
  On `Err`: ptr was not allocated; state unchanged.

---

### FN-1: `Kheap::layout_to_allocator(layout: &Layout) -> Result<SlabSize, AllocError>`

**Purpose**: Pure routing function. Maps an allocation size to the appropriate slab tier.

**Preconditions**: None (total function over all `Layout` values).

**Success postcondition** (`Ok(slab_size)`):
```
FN-1a: layout.size() >= 1 ∧ layout.size() <= MAX_SLAB_SIZE
        ==> result is Ok

FN-1b: slab_size as usize >= layout.size()
        // Returned slab is large enough for the requested allocation.

FN-1c: // Tightest fit: no smaller slab tier could serve this size.
        // Formally: slab_size is the smallest SlabSize variant
        // whose discriminant >= layout.size().
```

Where `MAX_SLAB_SIZE` = 512 (or 4096 under `hyperlight`).

**Error postcondition** (`Err(AllocError)`):
```
FN-1d: layout.size() == 0 ∨ layout.size() > MAX_SLAB_SIZE
        <==> result is Err
```

**Note**: `layout.size() == 0` maps to `Err` because `match` range starts at 1. This is
correct behavior — zero-sized allocations are meaningless for slab allocation.

**Frame**: No state mutation (pure function).

---

### FN-2: `Kheap::from_raw_parts(addr: usize, size: usize) -> Result<Kheap, Error>`

**Purpose**: Construct a `Kheap` by partitioning a raw memory region into `NUM_OF_SLABS`
slabs of equal size, each configured for its block-size tier.

**Preconditions** (caller must guarantee the memory region is valid and exclusively owned):
```
FN-2a: addr points to a valid, exclusively-owned memory region of at least `size` bytes.
        // Cannot be expressed in Verus without a memory permission model;
        // this is a safety precondition documented as a comment.
```

**Success postcondition** (`Ok(heap)`):
```
FN-2b: heap.inv()
        // All TYPE-1 through TYPE-4 hold on the returned Kheap.

FN-2c: ∀ i: 0 <= i < NUM_OF_SLABS ==>
            heap@.slabs[i].allocated_addrs == Set::<usize>::empty()
        // All slabs start fully unallocated.

FN-2d: heap@.slabs[i].block_size == BLOCK_SIZES[i]
        // where BLOCK_SIZES = [8, 16, 32, 64, 128, 256, 512, ...]

FN-2e: // Slab layout: each slab occupies [addr + i*slab_size, addr + (i+1)*slab_size)
        let slab_size = size / NUM_OF_SLABS;
        ∀ i: 0 <= i < NUM_OF_SLABS ==>
            heap@.slabs[i].start_addr >= addr + i * slab_size
            ∧ heap@.slabs[i].end_addr <= addr + (i + 1) * slab_size
```

**Error postcondition** (`Err(e)`):
```
FN-2f: e.code == ErrorCode::InvalidArgument

FN-2g: // Bidirectional failure condition:
        addr % PAGE_SIZE != 0
        ∨ size < MIN_HEAP_SIZE
        ∨ size % MIN_HEAP_SIZE != 0
        <==> result is Err (for the kheap-level checks)
```

**Note**: Even if kheap-level checks pass, individual `Slab::from_raw_parts` calls can
still fail with `ErrorCode::InvalidArgument`. The `?` operator propagates these. However,
given the known constants (`slab_size = size / NUM_OF_SLABS` ≥ `MIN_SLAB_SIZE` = 131072
and the smallest block size is 8), these inner calls should always succeed when kheap
preconditions hold (see LIVE-1).

**Frame**: No mutation of pre-existing state (constructor).

---

### FN-3: `Kheap::allocate(&mut self, layout: Layout) -> Result<*mut u8, AllocError>`

**Purpose**: Allocate a block from the slab matching `layout.size()`.

**Preconditions**:
```
FN-3a: self.inv()
```

**Success postcondition** (`Ok(ptr)`):
```
FN-3b: let slab_idx = slab_for_size(layout.size());
        old(self)@.slabs[slab_idx].free_addrs.contains(ptr as usize)
        // The returned address was previously free in the correct slab.

FN-3c: ptr as usize % self@.slabs[slab_idx].block_size == 0
        // Returned pointer is block-aligned.

FN-3d: self@.slabs[slab_idx] == SlabView {
            allocated_addrs: old(self)@.slabs[slab_idx].allocated_addrs.insert(ptr as usize),
            free_addrs: old(self)@.slabs[slab_idx].free_addrs.remove(ptr as usize),
            ..old(self)@.slabs[slab_idx]
        }
        // Exactly one address moves from free to allocated in the target slab.

FN-3e: ∀ j: 0 <= j < NUM_OF_SLABS ∧ j != slab_idx ==>
            self@.slabs[j] == old(self)@.slabs[j]
        // Frame: all other slabs are untouched.

FN-3f: self.inv()
        // Invariant preserved.
```

**Error postcondition** (`Err(AllocError)`):
```
FN-3g: layout.size() == 0
        ∨ layout.size() > MAX_SLAB_SIZE
        ∨ (let idx = slab_for_size(layout.size());
           old(self)@.slabs[idx].free_addrs == Set::<usize>::empty())
        // Either the size is unsupported, or the matching slab is full.

FN-3h: self@ == old(self)@
        // State preserved on error.
```

---

### FN-4: `Kheap::deallocate(&mut self, ptr: *mut u8, layout: Layout) -> Result<(), AllocError>`

**Purpose**: Return a previously-allocated block to its slab.

**Preconditions**:
```
FN-4a: self.inv()
```

**Success postcondition** (`Ok(())`):
```
FN-4b: let slab_idx = slab_for_size(layout.size());
        old(self)@.slabs[slab_idx].allocated_addrs.contains(ptr as usize)
        // The pointer was indeed allocated in the slab determined by the layout.

FN-4c: self@.slabs[slab_idx] == SlabView {
            allocated_addrs: old(self)@.slabs[slab_idx].allocated_addrs.remove(ptr as usize),
            free_addrs: old(self)@.slabs[slab_idx].free_addrs.insert(ptr as usize),
            ..old(self)@.slabs[slab_idx]
        }
        // Exactly one address moves from allocated to free.

FN-4d: ∀ j: 0 <= j < NUM_OF_SLABS ∧ j != slab_idx ==>
            self@.slabs[j] == old(self)@.slabs[j]
        // Frame: all other slabs are untouched.

FN-4e: self.inv()
        // Invariant preserved.
```

**Error postcondition** (`Err(AllocError)`):
```
FN-4f: layout.size() == 0
        ∨ layout.size() > MAX_SLAB_SIZE
        ∨ (let idx = slab_for_size(layout.size());
           !old(self)@.slabs[idx].allocated_addrs.contains(ptr as usize))
        // Either unsupported size, or ptr was not allocated in that slab.

FN-4g: self@ == old(self)@
        // State preserved on error.
```

---

### FN-5: `ArenaAllocator::alloc(&self, layout: Layout) -> *mut u8`

**Purpose**: `GlobalAlloc` trait — allocate memory or return null.

**Preconditions**: `HEAP` has been initialized (i.e., `init()` was called successfully).

**Postconditions**:
```
FN-5a: HEAP is Some ∧ allocate succeeds ==> result != null
        ∧ result == allocated pointer from FN-3

FN-5b: HEAP is None ==> result == null

FN-5c: HEAP is Some ∧ allocate fails ==> result == null
```

**Note**: This is the `GlobalAlloc` interface. It cannot return `Result`; failures are
signalled via null pointer. Verification of this function requires modelling the global
`HEAP` state, which may be out of scope for a first pass.

---

### FN-6: `ArenaAllocator::dealloc(&self, ptr: *mut u8, layout: Layout)`

**Purpose**: `GlobalAlloc` trait — deallocate memory. Silently ignores errors.

**Preconditions**: `ptr` was previously returned by `alloc` with the same `layout`.

**Postconditions**:
```
FN-6a: HEAP is Some ∧ deallocation succeeds ==>
        heap state transitions per FN-4 (success case)

FN-6b: HEAP is None ==> no-op (silent)

FN-6c: HEAP is Some ∧ deallocation fails ==> state unchanged (error logged)
```

**Note**: `dealloc` silently drops errors. This is standard for `GlobalAlloc` but means
double-free or mismatched-layout bugs produce only log messages, not panics.

---

### FN-7: `init() -> Result<(), Error>`

**Purpose**: One-time initialization of the global kernel heap.

**Preconditions**:
```
FN-7a: HEAP == None
        // init() should only be called once. Calling it twice overwrites the heap,
        // leaking all prior allocations. (See BUG-1.)
```

**Success postcondition** (`Ok(())`):
```
FN-7b: HEAP == Some(kheap)
        ∧ kheap.inv()
        ∧ ∀ i: 0 <= i < NUM_OF_SLABS ==>
            kheap@.slabs[i].allocated_addrs == Set::<usize>::empty()
        // Heap is initialized with all slabs empty.

FN-7c: // The heap is backed by HEAP_STORAGE:
        let base = HEAP_STORAGE.memory.as_ptr() as usize;
        kheap@.slabs[0].start_addr >= base
        ∧ kheap@.slabs[NUM_OF_SLABS - 1].end_addr <= base + MIN_HEAP_SIZE
```

**Error postcondition** (`Err(e)`):
```
FN-7d: HEAP == None  // unchanged (the `?` returns before assignment)
        ∧ e.code == ErrorCode::InvalidArgument
```

**Note on `init()` success guarantee**: Given the known constants
(`HEAP_STORAGE` is page-aligned, `MIN_HEAP_SIZE == NUM_OF_SLABS * MIN_SLAB_SIZE`,
`MIN_SLAB_SIZE == 32 * 4096 = 131072`), the address and size checks in `from_raw_parts`
will always pass. See LIVE-2 for the argument that `init()` always succeeds.

---

## 5. Module-Level Safety Properties

### MOD-1: Cross-slab allocation disjointness

No address can be simultaneously allocated in two different slabs.

```
∀ i, j: 0 <= i < j < NUM_OF_SLABS ==>
    self@.slabs[i].allocated_addrs.disjoint(self@.slabs[j].allocated_addrs)
```

This follows from TYPE-2 (region disjointness) + `SlabView::inv()` (allocated addresses
lie within `[start_addr, end_addr)`).

### MOD-2: Cross-slab free-set disjointness

No address appears in the free sets of two different slabs (same argument as MOD-1).

```
∀ i, j: 0 <= i < j < NUM_OF_SLABS ==>
    self@.slabs[i].free_addrs.disjoint(self@.slabs[j].free_addrs)
```

### MOD-3: Allocation conservation

For each slab, the total number of tracked addresses (allocated + free) is constant
across allocate/deallocate operations. Addresses are never created or destroyed; they
only move between the allocated and free sets.

```
∀ i: 0 <= i < NUM_OF_SLABS ==>
    self@.slabs[i].allocated_addrs.union(self@.slabs[i].free_addrs)
    == old(self)@.slabs[i].allocated_addrs.union(old(self)@.slabs[i].free_addrs)
```

This follows directly from the Slab allocate/deallocate contracts (insert into one set,
remove from other).

### MOD-4: No allocation at address zero

No slab ever returns a null pointer. This relies on `HEAP_STORAGE` being a static
with non-zero address and all slab addresses lying within `[start_addr, end_addr)`.

```
∀ i: 0 <= i < NUM_OF_SLABS ==>
    ¬ self@.slabs[i].allocated_addrs.contains(0)
    ∧ ¬ self@.slabs[i].free_addrs.contains(0)
```

### MOD-5: Alignment guarantee

Every allocated pointer is aligned to the block size of its slab, which is ≥ the
allocation size requested. For allocations of types with alignment requirements
≤ their size, this is sufficient.

```
∀ ptr ∈ self@.slabs[i].allocated_addrs:
    ptr % self@.slabs[i].block_size == 0
```

**Caveat**: `layout_to_allocator` routes based on `layout.size()` only, ignoring
`layout.align()`. See BUG-2.

### MOD-6: Slab-to-layout routing consistency

The same layout always maps to the same slab. Allocating with a layout and deallocating
with the same layout targets the same slab. This is critical for correctness of
`deallocate` — if the routing were inconsistent, `deallocate` would try to free from
the wrong slab.

```
∀ layout: layout_to_allocator(layout) == layout_to_allocator(layout)
// (deterministic: trivially true for a pure function, but essential to state)
```

The deeper property is:
```
∀ layout, ptr:
    allocate(layout) == Ok(ptr)
    ==> deallocate(ptr, layout) targets the same slab
```

---

## 6. Liveness Properties

### LIVE-1: Slab construction feasibility

Given valid kheap-level preconditions, each individual `Slab::from_raw_parts` call
succeeds. This requires showing that the arguments to each inner call satisfy Slab's
preconditions.

For each slab `i`:
- `addr_i = addr + i * slab_size` where `slab_size = size / NUM_OF_SLABS`
- `len = slab_size`
- `block_size = BLOCK_SIZES[i]` (8, 16, ..., 512)

**Must show**:
```
addr_i != 0                    // addr >= PAGE_SIZE, slab_size > 0, so addr_i > 0
slab_size != 0                 // size >= MIN_HEAP_SIZE > 0, NUM_OF_SLABS > 0
slab_size < i32::MAX           // slab_size = 131072 << i32::MAX ✓
slab_size <= isize::MAX        // trivially ✓
addr_i + slab_size <= usize::MAX  // heap within address space
block_size != 0                // smallest is 8 ✓
block_size < i32::MAX          // largest is 512 (or 4096) ✓
slab_size >= block_size * 2    // 131072 >= 512*2 = 1024 ✓ (even 4096*2 = 8192 ✓)
addr_i % block_size == 0       // addr is PAGE_SIZE-aligned, slab_size is multiple of
                               // PAGE_SIZE, all block sizes divide PAGE_SIZE → ✓
```

### LIVE-2: `init()` always succeeds

Given the static configuration:
- `HEAP_STORAGE` is `#[repr(align(4096))]` → page-aligned ✓
- `HEAP_STORAGE.memory.len() == MIN_HEAP_SIZE` → `size == MIN_HEAP_SIZE >= MIN_HEAP_SIZE` ✓
- `MIN_HEAP_SIZE % MIN_HEAP_SIZE == 0` ✓

Combined with LIVE-1, `init()` always returns `Ok(())`.

```
init() is infallible given the static HEAP_STORAGE configuration.
```

This is a strong liveness result: the kernel will never panic at heap initialization.

### LIVE-3: Allocation succeeds when slab has free blocks

```
∀ layout: 1 <= layout.size() <= MAX_SLAB_SIZE
    ∧ let idx = slab_for_size(layout.size())
    ∧ self@.slabs[idx].free_addrs != Set::<usize>::empty()
    ==> allocate(layout) is Ok
```

### LIVE-4: Deallocation succeeds for previously-allocated pointer

```
∀ layout, ptr:
    1 <= layout.size() <= MAX_SLAB_SIZE
    ∧ let idx = slab_for_size(layout.size())
    ∧ self@.slabs[idx].allocated_addrs.contains(ptr as usize)
    ==> deallocate(ptr, layout) is Ok
```

### LIVE-5: Allocate-then-deallocate returns to prior state

Round-tripping an allocation restores the slab's abstract state:

```
∀ layout:
    let old_view = self@;
    allocate(layout) == Ok(ptr)  ∧  deallocate(ptr, layout) == Ok(())
    ==> self@ == old_view
```

---

## 7. Cross-Module Properties

### GLOBAL-1: Heap memory region exclusivity

The memory region `[HEAP_STORAGE.memory.as_ptr(), HEAP_STORAGE.memory.as_ptr() + MIN_HEAP_SIZE)`
must not overlap with any other memory region used by the kernel (stack, BSS globals,
MMIO regions, page tables, etc.).

This is an architectural property that cannot be verified within kheap alone — it
depends on the linker script and memory map. It should be stated as a documented
assumption.

### GLOBAL-2: Single initialization

`init()` is called exactly once, before any allocation. The caller (`kmain`) ensures
this sequencing. After `init()` returns `Ok`, `HEAP` is `Some` for all subsequent
`alloc`/`dealloc` calls.

### GLOBAL-3: No concurrent access

`HEAP` and `HEAP_STORAGE` are `static mut`. The kernel must ensure no concurrent access
(single-threaded initialization, or proper synchronization for multi-core). This is
an architectural invariant outside kheap's scope.

### GLOBAL-4: Layout consistency across alloc/dealloc

The `GlobalAlloc` contract requires that `dealloc` is called with the same `Layout` that
was passed to `alloc`. If the caller violates this, `layout_to_allocator` may route to
a different slab, causing a failed deallocation (the pointer won't be found in that
slab's allocated set). The Slab contract guarantees this returns `Err` rather than
corrupting state, so safety is preserved, but the memory is leaked.

---

## 8. Suspected Bugs

### BUG-1: Double initialization leaks allocations (Severity: Medium)

`init()` unconditionally overwrites `HEAP` with a fresh `Kheap`. If called twice,
all allocations from the first initialization are silently leaked — the old slabs'
tracking state is dropped.

**Current mitigation**: The single call site in `kmain` makes double-init unlikely.

**Recommendation**: Add a guard `if HEAP.is_some() { return Err(...) }` at the
start of `init()`, or verify via GLOBAL-2 that it is called at most once.

### BUG-2: Alignment not checked in `layout_to_allocator` (Severity: Low–Medium)

`layout_to_allocator` routes based on `layout.size()` only, ignoring `layout.align()`.
Since slab block sizes are powers of two and allocated addresses are block-aligned,
any type whose alignment requirement is ≤ its size will be correctly aligned. However,
a `Layout` with `size=4, align=16` would be routed to the 8-byte slab, returning an
8-byte-aligned pointer that does not satisfy the 16-byte alignment requirement.

**Impact**: In practice, Rust's `Layout::from_size_align` enforces `align <= size` for
most types, and the standard library typically ensures `align` is a power of two ≤ size.
Exotic layouts (e.g., SIMD types) could trigger this.

**Recommendation**: Either document the assumption that `layout.align() <= layout.size()`
(which holds for all standard Rust types) or add a routing check:
route to `max(size, align)` instead of just `size`.

### BUG-3: `dealloc` silently ignores errors (Severity: Low)

`ArenaAllocator::dealloc` logs but swallows deallocation errors. A double-free or
mismatched-layout deallocation produces only a log message. The underlying Slab
correctly rejects invalid deallocations (returning `Err` with state unchanged), so
there is no memory corruption, but the allocator provides no mechanism for callers
to detect the mistake.

**Mitigation**: This is inherent to the `GlobalAlloc` trait, which defines `dealloc`
as returning `()`. The behavior is correct per the trait contract. Consider adding
a debug assertion in debug builds.

### BUG-4: Zero-sized layout handling (Severity: Info)

`layout_to_allocator` returns `Err(AllocError)` for `layout.size() == 0` because
the match range starts at 1. In `GlobalAlloc::alloc`, this translates to a null
pointer return. Rust's `GlobalAlloc` documentation states that zero-sized allocations
have implementation-defined behavior, so this is technically acceptable. However,
higher-level allocator APIs (`Allocator` trait) require zero-sized allocations to
return a valid dangling pointer. If the kernel ever uses `Box<ZST>` through this
allocator path, this could surface.

---

## 9. Excluded Properties

| Property | Rationale |
|----------|-----------|
| Thread safety / lock correctness | Module uses `static mut` with no synchronization. Concurrency is excluded from kheap's scope (see GLOBAL-3). The kernel is assumed single-threaded during heap access or uses external synchronization. |
| Pointer provenance / strict aliasing | Verus does not currently model Rust's pointer provenance rules. We assume `addr as *mut u8` produces a valid pointer when `addr` is within `HEAP_STORAGE`. |
| `GlobalAlloc::alloc` / `dealloc` full specs | These wrap `Kheap::allocate`/`deallocate` with global state access (`static mut HEAP`). Full verification requires modelling global state transitions, which is deferred. The core logic is verified through the `Kheap` methods. |
| Logging side effects (`info!`, `error!`) | Logging macros are side effects that do not affect correctness. Excluded. |
| `HeapStorage` memory initialization | `HEAP_STORAGE` is zero-initialized. Slab does not rely on initial memory contents (it uses a bitmap index). No property needed. |
| Overflow in `heap_start_addr.add(i * slab_size)` | With `MIN_HEAP_SIZE` = 917504 (7 × 131072) or 1310720 (10 × 131072), and addresses within the kernel's virtual address space, overflow of `usize` is not realistic. A formal proof would need to verify `addr + NUM_OF_SLABS * slab_size <= usize::MAX`, which follows from `size` fitting in `usize` and `addr + size` not wrapping. |
| `SlabSize` enum exhaustiveness | The compiler guarantees match exhaustiveness. No verification property needed. |
| Specific numeric constants (KILOBYTE, MEGABYTE) | Used only in `info!` logging, not in correctness-critical computation. |
