# Caller Analysis: `hal/mem/types/region` (`region.rs`)

## Scope

Verification-order target functions (only functions in scope for this module):

- `TruncatedMemoryRegion::start`
- `MemoryRegion::start`
- `TruncatedMemoryRegion::size`
- `MemoryRegion::size`

All four are pure, side-effect-free getters that project a stored field of an
immutable memory-region value. They feed the View design for `MemoryRegion<T>`
and `TruncatedMemoryRegion<T>`.

## Script Output

See: `verus-ai-logs/nanvix-phys-hal-memory-region/caller-analysis/find_callers_output.md`
(full `find_callers_lsp.py` report; intra-crate LSP analysis — no cross-crate
dependents because `kernel` is a leaf binary crate).

## Trait Obligations

None of the four target functions are trait methods; they are inherent getters.
For context, the enclosing types implement `Ord`/`PartialOrd` keyed solely on
`start` (`MemoryRegion::cmp` → `self.start.cmp(&other.start)`), so `start` also
implicitly backs the ordering used when regions are stored in sorted/searchable
collections. This does not add obligations to `start` beyond "returns the stored
start deterministically", but it means the View's `start` field must be the
ordering key.

## Caller Map (target functions)

### `TruncatedMemoryRegion::start` — 4 external + 1 internal call site
- `hal/io/mmio/allocator.rs:189` — `region.start().into_raw_value()` → raw base
  used as the low bound of an inclusive overlap range.
- `hal/io/mmio/allocator.rs:193` — same pattern for an already-registered entry.
- `hal/io/mmio/region.rs:75` — `Mmio::base()` returns `self.region.start()`
  verbatim as the public page-aligned base address.
- `mm/phys/frame.rs:588` — `region.start().into_frame_number().into_raw_value()`
  → start frame number for booking a physical frame range.
- internal: `TruncatedMemoryRegion::fmt` (Debug formatting).

### `TruncatedMemoryRegion::size` — 4 external + 1 internal call site
- `hal/io/mmio/allocator.rs:190` — `compute_inclusive_end(start, region.size())`
  → high bound of the inclusive overlap range.
- `hal/io/mmio/allocator.rs:194` — same for an existing entry.
- `hal/io/mmio/region.rs:103` — `Mmio::size()` returns `self.region.size()`
  verbatim (bytes).
- `mm/phys/frame.rs:589` — `start_frame_number + region.size() / FRAME_SIZE - 1`
  → end frame number. **Divides size by `FRAME_SIZE` and expects an exact frame
  count**, i.e. relies on `size` being a non-zero multiple of the page/frame size.
- internal: `TruncatedMemoryRegion::fmt`.

### `MemoryRegion::start` — 0 external, 3 internal call sites
- `from_memory_region` (L345) — `region.start().align_down(PAGE_ALIGNMENT)?` →
  page-aligned start for the truncated region.
- `TruncatedMemoryRegion::start` (L364) — delegates `self.0.start()`.
- `from_virtual_memory_region` (L441) — `PhysicalAddress::from_virtual_address(region.start())`
  → translate the (possibly unaligned) start, then page-align.

### `MemoryRegion::size` — 0 external, 2 internal call sites
- `from_memory_region` (L348) — `let size = region.size();` carried into the
  truncated region constructor.
- `TruncatedMemoryRegion::size` (L369) — delegates `self.0.size()`.

`MemoryRegion::{start,size}` have **no external callers**; they are reached only
through the conversion constructors and the `TruncatedMemoryRegion` delegating
getters. They are not dead code — they are the read side of the
`MemoryRegion → TruncatedMemoryRegion` pipeline.

## Caller Expectations

### `MemoryRegion::start(&self) -> T`
- Callers assume: returns exactly the start address stored at construction
  (`new`), unchanged, by value (`self.start.clone()`); pure and idempotent. The
  returned `T` is a full `Address` that supports `align_down`, raw-value
  extraction, and virtual→physical translation. For an unaligned (non-truncated)
  region the address may be arbitrarily aligned.
- Callers don't care about: how `T` is stored internally, the `String` name, or
  any other field; nor whether `start` is page-aligned (they align it
  themselves).
- Would break callers: returning a different/derived address, or one that no
  longer round-trips through `into_raw_value` / `from_virtual_address`.
- Would NOT break callers: changing the internal field layout, the name length
  cap, or cache-policy handling.

### `MemoryRegion::size(&self) -> usize`
- Callers assume: returns the exact byte length stored at construction, `> 0`
  (the constructor rejects `size == 0`), with `start_raw + size - 1 <= T::max_addr()`
  (constructor guarantees no overflow / in-range end). Pure, idempotent.
- Callers don't care about: whether `size` is page-aligned (the *untruncated*
  region need not be — `from_memory_region` later rounds up via `align_up`).
- Would break callers: returning 0, or a value inconsistent with the
  constructor-checked `[start, start+size)` range.

### `TruncatedMemoryRegion::start(&self) -> PageAligned<T>`
- Callers assume: returns the region's base address as a `PageAligned<T>`, i.e.
  **already page-aligned** (`start % page_size == 0`, the type-level invariant
  `inv()`). Callers immediately project it to a raw `usize` (overlap math) or a
  frame number (`into_frame_number`), both of which are only meaningful for an
  aligned base. Pure, idempotent, equals the value supplied to `new`.
- Callers don't care about: the underlying `MemoryRegion` wrapper, name, type, or
  permissions; only the numeric base.
- Would break callers: returning an unaligned address (frame-number / overlap
  computations would be wrong), or an address not equal to the stored start.
- Would NOT break callers: internal newtype representation
  (`TruncatedMemoryRegion(MemoryRegion<PageAligned<T>>)`).

### `TruncatedMemoryRegion::size(&self) -> usize`
- Callers assume: returns the byte length, `> 0` and a **multiple of the page /
  frame size** (`size % page_size == 0`, the `inv()` invariant established by
  `align_up` in `new`). `frame.rs` divides `size / FRAME_SIZE` expecting an exact
  frame count with no remainder; MMIO overlap math treats it as an exact byte
  extent. Pure, idempotent.
- Callers don't care about: the original pre-truncation size, or any other field.
- Would break callers: returning a non-page-multiple size (frame count would be
  truncated and the last frame dropped), returning 0, or a value inconsistent
  with `start + size` staying in range.
- Would NOT break callers: how truncation/rounding is implemented internally.

## Abstract Resource

A **memory region** is an immutable, contiguous, half-open address range
`[start, start + size)` within a typed address space (`T: Address`), tagged with
metadata (name, `MemoryRegionType`, `AccessPermission`, optional
`MmioCachePolicy`). `MemoryRegion<T>` is the general form; `TruncatedMemoryRegion<T>`
is the same resource with both endpoints snapped to page granularity. From the
caller's perspective the region is fully described by its **start** and **size**
(the geometry) plus its metadata; `start`/`size` are the read accessors that
expose the geometry.

## Key Invariants (caller perspective)

- **Non-empty:** `size > 0` for every constructed region (enforced by
  `MemoryRegion::new`).
- **In-range:** `start_raw + size - 1 <= T::max_addr()` (no overflow; region fits
  in the address space).
- **Page alignment (TruncatedMemoryRegion only):** `start % page_size == 0` and
  `size % page_size == 0` — this is the `inv()` predicate and is the load-bearing
  property for `frame.rs` (`size / FRAME_SIZE` exact) and for treating `start` as
  a valid frame base. `MemoryRegion` carries no alignment guarantee.
- **Stability / purity:** `start` and `size` are deterministic getters that
  return the exact values fixed at construction; repeated calls yield identical
  results and never mutate the region.
- **Ordering key:** region ordering is defined solely by `start`, so `start` must
  be a faithful, comparable projection of the stored base address.

## Pre-existing Specs (from upstream verification)

- `region.spec.rs` and `region.proof.rs` are both empty (`verus! { }`); no
  `#[verus_spec]` annotations exist on any function in `region.rs`.
- A `View` for `MemoryRegion<T>`/`TruncatedMemoryRegion<T>` (`MemoryRegionView`
  with fields `start: int, size: int, typ, perm, cache_policy`) and the
  `TruncatedMemoryRegion::inv()` predicate (`start % page_size == 0 &&
  size % page_size == 0`) already exist in the in-file `verus!` block. The View
  is closed and currently mirrors the implementation fields one-to-one.
- **Assessment:** No function-level specs to validate. The existing View already
  exposes the two geometry fields (`start`, `size`) that every caller relies on,
  plus metadata; it is a reasonable caller-abstract starting point. The `start`
  and `size` accessors should `ensure` they return `self@.start` / `self@.size`
  respectively (with the page-alignment invariant available for the truncated
  variant). `name` is the only stored field not represented in the View.
