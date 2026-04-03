# Property Analysis: `kheap.rs`

## 1. Module Overview

`kheap.rs` implements the kernel global allocator as a fixed arena partitioned into size-class slabs. Initialization (`init`) builds one `Kheap` over statically reserved aligned memory. Allocation/deallocation dispatches by request size to the corresponding `Slab`. Safety relies on slab-local invariants plus global assumptions about initialization order and synchronization.

---

## 2. Abstract State Design (`KheapView`)

A caller-facing abstract model should expose only allocation-relevant state:

```text
KheapView {
  initialized: bool,
  heap_start: usize,
  heap_size: usize,
  slab_views: map<SlabSize, SlabView>,
  // derived:
  allocated_union: Set<usize>,
  free_union: Set<usize>
}
```

### Required abstraction constraints

- `initialized == false` iff global `HEAP == None`.
- If initialized, every slab view satisfies `SlabView::inv()`.
- Slab regions are contiguous, non-overlapping, and cover exactly `[heap_start, heap_start + heap_size)` in `NUM_OF_SLABS` equal chunks.
- `allocated_union` is the disjoint union of per-slab `allocated_addrs`; similarly for `free_union`.
- Any pointer returned by allocator belongs to exactly one slab and appears in exactly one `allocated_addrs` set.

---

## 3. Type Invariants (TYPE-N)

### TYPE-1 (`HeapStorage` alignment and size)
`HeapStorage` is page-aligned and has exactly `MIN_HEAP_SIZE` bytes.

- Concrete: `repr(align(4096))` and static assert align with `mem::PAGE_SIZE`.
- Purpose: makes base address valid for all slab alignments up to page size.

### TYPE-2 (`SlabSize` domain)
`SlabSize` variants are positive powers of two and match supported size classes.

- Non-hyperlight: `{8,16,32,64,128,256,512}`.
- Hyperlight adds `{1024,2048,4096}`.

### TYPE-3 (`Kheap` structural well-formedness)
For initialized `Kheap`, each slab field is individually invariant-preserving (`slab.inv()`) and bound to the expected `block_size` for its field.

### TYPE-4 (Slab region partition)
All slab subregions are pairwise disjoint and ordered by offset `i * slab_size`; each lies within `[addr, addr + size)`.

### TYPE-5 (Per-slab address-class discipline)
For each slab field `S_k`, all addresses in `S_k.allocated_addrs ∪ S_k.free_addrs` are multiples of that slab’s class size.

### TYPE-6 (Global uniqueness)
No address appears in allocated/free sets of two different slab fields simultaneously.

### TYPE-7 (`HEAP` global state coherence)
`HEAP == None` means allocator uninitialized; `HEAP == Some(h)` implies `h` satisfies TYPE-3..TYPE-6.

---

## 4. Function Contracts (FN-N)

## `Kheap::from_raw_parts(addr, size) -> Result<Kheap, Error>`

### FN-1 (preconditions for successful construction)
Success requires:
- `addr % PAGE_SIZE == 0`
- `size >= MIN_HEAP_SIZE`
- `size % MIN_HEAP_SIZE == 0`
- plus all `Slab::from_raw_parts` preconditions for each class/offset.

### FN-2 (success postcondition: full heap construction)
On `Ok(kheap)`:
- every slab field satisfies `inv()`.
- each field block size matches its class value.
- slab `i` uses base `addr + i*slab_size`, `slab_size = size/NUM_OF_SLABS`.
- every slab starts with empty `allocated_addrs`.

### FN-3 (success frame/coverage)
On success, created slab regions are within `[addr, addr + size)`, pairwise non-overlapping, and together cover exactly that range.

### FN-4 (error postcondition)
On `Err(e)`, `e.code == InvalidArgument` and no global allocator state changes (constructor is pure w.r.t. globals).

### FN-5 (error completeness)
If any explicit precheck fails, function must return `Err(InvalidArgument)`.
If prechecks pass but some slab construction fails, return the propagated slab `InvalidArgument` error.

## `Kheap::layout_to_allocator(layout) -> Result<SlabSize, AllocError>`

### FN-6 (success classification)
If `layout.size()` is in a supported interval, returned class is the unique minimal supported class with `class_size >= layout.size()`.

### FN-7 (error classification)
If `layout.size()` is outside supported range (`0` or `> max_class`), returns `Err(AllocError)`.

### FN-8 (determinism/purity)
Result depends only on `layout.size()` and compile-time feature set; no state mutation.

## `Kheap::allocate(&mut self, layout) -> Result<*mut u8, AllocError>`

### FN-9 (precondition)
`self` satisfies `Kheap` invariants; requested layout must be representable by some class for success.

### FN-10 (success postcondition)
On `Ok(ptr)`:
- selected slab class equals `layout_to_allocator(layout)`.
- pointer belonged to selected slab’s old `free_addrs` and moves to its `allocated_addrs`.
- all non-selected slabs unchanged.

### FN-11 (error postcondition: unsupported layout)
If class mapping fails, returns `Err(AllocError)` and all slabs unchanged.

### FN-12 (error postcondition: exhaustion)
If class maps but selected slab has empty `free_addrs`, returns `Err(AllocError)` and whole `Kheap` abstract state unchanged.

## `Kheap::deallocate(&mut self, ptr, layout) -> Result<(), AllocError>`

### FN-13 (precondition)
`self` invariant holds; caller provides layout used for class dispatch.

### FN-14 (success postcondition)
On `Ok(())`:
- selected slab contained `ptr` in old `allocated_addrs`.
- `ptr` removed from selected slab allocated set and inserted into its free set.
- all other slabs unchanged.

### FN-15 (error postcondition: unsupported layout)
If class mapping fails, returns `Err(AllocError)` with no state change.

### FN-16 (error postcondition: pointer not allocated in selected slab)
If selected slab rejects `ptr`, returns `Err(AllocError)` and entire `Kheap` unchanged.

## `GlobalAlloc for ArenaAllocator::alloc(&self, layout) -> *mut u8`

### FN-17 (success behavior)
If `HEAP` is initialized and `heap.allocate(layout)` succeeds, returns that non-null pointer and updates `HEAP` accordingly.

### FN-18 (error behavior)
Returns null and preserves `HEAP` abstract state when:
- `HEAP == None`, or
- class unsupported, or
- selected slab exhausted.

### FN-19 (frame)
No mutation outside `HEAP` state (ignoring logging side effects).

## `GlobalAlloc for ArenaAllocator::dealloc(&self, ptr, layout)`

### FN-20 (success behavior)
If `HEAP` initialized and deallocation succeeds, corresponding slab state is updated exactly per slab contract.

### FN-21 (error behavior)
If `HEAP` uninitialized or deallocation fails, no allocator state mutation occurs.

### FN-22 (non-panicking)
Function must not panic; failures are contained to logging and state preservation.

## `init() -> Result<(), Error>`

### FN-23 (success postcondition)
On `Ok(())`, `HEAP == Some(kheap)` where `kheap` is constructed from full `HEAP_STORAGE` range and satisfies all `Kheap` invariants.

### FN-24 (error postcondition)
On `Err(e)`, `e.code == InvalidArgument` (from `from_raw_parts`) and previous `HEAP` value is preserved.

### FN-25 (single-init expectation)
Module contract should state intended usage: called once during boot before any allocation traffic.

---

## 5. Module-Level Safety (MOD-N)

### MOD-1 (memory-region containment)
All allocated pointers returned by this allocator lie within `HEAP_STORAGE.memory` bounds.

### MOD-2 (no cross-slab overlap)
Two allocations from different classes can never alias due to disjoint slab regions.

### MOD-3 (allocation uniqueness)
At any instant, a concrete address is allocated at most once globally.

### MOD-4 (deallocation soundness boundary)
Only pointers currently allocated in the selected slab can be freed successfully; invalid/double free attempts do not mutate state.

### MOD-5 (state conservation per slab)
For each slab, `allocated_addrs ∩ free_addrs == ∅`, and membership only moves between sets; addresses are never fabricated.

### MOD-6 (unsafe-global synchronization requirement)
Because `HEAP` is `static mut`, module safety requires external synchronization/serialization for concurrent `alloc/dealloc/init` calls.

### MOD-7 (init-before-use safety requirement)
System safety requires `init()` completes successfully before any allocator call expecting non-null allocations.

---

## 6. Liveness (LIVE-N)

### LIVE-1 (constructability)
`init()` is guaranteed to succeed given current constants and storage (`HEAP_STORAGE` page-aligned, size exactly `MIN_HEAP_SIZE`) and valid slab constructor assumptions.

### LIVE-2 (allocation progress under availability)
For any supported class, if its slab `free_addrs` is non-empty, `allocate(layout_for_class)` succeeds.

### LIVE-3 (reclaimability)
After successful `deallocate(ptr, matching_layout)`, that address becomes available for future successful allocations in that class.

### LIVE-4 (failure recoverability)
Allocation/deallocation failures preserve state, so subsequent valid operations remain possible; failures do not poison allocator state.

### LIVE-5 (boot fail-fast)
If heap init fails, caller (`kmain`) panics immediately; system does not continue with partially initialized allocator.

---

## 7. Cross-Module Properties (GLOBAL-N)

### GLOBAL-1 (dependency on `Slab` correctness)
`Kheap` functional correctness reduces to correct class routing plus preservation of each `Slab`’s verified transition contracts.

### GLOBAL-2 (error code contract propagation)
`Kheap::from_raw_parts` maps setup violations to `ErrorCode::InvalidArgument`, preserving kernel-wide error semantics.

### GLOBAL-3 (global allocator contract)
`ArenaAllocator` must satisfy Rust global allocator expectation: null indicates allocation failure; dealloc must be non-panicking and tolerant to allocator-internal failure.

### GLOBAL-4 (boot-order integration)
`kmain` establishes one-time init ordering (`init` before normal runtime allocations); this ordering is part of allocator correctness assumptions.

### GLOBAL-5 (architecture-constant coupling)
Correctness depends on `PAGE_SIZE` and power-of-two slab sizes; alignment and partition guarantees are tied to those constants.

---

## 8. Suspected Bugs

1. **Alignment bug risk (high severity):** `layout_to_allocator` ignores `layout.align()`. A request with small size but large alignment may be routed to a slab whose block alignment is insufficient, violating `Layout` requirements and potentially causing UB.
2. **Data-race/aliasing risk (high severity):** `static mut HEAP` accessed in `alloc/dealloc/init` without visible locking/interrupt exclusion. Concurrent access can violate Rust aliasing guarantees and corrupt allocator state.
3. **Re-initialization hazard (medium):** `init()` unconditionally overwrites `HEAP`. A second call would discard metadata for outstanding allocations, enabling leaks/inconsistency.
4. **Silent dealloc drop when uninitialized (low-medium):** `dealloc` does nothing if `HEAP` is `None`; this hides lifecycle/order bugs.
5. **Layout-dependent dealloc mismatch (medium):** wrong `layout` routes pointer to wrong slab and fails (state preserved) but leaks allocation; no hard failure signal to caller beyond log.

---

## 9. Excluded Properties (with rationale)

1. **EXCL-1: Log message contents/order.** Logging is non-functional observability; not part of allocator safety/correctness.
2. **EXCL-2: Physical memory/cache/TLB behavior.** Outside module’s abstract contract and Verus-level functional model.
3. **EXCL-3: Precise allocation fairness across classes.** Module guarantees per-class success under free blocks, not fairness/scheduling among request streams.
4. **EXCL-4: Integer-overflow impossibility for every pointer arithmetic expression.** Relevant low-level checks are delegated to verified `Slab::from_raw_parts` constraints plus constant bounds; duplicate proof here is redundant.
5. **EXCL-5: Behavior for API misuse outside GlobalAlloc contract (e.g., arbitrary invalid pointer provenance).** Module specifies state-preserving failure where detectable; full UB model for illegal caller behavior is outside this module-level spec scope.
