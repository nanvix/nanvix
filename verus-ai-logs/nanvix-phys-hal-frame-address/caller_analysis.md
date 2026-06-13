# Caller Analysis: `hal::mem::types::address::frame` (`FrameAddress`)

## Script Output
Generated with:
```
python scripts/find_callers_lsp.py \
  src/kernel/src/hal/mem/types/address/frame.rs --project-dir /home/ruize/nanvix-phy
```
- Parser: rust-analyzer LSP (intra-crate only)
- Crate: `kernel` (no external/cross-crate dependents — all callers live in this crate)
- Module summary: 9 exec functions (all `pub`/trait-pub), 0 private, 1 type (`FrameAddress`)

### Per-function call-site counts (script)
| Function (in scope?) | External callers | Notes |
|---|---:|---|
| `FrameAddress::new` | 7 | not in scope |
| `FrameAddress::into_physical_address` | 1 | not in scope |
| `FrameAddress::into_page_address` | 1 | not in scope |
| **`FrameAddress::into_frame_number`** | 7 | in scope |
| **`FrameAddress::from_frame_number`** | 9 | in scope |
| **`FrameAddress::from_raw_value`** | 3 | in scope |
| **`FrameAddress::into_raw_value`** | 19 (+1 internal, `fmt`) | in scope |
| **`FrameAddress` (type)** | 93 references | in scope |
| `Debug::fmt` | 0 | not in scope; calls `into_raw_value` internally |
| `PartialEq::eq` | 0 | not in scope; used implicitly via `==` |

> Note: a few "Context" snippets in the raw script table are misaligned to nearby
> comment/attribute lines (e.g. `mm/phys/frame.rs` L291/369/429, `mm/phys/manager.rs`
> L299/302/310). The verified call sites are confirmed below by reading the source.

## Verification-Order Targets (scope)
`into_raw_value`, `into_frame_number`, `from_raw_value`, `FrameAddress` (type),
`from_frame_number`. All other functions are out of scope and must not be touched.

## Trait Obligations
- `core::fmt::Debug for FrameAddress` — formats as `FrameAddress({:#010x})` using
  `into_raw_value()`. Expected semantics: the hex shown is the raw (page-aligned)
  physical address. No spec needed, but it constrains `into_raw_value` to be the
  user-visible raw address.
- `PartialEq for FrameAddress` — structural equality delegated to the inner
  `PageAligned<PhysicalAddress>`. Callers compare frame addresses (e.g. CoW
  old/new frame checks in `vmem.rs`) and rely on equality ⇔ same physical frame.

## Caller Expectations

### `FrameAddress::from_frame_number(FrameNumber) -> Result<Self, Error>`
Callers (9): `page_directory.rs` L147, `page_table.rs` L188/250/489,
`mm/virt/manager.rs` L293, `vmem.rs` L553/866, `mm/phys/frame.rs` L161.

Dominant pattern — decode a frame number stored inside a PTE/PDE back into a frame
address:
```rust
let paddr: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;
```
`mm/phys/frame.rs::alloc_any` (L150–168) takes a freshly allocated frame index,
turns it into a `FrameNumber`, then into a `FrameAddress` to hand back to the
allocator's callers.

- Callers assume on `Ok(fa)`: `fa` is the canonical, page-aligned physical frame
  whose frame number equals the input — i.e. `fa.into_frame_number() == input`
  and `fa.inv()` holds (`fa@ % PAGE_SIZE == 0`, `fa@ == input * PAGE_SIZE`).
- Callers assume on `Err`: nothing is produced; they propagate the error with `?`.
  No side effects.
- Would break callers: if the produced address were not page-aligned, or if the
  round-trip `from_frame_number → into_frame_number` lost/altered the frame index.
- Would NOT break callers: the internal representation
  (`PageAligned<PhysicalAddress>`), the concrete `Error` value, or how alignment
  is checked.

### `FrameAddress::into_frame_number(self) -> FrameNumber`
Callers (7): `page_table.rs` L143/494/582, `page_directory.rs` L105,
`mm/phys/frame.rs` (`free`, L300).

Dominant pattern — encode a frame address into a PTE/PDE frame field:
```rust
let pte = PageTableEntry::new(flags, paddr.into_frame_number());
```
`page_table.rs::fill` (L582) uses `base_address.into_frame_number()` then
iterates `base_frame.into_raw_value() + i` to build contiguous PTEs.
`mm/phys/frame.rs::free` (L300) does `frame.into_frame_number().into_raw_value()`
to recover the frame index for refcount bookkeeping.

- Callers assume: the returned `FrameNumber` is exactly the frame index of this
  address (`result * PAGE_SIZE == self@`); it is the inverse of
  `from_frame_number`. The value is stable and reusable to build PTEs and to index
  refcount arrays.
- Callers don't care about: how the conversion is computed internally.
- Would break callers: any off-by-one or non-inverse mapping vs `from_frame_number`,
  since PTEs round-trip through both directions.

### `FrameAddress::from_raw_value(raw_addr: usize) -> Result<Self, Error>`
Callers (3): `mm/virt/boot_init.rs` L207, plus `mm/phys/manager.rs` (the misaligned
snippets resolve to a real use site there).

Pattern — build a frame address from a raw physical address computed elsewhere
(e.g. an identity-mapped/linear address during boot mapping):
```rust
page_table.fill(start_index, count, FrameAddress::from_raw_value(raw_vaddr)?, flags, false)
```

- Callers assume on `Ok(fa)`: `fa@ == raw_addr` and `fa.inv()` (page-aligned). This
  is exactly the pre-existing ensures already attached to the function
  (`Ok(fa) => fa.inv()`).
- Callers assume on `Err`: the input was not a valid page-aligned physical address;
  error is propagated with `?`. No frame is created.
- Would break callers: accepting a non-page-aligned `raw_addr` as `Ok` (it must be
  rejected so the resulting frame satisfies `inv()`), or producing `fa@ != raw_addr`.
- Would NOT break callers: the failure reason / `Error` payload.

### `FrameAddress::into_raw_value(self) -> usize`
Callers (19 external + 1 internal in `fmt`). Heavy use in `vmem.rs`
(L1064/1221/1233/1375/1376, ...), `hwpt.rs` L219/318, `page_table.rs` L584 (via
`FrameNumber`), `kframe.rs`, `mm/phys/manager.rs`.

Dominant patterns — treat the raw value as a **physical base address**:
```rust
(src_frame.into_raw_value() + offset) as *const u8        // memcpy source
let dst_phys_addr_raw = dst_frame.into_raw_value() + offset; // memcpy dest
map(vaddr, paddr.into_raw_value(), true, true);            // program MMU
```

- Callers assume: the returned `usize` is the abstract physical address of the
  frame (`result as int == self@`), page-aligned, and safe to add byte offsets in
  `[0, PAGE_SIZE)` to and cast to a pointer. This matches the pre-existing ensures
  `result as int == self@`.
- Callers assume: it is the inverse of `from_raw_value` (`from_raw_value(x).into_raw_value() == x`).
- Callers don't care about: internal storage; only the numeric address matters.
- Would break callers: returning anything other than the page-aligned physical
  address (pointer arithmetic and MMU programming would corrupt memory).

### `FrameAddress` (type) — 93 references
Used pervasively as the canonical "physical frame" handle: return type of
`*.physical_address()` (page tables / page directories), the value stored/looked up
for user frames in `vmem.rs` (`find_user_frame`, CoW replace), the unit the physical
frame allocator (`mm/phys`) hands out and frees, and the `Deref` target backing
`KernelFrame` in `kframe.rs`.

- Callers assume: a `FrameAddress` always denotes a page-aligned physical frame
  (`inv()`: `self@ % PAGE_SIZE == 0`); it is `Copy`, comparable (`PartialEq`), and
  losslessly convertible to/from both a raw physical address and a frame number.
- Callers don't care about: that it wraps `PageAligned<PhysicalAddress>`.

## Pre-existing Specs (from upstream verification)
- Source: inline in `frame.rs` (the module under verification); `frame.spec.rs` and
  `frame.proof.rs` are empty stubs (`verus! { }`).
- View type: **exists** — `impl View for FrameAddress { type V = int; }` with
  `view(&self) == self.0@` (the abstract physical address as `int`).
- Invariant: `inv(&self) == (self@ % spec_page_size() == 0)` (page-aligned).
- Functions WITH specs:
  - `from_raw_value` — `#[verus_verify(external_body)]`, ensures `Ok(fa) => fa.inv()`.
  - `into_raw_value` — `#[verus_verify(external_body)]`, ensures `result as int == self@`.
- Functions WITHOUT specs (in scope): `from_frame_number`, `into_frame_number`
  (no `#[verus_spec]`), and the type/View/`inv` are defined but `from_frame_number`/
  `into_frame_number` lack ensures relating `self@` to the frame number.
- Both `external_body` functions are dependency-contract stubs ("until the address
  layer is verified"). Note: neither `from_raw_value` nor `into_raw_value` appears
  in `verus-ai-logs/tcb-allowed.md`, so the `external_body` here is a pre-existing
  module-local contract — do not add new `external_body` to other in-scope fns.

### Assessment
- Coverage: **partial**. `from_raw_value`/`into_raw_value` have ensures; the
  frame-number conversions (`from_frame_number`/`into_frame_number`) are
  unspecified despite 7–9 callers each that depend on a precise round-trip.
- Strength: **adequate** for the raw-value pair (`result as int == self@` and
  `Ok => inv()`), but `from_raw_value` lacks `Ok(fa) => fa@ == raw_addr` which
  several callers (boot_init) implicitly rely on. `into_frame_number` should ensure
  `result.into_raw_value() * PAGE_SIZE == self@` (inverse of `from_frame_number`).
- View design: **caller-abstract and appropriate** — `V = int` (the physical
  address) is exactly what every caller cares about (raw address for pointer math,
  `addr / PAGE_SIZE` for frame number). It is not biased toward any single caller.

## Abstract Resource
A `FrameAddress` is the address of a single page-aligned physical memory frame. The
module manages the bijection between a frame and its two equivalent identities — a
raw physical address (`usize`) and a frame number (`address / PAGE_SIZE`) — used by
the MMU layer to program PTEs/PDEs and by the physical/virtual memory managers to
allocate, free, copy, and map frames.

## Key Invariants (caller perspective)
- A valid `FrameAddress` is always page-aligned: `self@ % PAGE_SIZE == 0` (`inv()`).
- `into_raw_value` yields the physical address: `result as int == self@`.
- Round-trips are lossless and mutually inverse:
  - `from_raw_value(x).into_raw_value() == x` (for aligned, in-range `x`).
  - `from_frame_number(n).into_frame_number() == n`.
  - `into_frame_number(fa) * PAGE_SIZE == fa@` and
    `from_frame_number(n)@ == n * PAGE_SIZE`.
- Constructors that can fail (`from_raw_value`, `from_frame_number`) return `Ok`
  only when the result satisfies `inv()`; on `Err` no frame is produced and the
  error is propagated with `?` (no side effects).
- Equality of two `FrameAddress` values ⇔ same physical frame (relied on by CoW
  frame-replacement logic).
