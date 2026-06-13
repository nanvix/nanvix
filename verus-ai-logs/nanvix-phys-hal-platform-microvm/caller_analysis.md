# Caller Analysis: hal::platform::microvm (`gva_to_gpa`)

## Script Output

The automated caller-finding script
(`scripts/find_callers_lsp.py src/kernel/src/hal/platform/microvm/mod.rs`)
reported `gva_to_gpa` under **"Public Functions with No External Callers"**.

That result is a **false negative for the in-tree call graph**. The script
looks for callers that name `microvm::gva_to_gpa` directly, but the module is
re-exported one level up:

- `src/kernel/src/hal/platform/mod.rs:9`  — `mod microvm;`
- `src/kernel/src/hal/platform/mod.rs:21` — `pub use microvm::*;`

So callers reach the function through the **`crate::hal::platform`** path, not
through `microvm` directly. A repository-wide search confirms exactly one real
caller in source (excluding logs):

- `src/kernel/src/mm/phys/mod.rs:114` —
  `let mmio_addr: usize = crate::hal::platform::gva_to_gpa(start);`

## Function Under Analysis

```rust
// src/kernel/src/hal/platform/microvm/mod.rs:415
/// Translates a guest virtual address to a guest physical address.
/// Returns the guest physical address corresponding to the given guest virtual address.
#[inline(always)]
pub fn gva_to_gpa(gva: usize) -> usize {
    gva
}
```

On the MicroVM platform the kernel runs in an identity-mapped guest, so the
implementation is the identity function (`gpa == gva`).

## Trait Obligations

None. `gva_to_gpa` is a free function, not a trait method, and is not invoked
through any runtime-dispatched pattern (`GlobalAlloc`, `Drop`, `Iterator`,
etc.). It is a plain, total `usize -> usize` function.

## Caller Context

The sole caller is `book_mmio_regions` in `mm/phys/mod.rs`
(lines 103–132). It walks each MMIO `TruncatedMemoryRegion` frame-by-frame and,
for every page-aligned `start`, does:

```rust
let mmio_addr: usize = crate::hal::platform::gva_to_gpa(start);
let phys_addr = PageAligned::from_address(unsafe {
    // MMIO GPAs may legitimately lie outside tracked RAM, so they must not go
    // through the regular physical-address validator here.
    PhysicalAddress::from_mmio_address(VirtualAddress::from_raw_value(mmio_addr))?
})?;
if frame::is_covered(phys_addr) {
    frame::book(phys_addr)?;
}
start += mem::FRAME_SIZE;
```

The translated value is immediately treated as a **guest physical / MMIO
address** and converted into a `PhysicalAddress`, which is then tested against
the frame allocator's coverage and conditionally booked (reserved).

## Caller Expectations

### `gva_to_gpa(gva: usize) -> usize`

- **Callers assume (post-success):**
  - The function is **total and infallible** — it returns a `usize` directly
    (no `Result`/`Option`), never panics, and never traps. `book_mmio_regions`
    calls it inside a tight loop with no error handling around the call itself.
  - The returned value is the **guest physical address** that corresponds to
    the supplied guest virtual address, suitable to be wrapped by
    `PhysicalAddress::from_mmio_address`.
  - The mapping is **deterministic** (same input → same output) and
    **order-preserving / per-frame stable**: the caller advances `start` by
    `FRAME_SIZE` and relies on each translated address landing on the
    corresponding physical frame so that `frame::is_covered` / `frame::book`
    operate on the right frame.
  - On MicroVM specifically, the mapping is the **identity** (`gpa == gva`),
    which is why MMIO regions described with virtual addresses can be booked as
    physical frames without an offset.
- **Callers assume (post-failure):** N/A — there is no failure path. Any error
  handling at the call site (`?`) comes from the *subsequent*
  `PhysicalAddress::from_mmio_address`, not from `gva_to_gpa`.
- **Would break the caller if changed:**
  - Introducing panics, traps, or a non-`usize` return type.
  - Returning an address that does not correspond frame-for-frame to the input
    (e.g. a non-injective or non-frame-aligned remapping), which would cause the
    wrong frames to be coverage-checked and booked.
- **Would NOT break the caller if changed:**
  - The *internal* translation strategy. The caller does not depend on it being
    the identity function — only on it producing the correct GPA. A future
    platform could implement a real GVA→GPA walk and the caller would be
    unaffected as long as the returned GPA is correct.

## Abstract Resource

From the caller's perspective this function exposes the platform's **guest
virtual → guest physical address translation** for the MicroVM platform. It is
a pure, total mapping over the `usize` address space (the identity map on
MicroVM) that lets memory-management code reinterpret a virtual MMIO address as
the physical frame address it backs.

## Key Invariants (caller perspective)

- **Totality / infallibility:** defined for every `usize`; never panics, never
  errors.
- **Purity / determinism:** depends only on its argument; repeated calls with
  the same `gva` yield the same `gpa`.
- **Frame correspondence:** the GPA returned for a frame-aligned `gva` denotes
  the physical frame that backs that virtual frame (identity on MicroVM), so
  iterating per `FRAME_SIZE` over a region yields the matching sequence of
  physical frames.

## Pre-existing Specs (from upstream verification)

- Spec file `src/kernel/src/hal/platform/microvm/mod.spec.rs` exists but is
  empty (`verus! { }`).
- Functions with specs: **none** (no `verus_spec` annotations on `gva_to_gpa`).
- Functions WITHOUT specs: `gva_to_gpa` (and all other module functions).
- View type: **does not exist** for this module.

### Assessment

- Coverage: none — no upstream verifier has constrained this function yet.
- Strength: N/A.
- View design: open. Given the analysis above, the natural specification is a
  pure-function ensures of the form `result == gva` (identity) for MicroVM,
  with no `requires` clause, reflecting its total/infallible nature. No stateful
  View is required.
