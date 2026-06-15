# Caller Analysis: `hal::mem::types::address::phys` (`PhysicalAddress`)

## Script Output

See: `verus-ai-logs/nanvix-phys-hal-phys-address/find_callers_lsp_output.md`
(raw `find_callers_lsp.py` output — intra-crate, rust-analyzer LSP, crate `kernel`).

Module is leaf within the `kernel` crate: **no external crates depend on it**, so all
callers are intra-crate. The script reports 16 exec functions (all pub / trait-pub),
0 private, 1 type. This analysis focuses on the **verification-order target functions**:

- `PhysicalAddress` (the type)
- `PhysicalAddress::from_number`
- `PhysicalAddress::into_frame_number`
- `PhysicalAddress::from_mmio_address`

## Pre-existing Specs (from upstream verification)

- `phys.spec.rs` and `phys.proof.rs` exist but are **empty** (`verus! { }`).
- Functions with specs: **none**.
- View type: **does not exist** in the spec files. (A `closed spec fn view(&self) -> int`
  returning `self.0@` is defined inline in `phys.rs` under `verus_keep_ghost`, i.e. the
  physical address abstracts to the underlying virtual address's integer value.)
- Assessment: clean slate. No upstream caller has biased the View yet; the inline
  `View::V = int` (raw address as a mathematical integer) is the natural starting point
  and is consistent with how callers use the type (see below).

## Trait Obligations

`PhysicalAddress` implements:

- `Address` (`from_raw_value`, `align_up`, `align_down`, `is_aligned`, `max_addr`,
  `into_raw_value`, `as_ptr`, `as_mut_ptr`) — generic address contract. Used by generic
  code such as `PageAligned<T: Address>` (`from_address`, `PartialEq`, `PartialOrd`),
  which is the primary indirect consumer of `is_aligned`/`into_raw_value`.
- `Clone, Copy, PartialEq, Eq, PartialOrd, Ord` — value semantics; callers freely copy
  and compare physical addresses and order them as raw integers.
- `core::fmt::Debug` — delegates to inner `VirtualAddress`; callers only expect a
  human-readable hex dump (logging in `book`, boot_init, `mod.rs`).

Key trait-mediated expectation: `into_frame_number` / `from_number` and
`PageAligned::from_address` together rely on the `Address::is_aligned` contract — a
physical address built from a frame number is page-aligned, so wrapping it in
`PageAligned` cannot fail.

## Caller Expectations

### `PhysicalAddress` (the type) — 76 references

Representative callers: `mm/phys/frame.rs` (frame allocator: `book`, `is_covered`,
`alloc_range` take `PageAligned<PhysicalAddress>` / `TruncatedMemoryRegion<PhysicalAddress>`),
`mm/phys/mod.rs`, `kmod.rs` (stores `start` / `region_base` as `PhysicalAddress`),
`hal/mem/types/address/frame.rs` (`FrameAddress` wraps `PageAligned<PhysicalAddress>`).

- **Callers assume:**
  - It is a cheap `Copy` value that abstracts to a single integer (the raw physical
    address). Equality/ordering match integer equality/ordering of that raw value.
  - A `PhysicalAddress` that exists is a *valid* physical address (constructors enforce
    validity via `is_valid_physical_address`, except the explicitly-unsafe MMIO path).
  - Round-trips with `FrameNumber` and `usize` preserve the underlying value.
- **Callers don't care about:**
  - That the inner representation is a `VirtualAddress` newtype. They never see the inner
    type; they go through `into_raw_value` / `into_virtual_address` / frame conversions.
  - Any platform-specific validity range internals.

### `from_number(frame: FrameNumber) -> Self`

Sole caller: `hal/mem/types/address/frame.rs:78`
`Ok(Self(PageAligned::from_address(PhysicalAddress::from_number(frame_number))?))`
(inside `FrameAddress::from_frame_number`).

- **Callers assume (success — infallible, returns `Self` not `Result`):**
  - The produced address equals `frame * FRAME_SIZE`, i.e. it is **page-aligned**. This
    is load-bearing: the caller immediately feeds the result to
    `PageAligned::from_address`, which calls `is_aligned(PAGE_ALIGNMENT)` and would error
    on a misaligned address. The caller propagates that `?` but in practice expects it to
    never trip for a frame-number-derived address.
  - The result is a valid, in-range physical address for any well-formed `FrameNumber`
    (`FrameNumber` is constructed only within `0..=MAX`, where `MAX = MAX_ADDRESS/FRAME_SIZE - 1`).
  - It is the inverse of `into_frame_number`: `from_number(f).into_frame_number() == f`.
- **Callers don't care about:**
  - That it multiplies by `FRAME_SIZE` vs shifts by `FRAME_SHIFT` internally, or that it
    wraps a `VirtualAddress::new`.
- **Would break the caller if changed:** returning a non-page-aligned address, or an
  address not equal to `frame * FRAME_SIZE` (breaks the `PageAligned` wrap and the
  round-trip with `into_frame_number`).

### `into_frame_number(self) -> FrameNumber`

Callers (4):
- `hal/mem/types/address/frame.rs:82` — `FrameAddress::into_frame_number` delegates.
- `mm/phys/frame.rs:482` (`book`), `:518` (`is_covered`), `:569` (`alloc_range`):
  `let frame_number: usize = phys_addr.into_frame_number().into_raw_value();`
  then used to index a bitmap / refcount array.

- **Callers assume (success — infallible, total):**
  - It never panics for any `PhysicalAddress` that exists. The implementation `.unwrap()`s
    `FrameNumber::from_raw_value`; callers rely on the type invariant that a valid physical
    address always has a valid (in-range) frame number. The allocator code uses the result
    directly as an array/bitmap index with no bounds re-check beyond `is_covered`.
  - The returned frame number equals `raw_addr >> FRAME_SHIFT` (i.e. `raw_addr / FRAME_SIZE`),
    truncating any sub-frame offset. This is the abstract `view() / FRAME_SIZE`.
  - It is the inverse of `from_number` on page-aligned addresses; for the allocator,
    `phys_addr` arrives as `PageAligned<PhysicalAddress>`, so no offset is lost.
- **Callers don't care about:**
  - Whether it shifts or divides; only that the value is `floor(addr / FRAME_SIZE)`.
- **Would break the caller if changed:** introducing a panic for an in-range address, or
  returning a frame number not equal to `addr >> FRAME_SHIFT` (corrupts bitmap/refcount
  indexing → memory-safety bug in the frame allocator).

### `from_mmio_address(addr: VirtualAddress) -> Result<Self, Error>` (`unsafe`)

Callers (3):
- `mm/virt/boot_init.rs:100` and `:255` — build a `FrameAddress` for an MMIO region:
  `unsafe { PhysicalAddress::from_mmio_address(mmio_addr)? }` then `PageAligned::from_address`.
- `mm/phys/mod.rs:88` (region) / actual call at `book_mmio_regions` ~`:132` —
  `PhysicalAddress::from_mmio_address(VirtualAddress::from_raw_value(mmio_addr))?`.

- **Callers assume:**
  - It **bypasses the normal physical-address validity check** on purpose. The comment in
    `book_mmio_regions` is explicit: "MMIO GPAs may legitimately lie outside tracked RAM,
    so they must not go through the regular physical-address validator here." This is the
    whole reason the function exists separately from `from_virtual_address` /
    `from_raw_value`.
  - On success the resulting `PhysicalAddress` has the same raw value as the input
    `addr` (identity wrap): `view() == addr@`.
  - The `unsafe` contract: the caller is responsible for the address actually denoting a
    valid MMIO region (callers acknowledge this with `// FIXME: ensure safety here.`).
  - Returns `Ok` for any input in current impl (it never actually errors), but callers
    still `?`-propagate to stay forward-compatible with a fallible signature.
- **Callers don't care about:**
  - Internal representation; they immediately either page-align the result
    (`PageAligned::from_address`) or wrap it in a `FrameAddress`.
- **Would break the caller if changed:** re-introducing the RAM-range validity check
  (would reject legitimate out-of-RAM MMIO frames and abort boot), or returning an address
  whose raw value differs from the input (breaks the GVA→GPA→frame mapping).

## Abstract Resource

`PhysicalAddress` is a **validated handle to a single location in the host/guest physical
address space**, abstractly a non-negative integer (`view: int`, the raw address). It is
the currency exchanged between the frame allocator, the paging/boot code, and frame-number
arithmetic: callers convert it to/from `FrameNumber` (units of `FRAME_SIZE`), to/from raw
`usize`, and wrap it in `PageAligned<_>` / `FrameAddress` for frame-granular operations.

## Key Invariants (caller perspective)

- **Validity by construction:** any `PhysicalAddress` value denotes an in-range physical
  address — except via the deliberately `unsafe` `from_mmio_address`, which the caller
  promises to use only for legitimate MMIO GPAs that may lie outside tracked RAM.
- **Integer abstraction:** `view() == into_raw_value()`; `Eq`/`Ord` agree with integer
  equality/order on that value.
- **Frame round-trip:** for a valid frame number `f`,
  `from_number(f).into_frame_number() == f`; and `from_number(f)` is page-aligned
  (`== f * FRAME_SIZE`).
- **Frame projection is total and offset-truncating:** `into_frame_number()` never panics
  for an existing address and yields `floor(view() / FRAME_SIZE)` (`view() >> FRAME_SHIFT`),
  a value always within `FrameNumber`'s valid range — directly usable as an allocator index.
- **MMIO identity:** `from_mmio_address(a)` on success preserves the raw value (`view == a@`)
  and performs no RAM-range validation.

## Notes / Validation of Script Output

- Zero-caller pub functions (`from_frame_address`, `from_into_frame_address`, `fmt`,
  `is_aligned`, `max_addr`, `as_ptr`, `as_mut_ptr`, `align_up`) are **not dead**:
  - `fmt` is invoked implicitly via `{:?}` formatting in logging (`book`, boot_init, `mod.rs`).
  - `is_aligned`, `into_raw_value`, `from_raw_value`, `align_*` are reached generically
    through `PageAligned<T: Address>` / `TruncatedMemoryRegion<T>`; rust-analyzer attributes
    these to the generic site, so direct call-site counts undercount real usage.
  - `max_addr` / `as_ptr` / `as_mut_ptr` are part of the `Address` trait surface, available
    to generic consumers even without a current concrete call site.
  These are **out of scope** (not in the verification-order target list) and were not modified.
- No callers via function pointers/closures or external crates were found (leaf module,
  no dependents), so the LSP results are complete for the in-crate target functions.
