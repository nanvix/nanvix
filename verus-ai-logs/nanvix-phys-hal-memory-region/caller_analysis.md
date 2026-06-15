# Caller Analysis: hal::mem::types::region

## Script Output

Raw `find_callers_lsp.py` report: `/tmp/callers_out.md` (regenerated on demand).
Command:

```bash
python scripts/find_callers_lsp.py \
  src/kernel/src/hal/mem/types/region.rs --project-dir /home/ruize/nanvix-phy
```

- Crate: `kernel` (no external crate depends on it — intra-crate analysis only).
- Total exec functions: 28 (all `pub` / trait-`pub`, 0 private).
- Public types referenced externally: `MemoryRegion` (28 refs), `TruncatedMemoryRegion`
  (53 refs), `MemoryRegionType` (14 refs), `MmioCachePolicy` (8 refs).

This phase only puts the following functions in scope (verification-order target):

- `TruncatedMemoryRegion::start`
- `MemoryRegion::start`
- `TruncatedMemoryRegion::size`
- `MemoryRegion::size`

All four are pure read-only accessors returning a clone/copy of an immutable field
(`start: T` / `start: PageAligned<T>` and `size: usize`). They never mutate, never
fail, and never allocate beyond the address clone.

---

## Caller Map (target functions)

### `TruncatedMemoryRegion::start(&self) -> PageAligned<T>` — 10 external + 1 internal

| File | Line | Use |
|------|-----:|-----|
| `mm/kernel_vas.rs` | 139 | `let mut vaddr: PageAligned<VirtualAddress> = region.start();` — iteration base |
| `mm/kernel_vas.rs` | 141 | `region.start().into_raw_value() + (region.size() - 1)` — inclusive end address |
| `hal/io/mmio/allocator.rs` | 189 | `region.start().into_raw_value()` — overlap-check start |
| `hal/io/mmio/allocator.rs` | 193 | `entry.region.start().into_raw_value()` — overlap-check start |
| `hal/io/mmio/region.rs` | 75 | `self.region.start()` — exposed as `MmioRegion::base()` |
| `mm/phys/frame.rs` | 569 | `region.start().into_frame_number().into_raw_value()` — first frame number |
| `mm/phys/frame.rs` | 589 | `region.start().into_raw_value()` — region start for error reporting |
| `mm/virt/boot_init.rs` | 93 | `region.start().into_raw_value()` — identity-map base vaddr |
| `mm/virt/boot_init.rs` | 97 | `region.start().into_inner()` — MMIO base address |
| (internal) `TruncatedMemoryRegion::fmt` | 423 | Debug formatting |

### `TruncatedMemoryRegion::size(&self) -> usize` — 8 external + 1 internal

| File | Line | Use |
|------|-----:|-----|
| `mm/kernel_vas.rs` | 141 | `region.size() - 1` — inclusive end offset |
| `hal/io/mmio/allocator.rs` | 190 | `compute_inclusive_end(start, region.size())` |
| `hal/io/mmio/allocator.rs` | 194 | `compute_inclusive_end(reg_start, entry.region.size())` |
| `hal/io/mmio/region.rs` | 103 | `self.region.size()` — exposed as `MmioRegion::size()` |
| `mm/phys/frame.rs` | 570 | `start_frame_number + region.size() / mem::FRAME_SIZE - 1` — last frame number |
| `mm/phys/frame.rs` | 590 | `region_start.saturating_add(region.size())` — region end |
| `mm/virt/boot_init.rs` | 111 | `raw_vaddr + (region.size() - 1)` — identity-map end |
| (internal) `TruncatedMemoryRegion::fmt` | 424 | Debug formatting |

Note: `mm/phys/mod.rs::book_physical_memory_regions` reaches `start`/`size` transitively
through `frame::alloc_range`; it is `external_body` (TCB-allowed) and only relies on the
abstract `PhysMemView` booking effect, not on the accessors directly.

### `MemoryRegion::start(&self) -> T` — 1 external + 3 internal

| File | Line | Use |
|------|-----:|-----|
| `mm/kernel_vas.rs` | 68 | `PhysicalAddress::from_virtual_address(region.start()).is_ok()` — routing test |
| (internal) `TruncatedMemoryRegion::from_memory_region` | 345 | `region.start().align_down(PAGE_ALIGNMENT)?` |
| (internal) `TruncatedMemoryRegion::start` | 364 | delegates `self.0.start()` |
| (internal) `from_virtual_memory_region` | 441 | `PhysicalAddress::from_virtual_address(region.start())?` |

### `MemoryRegion::size(&self) -> usize` — 0 external + 3 internal

| File | Line | Use |
|------|-----:|-----|
| (internal) `TruncatedMemoryRegion::from_memory_region` | 348 | `let size = region.size();` forwarded to `new` |
| (internal) `TruncatedMemoryRegion::size` | 369 | delegates `self.0.size()` |
| (internal) `from_virtual_memory_region` | 442 | `let size = region.size();` forwarded to `new` |

`MemoryRegion::size` has no direct external call site — it is consumed only through the
`TruncatedMemoryRegion` wrapper and conversion constructors. It is *not* dead code; the
conversion path (`from_memory_region` / `from_virtual_memory_region`) round-trips it into
the truncated region whose `size` is then read widely.

---

## Trait Obligations

The target functions implement no trait; they are inherent accessors. However the values
they expose feed `Ord`/`PartialOrd` for both region types, which sort **by start address**
(`self.start.cmp(&other.start)` / delegated). Callers that keep regions in ordered/overlap
structures (mmio allocator, kernel_vas lists) therefore rely on `start` being the stable
ordering key.

---

## Caller Expectations

### `TruncatedMemoryRegion::start`
- **Callers assume:** returns the region's first valid address as a `PageAligned<T>`, i.e.
  the value is page-aligned by construction (callers feed it straight into
  `into_frame_number`, `into_inner`, `into_raw_value` and treat it as a frame/page base
  with no re-alignment). The address equals the `start` passed at construction.
- **Callers assume (range math):** `start.into_raw_value() + (size - 1)` does not overflow
  for valid regions — guaranteed because `MemoryRegion::new` rejected regions whose end
  exceeds `T::max_addr()`.
- **Callers don't care about:** the `name`, `typ`, `perm`, `cache_policy` fields, nor that
  the value is stored inside a wrapped `MemoryRegion<PageAligned<T>>`. The return is purely
  the start address.

### `MemoryRegion::start`
- **Callers assume:** returns the start address `T` exactly as supplied at construction (a
  faithful clone), suitable for `from_virtual_address` conversion and `align_down`.
- **Callers don't care about:** alignment guarantees (unlike the truncated variant, this `T`
  is *not* necessarily page-aligned — `from_memory_region` explicitly `align_down`s it).

### `TruncatedMemoryRegion::size`
- **Callers assume:** returns the byte length, which is a **non-zero multiple of the page
  size** (`size % page_size == 0`, `size >= 1`). Frame-count callers depend on exact
  divisibility: `size / FRAME_SIZE` is used without remainder handling, and `size - 1` is
  used as an inclusive-end offset assuming `size >= 1`. The page-multiple property comes
  from `TruncatedMemoryRegion::new` (`align_up(size, PAGE_ALIGNMENT)`) and is captured by
  the existing `inv()` (`self@.size % spec_page_size() == 0`).
- **Callers don't care about:** how truncation happened or any other field.

### `MemoryRegion::size`
- **Callers (internal only) assume:** returns the byte length as supplied/validated by
  `new` (`size >= 1`, end within the address space). It is forwarded verbatim into the
  truncated constructor, so no alignment is assumed at this layer.
- **Callers don't care about:** alignment — the multiple-of-page property is established
  later by `TruncatedMemoryRegion::new`, not here.

### Cross-cutting consistency expectation
Across all four, callers rely on the pair `(start, size)` describing a half-open byte
interval `[start, start + size)` that lies entirely within the address space and does not
wrap, so derived values — inclusive end `start + size - 1`, frame range
`[start/FRAME_SIZE, (start+size)/FRAME_SIZE)`, overlap tests — are well-defined.

---

## Abstract Resource

This module models an **address-space interval with metadata**: a contiguous byte range
identified by a start address and a size, tagged with type/permissions/cache policy.
`TruncatedMemoryRegion` is the same interval constrained so that both endpoints are
page-aligned (start is `PageAligned<T>`, size is a page-size multiple), making it directly
usable for paging/frame booking. From the caller's perspective the only state that matters
for the target accessors is the `(start, size)` geometry; the View should expose `start`
and `size` as abstract integers (already present as `MemoryRegionView.start: int` /
`size: int`).

---

## Key Invariants (caller perspective)

- `start` and `size` are immutable for a region's lifetime; the accessors are pure and
  return values equal to the constructor inputs (modulo truncation for the truncated
  variant). Repeated calls return the same value.
- `size >= 1` for every constructed region (`new` rejects `size == 0`).
- The interval does not overflow the address space: `start + size - 1 <= T::max_addr()`.
- For `TruncatedMemoryRegion`: `start` is page-aligned and `size % page_size == 0`
  (existing `inv()`), so `size / FRAME_SIZE` is exact and `start` needs no re-alignment.
- `start` is the ordering key (`Ord`); equal-start regions compare equal on start.

---

## Pre-existing Specs (from upstream verification)

- **Source:** `region.spec.rs` is an empty `verus! { }` stub — no executable specs.
- **Functions with specs:** none (`grep verus_spec region.rs` → none).
- **View type:** **exists** in the in-file `verus!` block — `MemoryRegionView { start: int,
  size: int, typ, perm, cache_policy }`, with `View` impls for both `MemoryRegion<T>` and
  `TruncatedMemoryRegion<T>` (the latter forwards to the inner region), plus
  `TruncatedMemoryRegion::inv()` requiring page-aligned `start`/`size`.

### Assessment
- **Coverage:** partial — a View and `inv()` exist, but none of the four target accessors
  have `ensures` linking their return value to `self@.start` / `self@.size`.
- **Strength:** weak for the accessors — callers above need `start() == self@.start` and
  `size() == self@.size` (as the abstract int) to discharge range/frame arithmetic; these
  ensures are currently missing.
- **View design:** caller-abstract and adequate — `start`/`size` as `int` exactly match what
  every caller uses (`into_raw_value`, frame math, overlap tests). No field is biased toward
  a single caller. The `closed` view is fine; accessor `ensures` should bridge to it.
