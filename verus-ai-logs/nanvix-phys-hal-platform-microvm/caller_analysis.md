# Caller Analysis: `hal::platform::microvm` (`mod.rs`)

## Script Output
See: `verus-ai-logs/nanvix-phys-hal-platform-microvm/find_callers_lsp_output.md`
(raw output of `scripts/find_callers_lsp.py`).

- Crate: `kernel` (intra-crate analysis only; no external crate depends on it).
- Module summary: 28 exec functions (21 pub/trait-pub, 7 private), 3 types.
- **Verification-order target (in scope): `gva_to_gpa`** — the only function
  analyzed in depth here. All other functions are out of scope per the task's
  hard rules and are not modified.

## Verification-order Target: `gva_to_gpa`

```rust
///
/// # Description
/// Translates a guest virtual address to a guest physical address.
/// # Returns
/// The guest physical address corresponding to the given guest virtual address.
///
#[inline(always)]
pub fn gva_to_gpa(gva: usize) -> usize {
    gva
}
```

On the MicroVM platform this is an **identity translation** (GVA == GPA): the
guest runs with a flat/identity-mapped address space, so the function returns
its argument unchanged. It is `#[inline(always)]`, total (no panics, no error
path), and pure.

### Callers (1 external call site, via re-export)

The function is re-exported from `hal::platform` through
`pub use microvm::*;` (`src/kernel/src/hal/platform/mod.rs:21`) and reached as
`crate::hal::platform::gva_to_gpa`.

| File | Line | Caller fn | Context |
|------|-----:|-----------|---------|
| `src/kernel/src/mm/phys/mod.rs` | 128 | `book_mmio_regions` | `let mmio_addr: usize = crate::hal::platform::gva_to_gpa(start);` |

The LSP script reported the call site one line earlier (the fn signature at
L84) because the call is wrapped inside `book_mmio_regions`; the actual
invocation is at `mm/phys/mod.rs:128`, confirmed by grep.

#### Caller code (`book_mmio_regions`, abbreviated)

```rust
for region in mmio_regions.iter() {
    let mut start: usize = region.start().into_raw_value();
    let end: usize = start + (region.size() - 1);
    while start < end {
        let mmio_addr: usize = crate::hal::platform::gva_to_gpa(start);
        let phys_addr: PageAligned<PhysicalAddress> = PageAligned::from_address(unsafe {
            // MMIO GPAs may legitimately lie outside tracked RAM, so they must
            // not go through the regular physical-address validator here.
            PhysicalAddress::from_mmio_address(VirtualAddress::from_raw_value(mmio_addr))?
        })?;
        if frame::is_covered(phys_addr) {
            frame::book(phys_addr)?;
        }
        start += mem::FRAME_SIZE;
    }
}
```

## Trait Obligations
- None. `gva_to_gpa` is a free function and implements no trait. There is no
  runtime/implicit dispatch obligation on this function (no `Drop`, `Iterator`,
  `GlobalAlloc`, etc.).

## Caller Expectations

### `gva_to_gpa(gva: usize) -> usize`
- **Callers assume (success):**
  - The call is **total**: it always returns for any `usize` input and never
    panics or aborts (it sits inside a boot-time loop with no error handling
    around the call itself).
  - The result is a **guest physical address** suitable to be fed straight into
    `PhysicalAddress::from_mmio_address(VirtualAddress::from_raw_value(result))`.
    The caller treats the returned `usize` as a raw physical/MMIO address.
  - It is a **pure, deterministic function of `gva`** — same input yields same
    output, no observable side effects, no global state read/written. This lets
    the caller invoke it once per frame inside a tight `while` loop.
  - On MicroVM, the mapping is **address-preserving (identity)**: `start`,
    `start + FRAME_SIZE`, ... map to the same numeric GPAs. The caller relies on
    the translation **not reordering or aliasing** distinct page-aligned inputs,
    so iterating `start` by `FRAME_SIZE` walks distinct physical frames.
  - The result may legitimately lie **outside tracked RAM** (e.g. the LAPIC at
    `0xFEE0_0000`); the caller guards every result with `frame::is_covered`
    before booking, so `gva_to_gpa` is **not** required to validate or bound its
    output.
- **Callers assume (failure):** N/A — there is no failure mode; the signature
  returns `usize`, not `Result`.
- **Would break the caller if changed:**
  - Introducing a panic / non-total behavior for any input.
  - Returning a value that is not a valid raw MMIO/physical address encoding
    (anything `VirtualAddress::from_raw_value` + `from_mmio_address` cannot
    accept).
  - Making the mapping non-injective on page-aligned inputs (collapsing distinct
    frames), or non-monotone/non-identity in a way that skips or double-books
    frames during the MMIO walk.
  - Adding hidden side effects or input-independent results (non-purity /
    non-determinism).
- **Would NOT break the caller if changed:**
  - The *internal* implementation (currently `gva`) — e.g. applying a real
    page-table walk or an offset, as long as the output remains a valid GPA and
    the injectivity/totality guarantees hold.
  - Whether or not the returned GPA is backed by tracked RAM (caller filters via
    `is_covered`).

## Abstract Resource
From the caller's perspective this function exposes the platform's
**guest-virtual → guest-physical address translation** for the MicroVM platform.
On MicroVM that translation is the identity map, reflecting the guest's flat,
identity-mapped address space; the module as a whole abstracts the MicroVM
hardware/firmware platform (control registers, MMIO regions, boot stack, klog
storage, shutdown), of which address translation is one small, pure facet.

## Key Invariants (caller perspective)
- **Totality:** defined for every `usize`; never panics.
- **Purity / determinism:** result depends only on `gva`; no side effects.
- **Identity (MicroVM):** `gva_to_gpa(gva) == gva` — address-preserving, hence
  injective and order-preserving over the frame walk.
- **Valid address encoding:** the result is always acceptable to
  `VirtualAddress::from_raw_value` + `PhysicalAddress::from_mmio_address`.
- The function makes **no claim** about whether the resulting GPA is tracked RAM;
  coverage is the caller's responsibility.

## Pre-existing Specs (from upstream verification)
- Searched `mod.rs`, `mod.spec.rs`, `mod.proof.rs`: **no** `#[verus_spec]` /
  `#[verus_verify]` annotations reference `gva_to_gpa`, and `mod.spec.rs`
  currently contains only an empty `verus! { }` block. (`tcb-allowed.md` was not
  found, so `gva_to_gpa` is not authorized for `external_body`.)
- View type: does not exist for this function.
- **Assessment:** No upstream bias to reconcile. A View/spec should encode the
  caller-relevant contract above — totality + purity + identity (`result == gva`
  on MicroVM) — rather than mirroring any internal platform detail.
