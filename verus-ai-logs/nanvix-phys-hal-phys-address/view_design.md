# View Design: `PhysicalAddress` (`hal/mem/types/address/phys.rs`)

In-scope (verification-order) targets: the type `PhysicalAddress` (its `View` +
`inv`), `PhysicalAddress::from_number`, `PhysicalAddress::into_frame_number`, and
`PhysicalAddress::from_mmio_address`. All other functions (the `Address` trait
impl, `from_virtual_address`, `from_frame_address`, `Debug`, …) are **out of
scope** and untouched.

## Abstract Resource

`PhysicalAddress` is an **opaque integer identifier for a byte location in
guest-physical memory**. To every caller (frame allocator, page-table machinery,
kernel-module descriptors, `FrameAddress`) its only observable state is a single
mathematical integer — the **raw physical address** — together with one *derived*
concept computed from it: the **frame** the address lies in
(`addr >> FRAME_SHIFT`, i.e. `addr / FRAME_SIZE`).

It is *not* a collection, resource manager, or state machine: it is an immutable,
`Copy`, totally-ordered scalar value. The fact that it is internally a newtype
over `VirtualAddress` is invisible to callers (confirmed by the caller analysis:
"callers don't care that it is internally a newtype over `VirtualAddress`").

## View Type

The abstract value of a `PhysicalAddress` is a **single integer**: the raw
address. There is exactly one caller-observable quantity, so the View is a scalar
`int`, not a one-field struct. This View **already exists** in the source and is
kept unchanged:

```rust
impl View for PhysicalAddress {
    type V = int;

    // `closed`: callers may reference self@, but the mapping to the inner
    // VirtualAddress field is hidden. The abstract value is "the address as int".
    closed spec fn view(&self) -> int {
        self.0@
    }
}
```

`self@ : int` is the entire abstract state. This mirrors the already-verified
sibling abstractions (`VirtualAddress@ == self.0 as int`,
`PageAligned<T>@ == self.0@`, `FrameAddress@ == self.0@`), confirming the
abstraction is caller-driven, not implementation-driven: the whole address tower
models a value as one `int`.

### Why `int`, not a struct

Per the view-design skill (Step 2, *minimize fields*): every field must
correspond to a caller-observable abstract concept, and there is only one — the
address. The frame size / shift is a **module-wide constant**
(`spec_page_size()`), not per-value state, so it belongs in spec helpers and
`inv()`, not the View. The frame number is **derived** from the address
(`self@ / spec_page_size()`), so storing it would duplicate state and risk
inconsistency. Wrapping a lone `int` in a struct would add ceremony to every spec
with no gain.

### Derived spec helper (on the View domain)

The frame index is the one derived quantity callers reason about, so it gets a
named helper over the abstract `int` rather than a stored field:

```rust
// The frame that this physical address belongs to. Equivalent to the
// implementation's `addr >> FRAME_SHIFT`, stated as exact integer division
// so the spec is independent of shift-vs-divide.
pub open spec fn spec_frame_number(addr_view: int) -> int {
    addr_view / spec_page_size()
}
```

(`spec_page_size() : int` is the pre-existing uninterpreted module constant, tied
to `FRAME_SIZE`/`PAGE_SIZE` by the existing `assume_specification`s; the analysis
established `FRAME_SIZE == spec_page_size()`.)

## Well-formedness Invariant

`PhysicalAddress` carries **one** universal property that every constructible
value satisfies and that callers depend on: its address has a **representable
frame number**, which is exactly what makes `into_frame_number` total (its
internal `FrameNumber::from_raw_value(..).unwrap()` never panics).

```rust
impl PhysicalAddress {
    // `open`: callers (frame allocator, page tables) rely on totality of
    // into_frame_number, so the underwriting fact must be visible in their proofs.
    pub open spec fn inv(&self) -> bool {
        // 0 <= self@ holds structurally (view comes from a usize); the operative
        // bound is that the frame index fits a FrameNumber.
        spec_frame_number(self@) <= spec_max_frame_number()
    }
}
```

where `spec_max_frame_number() : int` denotes `FrameNumber::MAX`
(`= MAX_ADDRESS / FRAME_SIZE - 1`). This is the minimal invariant required for
totality; the exact integer form is finalized in the spec phase (it may be stated
as `self@ < (spec_max_frame_number() + 1) * spec_page_size()`).

**Why this and not alignment.** Unlike `PageAligned`, `PhysicalAddress` does *not*
carry an alignment invariant: `from_mmio_address` deliberately wraps arbitrary,
possibly-unaligned MMIO addresses (e.g. the LAPIC at `0xFEE0_0000`) and bypasses
the RAM-range validator. So alignment is a property of *specific* constructors
(`from_number`), not of the type. Frame-representability is the only property
*every* `PhysicalAddress` shares, and it is precisely what the totality
obligation needs.

## Spec Transition Functions

`PhysicalAddress` is an immutable value type; the in-scope functions are
constructors/projections, so the "transitions" relate input values to the
resulting abstract value, expressed over the View's `int` domain.

```rust
// Base address of a frame: from_number multiplies the frame index by the
// frame size. Stated as multiplication, independent of the body's shift/mul.
pub open spec fn spec_from_number(frame_view: int) -> int {
    frame_view * spec_page_size()
}
```

### `from_number(frame: FrameNumber) -> Self` — total constructor

```text
ensures
    result@ == spec_from_number(frame.into_raw_value() as int)
    // i.e. result@ == frame.into_raw_value() as int * spec_page_size()
    // hence result@ % spec_page_size() == 0   (frame-aligned base address)
    // and   result.inv()                      (frame index == frame <= MAX)
```

- **Total** (no `Result`): every `FrameNumber` yields a `PhysicalAddress`, as
  callers assume.
- **Value relation, not mechanism**: result is the frame's base address; the
  caller relies on it being `FRAME_SIZE`-aligned so the immediately-following
  `PageAligned::from_address(..)?` succeeds. Alignment and `inv()` both *follow*
  from this single ensures (frame index `frame <= FrameNumber::MAX`), so they need
  not be listed separately.

### `into_frame_number(self) -> FrameNumber` — total projection

```text
requires self.inv()                 // underwrites totality of the internal unwrap
ensures  result.into_raw_value() as int == spec_frame_number(self@)
         // == self@ / spec_page_size()  (equivalently self@ >> FRAME_SHIFT)
```

- **Total** under `inv()`: the receiver's invariant guarantees the frame index
  fits a `FrameNumber`, so the body's `unwrap` never panics.
- **Identifies the containing frame**: used directly as bitmap index, refcount
  index, and PTE frame field, so the value must equal `addr / FRAME_SIZE` exactly.
- **Same-frame / distinct-frame** behaviour (relied on by `pm/test.rs`
  distinctness checks and bitmap indexing) is a *consequence* of this exact
  integer-division equality, not a separate clause.
- **Round trip** (caller intent, provable from the two ensures above):
  `from_number(n).into_frame_number().into_raw_value() == n.into_raw_value()`,
  and for a frame-aligned `p`,
  `from_number(p.into_frame_number())@ == p@`.

### `from_mmio_address(addr: VirtualAddress) -> Result<Self, Error>` (`unsafe`)

```text
ensures
    match result {
        Ok(r)  => r@ == addr@,   // identity wrapping; no RAM-range validation
        Err(_) => true,          // payload unconstrained; arm currently unreachable
    }
```

- **Identity, unchecked**: `Ok(r) => r@ == addr@`. This is the whole point —
  it bypasses `is_valid_physical_address`, so it succeeds for addresses that the
  range-checked constructors reject. Callers gate separately on
  `frame::is_covered(..)`.
- **`unsafe` contract**: the caller is responsible for the MMIO address being
  valid. That obligation underwrites `r.inv()` on the result (so the address may
  later flow into `into_frame_number` via `is_covered`); whether `inv()` is
  attached to the `Ok` arm as an ensures or discharged from a `requires`/`unsafe`
  precondition is settled in the spec phase. Real MMIO addresses sit far below the
  top of the address space, so the frame-representability bound holds in practice.
- **Error path**: the body is currently `Ok(Self(addr))`, so `Err` never occurs;
  callers propagate with `?` and treat `Err` as a benign skip. The spec leaves the
  `Error`/`ErrorCode` payload unconstrained (callers never inspect it) rather than
  inventing a failure condition that the implementation does not exhibit.

## Design Rationale (substitution test per field)

The View has a single field, `self@ : int`. Applying the test — *"if the
implementation were completely rewritten with a different algorithm, would this
still make sense?"*:

- **`self@ : int` (raw address)** — ✅ survives any rewrite. Whether
  `PhysicalAddress` is a newtype over `VirtualAddress`, a bare `usize`, or a
  struct with tag bits, "the integer physical address" is the value every caller
  reasons about (frame indexing, ordering, MMIO identity). The `closed` view hides
  *how* the int is stored.
- **`spec_frame_number` helper** — ✅ derived purely from `self@` and the
  module constant; any implementation that means the same thing by "frame number"
  computes `addr / FRAME_SIZE`, regardless of shift-vs-divide.
- **`inv()` = frame-representable** — ✅ a property of the *abstract* value
  (its frame index fits a `FrameNumber`), not of any storage layout. Any
  implementation must maintain it for `into_frame_number` to be total.

All three are caller-only (understandable without reading bodies), complete (cover
every caller-observable concept in the analysis: address value, frame number,
totality, alignment-of-`from_number`, MMIO identity), and minimal (each is used by
at least one in-scope spec).

## Rejected Alternatives

- **A struct View `{ addr: int, frame: int }`** — rejected. `frame` is a pure
  function of `addr` (`addr / spec_page_size()`); storing it duplicates state,
  forces a redundant `inv()` consistency clause, and fails minimality. Kept as the
  derived helper `spec_frame_number` instead.

- **Exposing the inner `VirtualAddress` (e.g. `view() -> VirtualAddress` or a
  field for it)** — rejected. The caller analysis is explicit that the
  `VirtualAddress` wrapper is an implementation detail; only the integer matters.
  Mirroring it would violate the cardinal rule (don't mirror internal fields) and
  break specs if the wrapper changed.

- **An alignment invariant `self@ % spec_page_size() == 0`** — rejected as a
  *type* invariant. `from_mmio_address` legitimately produces unaligned addresses,
  so alignment cannot hold for all `PhysicalAddress` values. It is captured where
  it actually holds: as a *consequence* of `from_number`'s ensures.

- **A RAM-range invariant `is_valid_physical_address(self@)`** — rejected.
  `from_mmio_address` deliberately bypasses RAM validation for MMIO regions outside
  tracked RAM, so this is false for valid MMIO `PhysicalAddress` values. The weaker
  frame-representability bound is the property that is both universally true and
  sufficient for the totality obligation.

- **Modeling `into_frame_number` as returning `int` directly in the View** —
  N/A as a View field; handled by relating the returned `FrameNumber`'s
  `into_raw_value()` to `spec_frame_number(self@)` in the ensures, so the View
  stays a single scalar.

- **A `nat` View instead of `int`** — rejected for consistency with the existing
  `View` (`type V = int`) shared across `VirtualAddress`, `PageAligned`, and
  `FrameAddress`; switching to `nat` would desync this address from the rest of the
  tower for no benefit (non-negativity is structural and recorded in `inv()` if
  needed).
