# Property Analysis: `mm::kheap` — Kernel Heap Allocator

**Module**: `src/kernel/src/mm/kheap.rs`
**Domain**: Memory management — slab-based kernel heap allocator
**Verified dependency**: `Slab` (with `SlabView` abstract state, fully verified)
**Consolidated from**: independent analyses by Claude Opus 4.6 and GPT-5.3-Codex

---

## 1. Module Overview

The `kheap` module implements the kernel's dynamic memory allocator. It manages a
statically-allocated `HeapStorage` region (page-aligned, zero-initialized), partitions
it into `NUM_OF_SLABS` equally-sized arenas, and wraps each in a verified `Slab`
configured for a specific block size (8, 16, 32, 64, 128, 256, 512 bytes; extended to
1024, 2048, 4096 under the `hyperlight` feature flag).

**Public interface**:
- `init()` — one-time initialization from `kmain`
- `layout_to_allocator(layout)` — pure routing from `Layout` to slab tier
- `GlobalAlloc` trait impl (`alloc`/`dealloc`) — called by Rust's allocator infrastructure

**Key design properties**:
- Fixed-size slab allocator: each allocation is rounded up to the next power-of-two tier.
- Slabs occupy disjoint, contiguous sub-regions of a single page-aligned static buffer.
- `HEAP` is a global `Option<Kheap>` — `None` before `init()`, `Some` after.
- Module correctness reduces to correct class routing plus preservation of each `Slab`'s
  verified transition contracts.

**Architecture constants**:
- `PAGE_SIZE` = 4096, `SLAB_COUNT` = 32
- `MIN_SLAB_SIZE` = 32 × 4096 = 131072
- `NUM_OF_SLABS` = 7 (or 10 under hyperlight)
- `MIN_HEAP_SIZE` = 7 × 131072 = 917504 (or 10 × 131072 = 1310720)

**Verified dependency (`Slab`) contracts** (assumed proven):
- `SlabView` = `{ block_size, start_addr, end_addr, allocated_addrs, free_addrs }`
- `SlabView::inv()` guarantees block-alignment, range validity, allocated/free disjointness.
- `from_raw_parts`: On Ok → `slab.inv()`, block_size set, empty allocated set, region within input.
  On Err → `InvalidArgument`, at least one precondition violated.
- `allocate`: Requires `self.inv()`. Ok → address moves free→allocated, inv preserved.
  Err → free set empty, state unchanged.
- `deallocate`: Requires `self.inv()`. Ok → address moves allocated→free, inv preserved.
  Err → ptr not in allocated set, state unchanged.

---

## 2. Abstract State Design

### `KheapView` — Proposed Abstract State

```rust
pub struct KheapView {
    /// Abstract state of each slab, indexed by tier.
    /// slabs[0] = 8-byte slab, slabs[1] = 16-byte, ..., slabs[6] = 512-byte
    /// (hyperlight: slabs[7] = 1024, slabs[8] = 2048, slabs[9] = 4096)
    pub slabs: Seq<SlabView>,
}
```

**Design rationale**: Using `Seq<SlabView>` (length = `NUM_OF_SLABS`) enables quantified
properties over all slabs without enumerating each field. Concrete field-to-index mapping:

| Index | Block size | Field              |
|------:|-----------:|:-------------------|
|     0 |          8 | `slab_8_bytes`     |
|     1 |         16 | `slab_16_bytes`    |
|     2 |         32 | `slab_32_bytes`    |
|     3 |         64 | `slab_64_bytes`    |
|     4 |        128 | `slab_128_bytes`   |
|     5 |        256 | `slab_256_bytes`   |
|     6 |        512 | `slab_512_bytes`   |
|   7\* |       1024 | `slab_1024_bytes`  |
|   8\* |       2048 | `slab_2048_bytes`  |
|   9\* |       4096 | `slab_4096_bytes`  |

\* hyperlight-only

### Convenience spec functions on `KheapView`

```
spec fn inv(&self) -> bool
    // All TYPE-* properties hold

spec fn all_allocated(&self) -> Set<usize>
    // Union of allocated_addrs across all slabs

spec fn all_free(&self) -> Set<usize>
    // Union of free_addrs across all slabs

spec fn slab_for_size(size: int) -> Option<int>
    // Maps allocation size to slab index (mirrors layout_to_allocator)
    // Returns None for size == 0 or size > MAX_SLAB_SIZE

spec fn spec_allocate(self, slab_idx: int, addr: usize) -> KheapView
    // State transition: move addr from free to allocated in slabs[slab_idx]

spec fn spec_deallocate(self, slab_idx: int, addr: usize) -> KheapView
    // State transition: move addr from allocated to free in slabs[slab_idx]
```

---

## 3. Type Invariants

### TYPE-1: `KheapView` well-formedness

Each slab satisfies its own invariant, and the slab count matches configuration.

```
self.slabs.len() == NUM_OF_SLABS

∀ i: 0 <= i < self.slabs.len() ==>
    self.slabs[i].inv()
```

### TYPE-2: Slab region disjointness

No two slabs overlap in their address ranges. This is the central safety property —
without it, one slab's allocation could corrupt another's data.

Slabs are laid out contiguously in ascending order:

```
∀ i: 0 <= i < self.slabs.len() - 1 ==>
    self.slabs[i].end_addr <= self.slabs[i + 1].start_addr
```

### TYPE-3: Block-size sequence

Slab block sizes follow the expected power-of-two sequence with strict monotonic ordering,
ensuring `layout_to_allocator` routes to the tightest-fitting slab.

```
self.slabs[0].block_size == 8
self.slabs[1].block_size == 16
self.slabs[2].block_size == 32
self.slabs[3].block_size == 64
self.slabs[4].block_size == 128
self.slabs[5].block_size == 256
self.slabs[6].block_size == 512
// hyperlight: slabs[7..9] == 1024, 2048, 4096

∀ i: 0 <= i < self.slabs.len() - 1 ==>
    self.slabs[i].block_size < self.slabs[i + 1].block_size
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

Each `SlabSize` variant's discriminant equals the block size it represents. Used by
`layout_to_allocator` and `from_raw_parts` to pass correct block sizes to `Slab`.

```
SlabSize::Slab8 as usize == 8
SlabSize::Slab16 as usize == 16
SlabSize::Slab32 as usize == 32
SlabSize::Slab64 as usize == 64
SlabSize::Slab128 as usize == 128
SlabSize::Slab256 as usize == 256
SlabSize::Slab512 as usize == 512
// hyperlight: Slab1024 == 1024, Slab2048 == 2048, Slab4096 == 4096
```

### TYPE-6: `HeapStorage` alignment

The heap storage base address is page-aligned. This is established by `#[repr(align(4096))]`
and the static assertion, and is essential for slab alignment arguments.

```
HEAP_STORAGE.memory.as_ptr() as usize % PAGE_SIZE == 0
```

---

## 4. Function Contracts

### Preamble: Slab dependency contracts (assumed verified)

See §1 for a summary of `Slab::from_raw_parts`, `Slab::allocate`, `Slab::deallocate`
contracts. These are referenced throughout.

---

### FN-1: `Kheap::layout_to_allocator(layout: &Layout) -> Result<SlabSize, AllocError>`

**Purpose**: Pure routing function. Maps allocation size to the appropriate slab tier.

**Preconditions**: None (total function over all `Layout` values).

**Success postcondition** (`Ok(slab_size)`):
```
FN-1a: 1 <= layout.size() <= MAX_SLAB_SIZE  ==>  result is Ok

FN-1b: slab_size as usize >= layout.size()
        // Returned slab is large enough for the request.

FN-1c: slab_size is the smallest SlabSize variant whose value >= layout.size()
        // Tightest fit — no smaller tier could serve this size.
```

Where `MAX_SLAB_SIZE` = 512 (or 4096 under `hyperlight`).

**Error postcondition** (`Err(AllocError)`):
```
FN-1d: layout.size() == 0 ∨ layout.size() > MAX_SLAB_SIZE
        <==> result is Err
        // Bidirectional: error iff size is unsupported.
```

**Frame**: No state mutation (pure function). Deterministic: result depends only on
`layout.size()` and compile-time feature set.

---

### FN-2: `Kheap::from_raw_parts(addr: usize, size: usize) -> Result<Kheap, Error>`

**Purpose**: Construct a `Kheap` by partitioning a raw memory region into `NUM_OF_SLABS`
slabs of equal size.

**Preconditions**:
```
FN-2a: The memory region [addr, addr + size) is valid and exclusively owned.
        // Safety precondition — cannot be expressed in Verus without a memory
        // permission model. Documented as a comment/safety requirement.
```

**Success postcondition** (`Ok(heap)`):
```
FN-2b: heap.inv()
        // All TYPE-1 through TYPE-6 hold on the returned Kheap.

FN-2c: ∀ i: 0 <= i < NUM_OF_SLABS ==>
            heap@.slabs[i].allocated_addrs == Set::<usize>::empty()
        // All slabs start fully unallocated.

FN-2d: heap@.slabs[i].block_size == BLOCK_SIZES[i]
        // where BLOCK_SIZES = [8, 16, 32, 64, 128, 256, 512, ...]

FN-2e: let slab_size = size / NUM_OF_SLABS;
        ∀ i: 0 <= i < NUM_OF_SLABS ==>
            heap@.slabs[i].start_addr >= addr + i * slab_size
            ∧ heap@.slabs[i].end_addr <= addr + (i + 1) * slab_size
        // Each slab is contained within its designated partition.
```

**Error postcondition** (`Err(e)`):
```
FN-2f: e.code == ErrorCode::InvalidArgument

FN-2g: addr % PAGE_SIZE != 0
        ∨ size < MIN_HEAP_SIZE
        ∨ size % MIN_HEAP_SIZE != 0
        <==> result is Err  (for kheap-level checks)
        // Bidirectional failure condition.
```

**Note**: Even if kheap-level checks pass, individual `Slab::from_raw_parts` calls could
theoretically fail (propagated via `?`). However, given the known constants, these inner
calls always succeed when kheap preconditions hold (see LIVE-1).

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
        // Returned address was previously free in the correct slab.

FN-3c: ptr as usize % self@.slabs[slab_idx].block_size == 0
        // Returned pointer is block-aligned.

FN-3d: self@ == old(self)@.spec_allocate(slab_idx, ptr as usize)
        // Exact state transition: one address moves free→allocated in target slab,
        // all other slabs unchanged (frame included in spec_allocate).

FN-3e: self.inv()
        // Invariant preserved.
```

**Error postcondition** (`Err(AllocError)`):
```
FN-3f: layout.size() == 0
        ∨ layout.size() > MAX_SLAB_SIZE
        ∨ (let idx = slab_for_size(layout.size());
           old(self)@.slabs[idx].free_addrs == Set::<usize>::empty())
        // Either size unsupported, or matching slab is exhausted.

FN-3g: self@ == old(self)@
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
        // Pointer was allocated in the slab determined by the layout.

FN-4c: self@ == old(self)@.spec_deallocate(slab_idx, ptr as usize)
        // Exact state transition: one address moves allocated→free in target slab,
        // all other slabs unchanged (frame included in spec_deallocate).

FN-4d: self.inv()
        // Invariant preserved.
```

**Error postcondition** (`Err(AllocError)`):
```
FN-4e: layout.size() == 0
        ∨ layout.size() > MAX_SLAB_SIZE
        ∨ (let idx = slab_for_size(layout.size());
           !old(self)@.slabs[idx].allocated_addrs.contains(ptr as usize))
        // Either unsupported size, or ptr was not allocated in that slab.

FN-4f: self@ == old(self)@
        // State preserved on error.
```

---

### FN-5: `ArenaAllocator::alloc(&self, layout: Layout) -> *mut u8`

**Purpose**: `GlobalAlloc` trait — allocate memory or return null.

**Postconditions**:
```
FN-5a: HEAP is Some ∧ allocate succeeds ==> result != null_mut()
        ∧ HEAP state transitions per FN-3 success case

FN-5b: HEAP is None ==> result == null_mut()
        ∧ no state change

FN-5c: HEAP is Some ∧ allocate fails ==> result == null_mut()
        ∧ HEAP state unchanged
```

**Note**: Full verification requires modelling global `HEAP` state. The core logic
is verified through `Kheap::allocate`. The `GlobalAlloc` wrapper is a thin
delegation layer.

---

### FN-6: `ArenaAllocator::dealloc(&self, ptr: *mut u8, layout: Layout)`

**Purpose**: `GlobalAlloc` trait — deallocate memory. Silently logs errors.

**Postconditions**:
```
FN-6a: HEAP is Some ∧ deallocation succeeds ==>
        HEAP state transitions per FN-4 success case

FN-6b: HEAP is None ==> no-op

FN-6c: HEAP is Some ∧ deallocation fails ==> state unchanged (error logged only)
```

**Non-panicking**: This function must never panic regardless of inputs. Failures are
contained to logging and state preservation.

---

### FN-7: `init() -> Result<(), Error>`

**Purpose**: One-time initialization of the global kernel heap.

**Preconditions**:
```
FN-7a: HEAP == None
        // init() should only be called once. See BUG-1.
```

**Success postcondition** (`Ok(())`):
```
FN-7b: HEAP == Some(kheap)
        ∧ kheap.inv()
        ∧ ∀ i: 0 <= i < NUM_OF_SLABS ==>
            kheap@.slabs[i].allocated_addrs == Set::<usize>::empty()
        // Heap is initialized with all slabs empty.

FN-7c: let base = HEAP_STORAGE.memory.as_ptr() as usize;
        kheap@.slabs[0].start_addr >= base
        ∧ kheap@.slabs[NUM_OF_SLABS - 1].end_addr <= base + MIN_HEAP_SIZE
        // Heap is backed by HEAP_STORAGE.
```

**Error postcondition** (`Err(e)`):
```
FN-7d: HEAP is unchanged (the `?` returns before assignment)
        ∧ e.code == ErrorCode::InvalidArgument
```

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

No address appears in the free sets of two different slabs.

```
∀ i, j: 0 <= i < j < NUM_OF_SLABS ==>
    self@.slabs[i].free_addrs.disjoint(self@.slabs[j].free_addrs)
```

### MOD-3: Global allocation uniqueness

At any instant, a concrete address is allocated at most once globally (union across all slabs).

```
∀ i, j: 0 <= i < j < NUM_OF_SLABS ==>
    self@.slabs[i].allocated_addrs.disjoint(self@.slabs[j].allocated_addrs)
    ∧ self@.slabs[i].free_addrs.disjoint(self@.slabs[j].free_addrs)
    ∧ self@.slabs[i].allocated_addrs.disjoint(self@.slabs[j].free_addrs)
```

**Note**: MOD-1 and MOD-2 are subsumed by MOD-3, which is the strongest form. MOD-1 and MOD-2
are listed separately for clarity, but MOD-3 is the property to verify.

### MOD-4: No allocation at address zero

No slab ever contains address 0 (null). This relies on `HEAP_STORAGE` being a static
with non-zero address and all slab addresses lying within `[start_addr, end_addr)`.

```
∀ i: 0 <= i < NUM_OF_SLABS ==>
    ¬ self@.slabs[i].allocated_addrs.contains(0)
    ∧ ¬ self@.slabs[i].free_addrs.contains(0)
```

### MOD-5: Allocation conservation

For each slab, the union of allocated + free addresses is constant across
allocate/deallocate operations. Addresses are never created or destroyed.

```
∀ i: 0 <= i < NUM_OF_SLABS ==>
    self@.slabs[i].allocated_addrs.union(self@.slabs[i].free_addrs)
    == old(self)@.slabs[i].allocated_addrs.union(old(self)@.slabs[i].free_addrs)
```

### MOD-6: Slab-to-layout routing consistency

The same layout always maps to the same slab. Critical for correctness of deallocate —
if routing were inconsistent, deallocate would target the wrong slab.

```
∀ layout, ptr:
    allocate(layout) == Ok(ptr)
    ==> deallocate(ptr, layout) targets the same slab
```

Formally: `layout_to_allocator` is a pure deterministic function.

### MOD-7: Memory-region containment

All allocated pointers returned by this allocator lie within `HEAP_STORAGE.memory` bounds.

```
∀ ptr ∈ all_allocated():
    HEAP_STORAGE.memory.as_ptr() as usize <= ptr
    ∧ ptr < HEAP_STORAGE.memory.as_ptr() as usize + MIN_HEAP_SIZE
```

---

## 6. Liveness Properties

### LIVE-1: Slab construction feasibility

Given valid kheap-level preconditions, each individual `Slab::from_raw_parts` call
succeeds. For each slab `i` with `slab_size = size / NUM_OF_SLABS`:

```
addr_i = addr + i * slab_size
// Must satisfy all Slab::from_raw_parts preconditions:
addr_i != 0              // addr >= PAGE_SIZE, slab_size > 0 → addr_i > 0
slab_size != 0           // size >= MIN_HEAP_SIZE > 0
slab_size < i32::MAX     // 131072 < 2^31 ✓
slab_size <= isize::MAX  // trivially ✓
addr_i + slab_size ≤ usize::MAX  // addr + size fits in usize
block_size != 0          // smallest is 8
block_size < i32::MAX    // largest is 4096
slab_size >= block_size * 2  // 131072 >= 4096*2 ✓
addr_i % block_size == 0    // PAGE_SIZE-aligned + PAGE_SIZE-multiple offset, all
                              // block sizes divide PAGE_SIZE → aligned ✓
```

### LIVE-2: `init()` always succeeds

Given the static configuration:
- `HEAP_STORAGE` is `#[repr(align(4096))]` → page-aligned ✓
- `HEAP_STORAGE.memory.len() == MIN_HEAP_SIZE` → `size >= MIN_HEAP_SIZE` ✓
- `MIN_HEAP_SIZE % MIN_HEAP_SIZE == 0` ✓

Combined with LIVE-1, `init()` always returns `Ok(())`.

```
init() is infallible given the static HEAP_STORAGE configuration.
```

This is a strong liveness result: the kernel will never panic at heap initialization
(assuming HEAP_STORAGE is at a valid, non-zero address — which linker placement guarantees).

### LIVE-3: Allocation succeeds when slab has free blocks

```
∀ layout: 1 <= layout.size() <= MAX_SLAB_SIZE
    ∧ let idx = slab_for_size(layout.size())
    ∧ self@.slabs[idx].free_addrs != Set::<usize>::empty()
    ==> allocate(layout) returns Ok
```

### LIVE-4: Deallocation succeeds for previously-allocated pointer

```
∀ layout, ptr:
    1 <= layout.size() <= MAX_SLAB_SIZE
    ∧ let idx = slab_for_size(layout.size())
    ∧ self@.slabs[idx].allocated_addrs.contains(ptr as usize)
    ==> deallocate(ptr, layout) returns Ok
```

### LIVE-5: Allocate-then-deallocate round-trip

Round-tripping an allocation restores the slab's abstract state:

```
∀ layout:
    let old_view = self@;
    allocate(layout) == Ok(ptr)  ∧  deallocate(ptr, layout) == Ok(())
    ==> self@ == old_view
```

### LIVE-6: Failure recoverability

Allocation/deallocation failures preserve state, so subsequent valid operations remain
possible. Failures do not poison allocator state.

---

## 7. Cross-Module Properties

### GLOBAL-1: Heap memory region exclusivity

The memory region `[HEAP_STORAGE base, base + MIN_HEAP_SIZE)` must not overlap with
any other memory region used by the kernel (stack, BSS globals, MMIO, page tables).
This is an architectural property depending on linker script and memory map — cannot
be verified within kheap alone.

### GLOBAL-2: Single initialization / boot ordering

`init()` is called exactly once, before any allocation. The caller (`kmain`) ensures
this sequencing. After `init()` returns `Ok`, `HEAP` is `Some` for all subsequent
`alloc`/`dealloc` calls.

### GLOBAL-3: No concurrent access

`HEAP` and `HEAP_STORAGE` are `static mut`. The kernel must ensure no concurrent access
(single-threaded initialization, or proper synchronization for multi-core). This is an
architectural invariant outside kheap's scope.

### GLOBAL-4: Layout consistency across alloc/dealloc

The `GlobalAlloc` contract requires that `dealloc` is called with the same `Layout` that
was passed to `alloc`. If violated, `layout_to_allocator` may route to a different slab,
causing deallocation failure (state preserved, but memory leaked). The Slab contract
guarantees no corruption on failed deallocation.

### GLOBAL-5: Architecture-constant coupling

Correctness depends on `PAGE_SIZE` being 4096 and all slab block sizes being powers of
two that divide `PAGE_SIZE`. The partition and alignment guarantees are tied to these
constants.

---

## 8. Suspected Bugs

### BUG-1: Double initialization leaks allocations (Severity: Medium)

`init()` unconditionally overwrites `HEAP` with a fresh `Kheap`. If called twice, all
allocations from the first initialization are silently leaked — the old slabs' tracking
state is dropped.

**Current mitigation**: The single call site in `kmain` makes double-init unlikely.

**Recommendation**: Verify via GLOBAL-2 (requires precondition `HEAP == None`) that init
is called at most once, or add a runtime guard.

### BUG-2: Alignment not checked in `layout_to_allocator` (Severity: Low–Medium)

`layout_to_allocator` routes based on `layout.size()` only, ignoring `layout.align()`.
Since slab block sizes are powers of two and allocated addresses are block-aligned,
any type whose alignment ≤ its size will be correctly aligned. However, a `Layout` with
`size=4, align=16` would be routed to the 8-byte slab, returning an 8-byte-aligned
pointer that does not satisfy the 16-byte alignment requirement.

**Impact**: In practice, Rust's standard types have `align <= size`. Exotic layouts
(SIMD types with `repr(align(N))` where N > size) could trigger this.

**Recommendation**: Document the assumption that `layout.align() <= layout.size()`,
or route based on `max(size, align)`.

### BUG-3: `dealloc` silently ignores errors (Severity: Low)

`ArenaAllocator::dealloc` logs but swallows deallocation errors. A double-free or
mismatched-layout deallocation produces only a log message. The underlying Slab correctly
rejects invalid deallocations (state unchanged), so no memory corruption occurs, but the
memory is permanently leaked.

**Mitigation**: This is inherent to the `GlobalAlloc` trait, which defines `dealloc` as
returning `()`. The behavior is correct per the trait contract.

### BUG-4: Zero-sized layout handling (Severity: Info)

`layout_to_allocator` returns `Err(AllocError)` for `layout.size() == 0` because the
match range starts at 1. In `GlobalAlloc::alloc`, this translates to null pointer return.
Rust's `GlobalAlloc` documentation states zero-sized allocations have implementation-defined
behavior, so this is technically acceptable. However, the `Allocator` trait requires ZST
allocations to return a valid dangling pointer. If the kernel uses `Box<ZST>` through this
path, this could surface.

### BUG-5: Data-race risk on `static mut HEAP` (Severity: High — architectural)

`static mut HEAP` is accessed in `alloc`/`dealloc`/`init` without visible locking or
interrupt exclusion. Concurrent access violates Rust aliasing guarantees. This is an
architectural concern (see GLOBAL-3) — the module assumes single-threaded access or
external synchronization.

---

## 9. Excluded Properties

| Property | Rationale |
|----------|-----------|
| Thread safety / lock correctness | Module uses `static mut` with no synchronization. Concurrency excluded from kheap's scope (see GLOBAL-3). |
| Pointer provenance / strict aliasing | Verus does not model Rust's pointer provenance rules. We assume `addr as *mut u8` produces a valid pointer when `addr` is within `HEAP_STORAGE`. |
| `GlobalAlloc::alloc`/`dealloc` full specs | These wrap `Kheap::allocate`/`deallocate` with global state access. Full verification requires modelling global mutable state transitions, which may be deferred. Core logic verified through `Kheap` methods. |
| Logging side effects (`info!`, `error!`) | Non-functional observability, not part of allocator correctness. |
| `HeapStorage` memory initialization | Zero-initialized by default. Slab does not rely on initial memory contents (uses bitmap index). |
| Overflow in `heap_start_addr.add(i * slab_size)` | Delegated to `Slab::from_raw_parts` constraints plus known constant bounds. See LIVE-1 for the argument. |
| `SlabSize` enum match exhaustiveness | Compiler-guaranteed. |
| Constants used only in logging (`KILOBYTE`, `MEGABYTE`) | Not correctness-critical. |
| Physical memory / cache / TLB behavior | Outside functional model. |
| Allocation fairness across classes | Module guarantees per-class success when blocks are free, not fairness. |

---

## API Contracts for Review

The following external-top specs define the module's public promises. A human should
confirm these capture the intended behavior:

| Property ID | Description |
|-------------|-------------|
| **TYPE-1** | `KheapView` well-formedness — every slab satisfies `SlabView::inv()`, count = `NUM_OF_SLABS` |
| **TYPE-2** | Slab region disjointness — contiguous, non-overlapping layout |
| **TYPE-3** | Block-size sequence — correct power-of-two assignment per tier |
| **FN-1a–d** | `layout_to_allocator` — bidirectional size→tier routing contract |
| **FN-2b–g** | `from_raw_parts` — constructor success/error postconditions |
| **FN-3b–g** | `allocate` — state transition on success, preservation on error |
| **FN-4b–f** | `deallocate` — state transition on success, preservation on error |
| **FN-7b–d** | `init` — global HEAP initialization postcondition |
| **LIVE-2** | `init()` infallibility from static configuration |
| **MOD-3** | Global allocation uniqueness across all slabs |

---

## Needed Assumptions

External-bottom trust boundaries — operations lacking Verus-native specs:

- [ ] `ptr::add` — pointer arithmetic (`heap_start_addr.add(i * slab_size)`) — the slab crate already has an `assume_specification` for `<*mut T>::add`; kheap may reuse it
- [x] `Error::new` — external error constructor, no Verus spec
- [x] `Layout::size` — `core::alloc::Layout::size()` accessor, no Verus spec
- [ ] `usize::is_multiple_of` — standard library method, no Verus spec
- [ ] `ptr::addr_of_mut!` — raw pointer creation macro, no Verus spec
- [x] `core::ptr::null_mut` — null pointer constructor, needs spec that `result as usize == 0`
- [x] `HEAP_STORAGE.memory.as_ptr()` — static array pointer, needs spec for address value
- [x] `HEAP_STORAGE.memory.len()` — static array length, needs spec that `result == MIN_HEAP_SIZE`
- [x] `GlobalAlloc` trait dispatch — how Rust runtime calls `alloc`/`dealloc`

Human: `Layout` and `AllocError` need to use `assume_specificaton`. Then other functions can be verified and must be verified. `is_multiple_of` does have a verus assume_specification in vstd, you should first discover if anything is supported in vstd before claiming they are not supported. You don't need assume_specification for `ptr::add`, you can search vstd for raw pointer support to bypass directly using it.
