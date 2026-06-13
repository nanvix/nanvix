# Caller Analysis: `bump_allocator` (`src/libs/bump_allocator/src/lib.rs`)

## Script Output

Raw `find_callers_lsp.py` output saved to:
`verus-ai-logs/nanvix-phys-bump-allocator/find_callers_lsp_output.md`

**Script summary:** crate `bump-allocator`, "Depended on by: `kernel`".
12 exec functions (9 pub/trait-pub, 3 private), 8 types.

> ⚠️ **Script limitation — cross-crate misses.** The LSP run resolved only
> *intra-crate* references (the module's own `#[cfg(test)]` callers) and reported
> **0 external callers** for every public item. The kernel crate is a separate
> `no_std` build target that rust-analyzer did not fully resolve into the call
> graph, so all real consumers — which live in `src/kernel/src/mm/virt/` — were
> missed by the automated tool. These were recovered by manual `grep` + source
> reading and are documented below. The script *did* correctly recover the
> internal call graph (`alloc_as` → `alloc` → `align_up`).

### Verified external callers (manual, kernel crate)

| Target item | External call sites |
|-------------|---------------------|
| `align_up` | `mm/virt/page_table_allocator.rs:49` (const `PAGE_TABLE_SLOT_STRIDE`) |
| `FixedSizeBumpAllocator::new` | `mm/virt/page_table_allocator.rs:103` (static `PAGE_TABLE_ALLOCATOR`) |
| `FixedSizeBumpAllocator::alloc_as` | `mm/virt/boot_init.rs:129`, `boot_init.rs:161`, `identity_map.rs:518`, `vmem.rs:111` |
| `FixedSizeBumpAllocator::alloc` | none outside crate (only invoked internally by `alloc_as`; unit tests use it directly) |
| `BssStorage::as_mut_ptr` | implemented by `PageTableBss` at `page_table_allocator.rs:90`; invoked internally by `alloc` (`lib.rs:272`) |

All four `alloc_as` sites use the same concrete type:
`alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()`, i.e. one page-table-sized slot.

## Trait Obligations

- **Trait `BssStorage` (unsafe).** Implemented once outside the crate by
  `PageTableBss` (`page_table_allocator.rs`).
  - `as_mut_ptr() -> *mut u8` — expected semantics: returns a **stable**,
    `STORAGE_SIZE`-byte, writable, `A`-aligned base pointer to a region
    *exclusively* owned by the single allocator instance. The kernel impl returns
    `addr_of_mut!(PAGE_TABLE_STORAGE.bytes)` over a `#[repr(align(4096))]` static.
    The allocator relies on this pointer being constant across calls and the
    region being large enough that `base + idx*stride + N <= base + STORAGE_SIZE`.
  - `NUM_UNITS` / `STORAGE_SIZE` consts — caller picks them so that
    `NUM_UNITS * align_up(N, A) == STORAGE_SIZE`. `alloc` treats `NUM_UNITS` as the
    exhaustion bound and `STORAGE_SIZE` as the hard out-of-bounds bound.
- **`Default for FixedSizeBumpAllocator`** — delegates to `new()`; no external
  caller (kernel uses `new()` directly). Carries the same singleton safety duty.
- **`Display for BumpAllocError`** — used implicitly: kernel callers format the
  error in `error!("... {}", e)` log lines before mapping it to a kernel `Error`.

## Caller Expectations

### `align_up(value, alignment) -> Option<usize>`
- **Callers assume:** pure, `const`-evaluable, total. Returns the least multiple
  of `alignment` that is `>= value`; `None` only on `alignment == 0` or multiply
  overflow. The kernel uses it in a `const` context to derive a slot stride and
  `panic!`s on `None`, so it must be usable at compile time and never panic
  itself. For already-aligned `value`, result `== value`.
- **Callers don't care about:** the internal use of `div_ceil`/`checked_mul`, or
  any particular representation — only the numeric result and the `None` overflow
  signal.

### `FixedSizeBumpAllocator::new() -> Self` (const unsafe)
- **Callers assume:** produces a fresh allocator with the bump index at 0, usable
  in `static` initialization (`const`), with the documented singleton precondition
  (exactly one instance per `S` backend). The kernel constructs exactly one global
  `PAGE_TABLE_ALLOCATOR`.
- **Callers don't care about:** the `AtomicUsize`/`PhantomData` internals.

### `FixedSizeBumpAllocator::alloc(&self) -> Result<&'static mut [u8; N], BumpAllocError>`
- **No direct external callers** (only the internal `alloc_as` and unit tests),
  but its contract is inherited by `alloc_as`.
- **Callers (via `alloc_as`) assume on success:** a `'static`, unique,
  non-aliasing, `A`-aligned, `N`-byte slot; distinct from every previously
  returned slot (the unit test `alloc_returns_distinct_slots` asserts pointer
  inequality). Lock-free / safe under concurrency.
- **Callers assume on failure:** no slot is consumed observably beyond the
  reserved index; the error variant tells why — `Exhausted` once `NUM_UNITS`
  slots are gone (asserted by `alloc_returns_exhausted_error`), or
  `Overflow`/`OutOfBounds`/`Misaligned` for arithmetic/bounds/alignment faults.
- **Callers don't care about:** the CAS retry loop, the exact bump index, or how
  the address is computed — only that slots are unique, in-bounds, and aligned.

### `FixedSizeBumpAllocator::alloc_as<T>(&self) -> Result<&'static mut MaybeUninit<T>, BumpAllocError>` (unsafe)
- **Callers assume on success:** a `'static mut MaybeUninit<T>` backing a unique,
  correctly sized (`size_of::<T>() == N`) and aligned (`align_of::<T>() <= A`)
  slot. Kernel callers immediately `.assume_init_mut()` — sound *only because* the
  backing static BSS is zero-initialized and `T` here is an integer array
  (`[PteWord; PAGE_TABLE_LENGTH]`), for which all-zero is a valid value. They then
  treat the result as a live page table / page directory.
- **Callers assume on failure:** a `BumpAllocError` they can log via `Display` and
  map to a kernel `Error` (`OutOfMemory`), with no memory handed out. Specifically
  they rely on `SizeMismatch`/`AlignmentMismatch` guarding the unchecked cast, plus
  the propagated `alloc` errors.
- **Callers don't care about:** that it is implemented as `alloc()` + a pointer
  cast, nor the `MaybeUninit` reinterpretation details — only that the returned
  reference points at a fresh, exclusively owned, properly sized/aligned `T`-slot.

### `BssStorage::as_mut_ptr() -> *mut u8` (caller-implemented)
- **The module (allocator) assumes:** the pointer is stable across calls, points
  to `>= STORAGE_SIZE` writable bytes, and is aligned to at least `A`. `alloc`
  re-reads it on every call and computes all slot addresses relative to it.
- **The implementor (kernel) assumes:** the allocator never writes through the
  pointer itself and only derives non-overlapping slot sub-ranges from it.

## Abstract Resource

A **fixed-capacity pool of `NUM_UNITS` equal-sized (`N`-byte, `A`-aligned) memory
slots** carved from a single statically reserved BSS region. The allocator is a
monotonic, lock-free *bump cursor* over that pool that hands out each slot at most
once as a unique `'static mut` reference, and reports `Exhausted` when the pool is
empty. Callers (the kernel page-table subsystem) use it as the source of page-table
and page-directory frames during and after boot.

## Key Invariants (caller perspective)

- **Uniqueness / non-aliasing:** every successful allocation returns a pointer
  distinct from all prior successful allocations; no two live `&'static mut` slots
  overlap. (Foundational to the `unsafe` soundness the kernel relies on.)
- **In-bounds:** every returned slot lies fully within `[base, base + STORAGE_SIZE)`.
- **Alignment:** every returned slot is aligned to `A` (and, for `alloc_as`,
  `align_of::<T>() <= A` is enforced before the cast).
- **Monotone capacity:** at most `NUM_UNITS` successful allocations ever; the
  `(NUM_UNITS+1)`-th and beyond return `Exhausted`. The bump index never moves
  backward.
- **Stable size contract:** `alloc_as::<T>` succeeds iff `size_of::<T>() == N` and
  `align_of::<T>() <= A`; otherwise it fails *before* touching storage.
- **No spurious consumption on error:** size/alignment mismatches and overflow do
  not yield a usable-but-invalid slot.

## Pre-existing Specs (from upstream verification)

- `lib.spec.rs` and `lib.proof.rs` exist but contain only empty `verus! { }`
  blocks — **no `#[verus_spec]` annotations, no `View` type, no ensures/requires**
  on any target function yet.
- View type: **does not exist.**
- **Assessment:** clean slate. The View should be designed from the caller
  expectations above (abstract pool of unique, in-bounds, aligned slots with a
  monotone allocation count), not by mirroring the `AtomicUsize` cursor.

## Verification-Order Target Functions

In scope: `FixedSizeBumpAllocator::alloc_as`, `FixedSizeBumpAllocator::alloc`,
`align_up`, `as_mut_ptr` (the `BssStorage` trait method). Do not modify other
functions.
