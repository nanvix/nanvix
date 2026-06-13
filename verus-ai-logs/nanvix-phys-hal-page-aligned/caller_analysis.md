# Caller Analysis: `PageAligned` (`hal/mem/types/address/aligned/page.rs`)

## Script Output

See: `verus-ai-logs/nanvix-phys-hal-page-aligned/find_callers_lsp_output.md`
(raw output of `scripts/find_callers_lsp.py`).

Crate: `kernel` (no cross-crate dependents — intra-crate analysis only).
17 exec functions, all public/trait-pub, 0 private, 1 type (`PageAligned`).

In-scope (verification-order) targets, the only functions analyzed for design here:
`PageAligned::into_raw_value`, `PageAligned::from_address`, and the type `PageAligned`.

## Script Validation Notes (corrections / enrichment)

The LSP script reported `into_raw_value` as having **0 external callers**. This is a
**false negative**: `into_raw_value` is the `Address` trait method, and the script does
not resolve trait-method dispatch on concrete `PageAligned<_>` receivers. Real callers
found by reading the code:

- `src/kernel/src/hal/mem/types/address/frame.rs:120` —
  `FrameAddress::into_raw_value(self)` returns `self.0.into_raw_value()`, where
  `self.0: PageAligned<PhysicalAddress>`. This is the **upstream verified caller**.
- `src/kernel/src/mm/elf.rs:288` —
  `let page_addr: usize = vaddr.into_raw_value();` with
  `vaddr: PageAligned<VirtualAddress>`.
- `page.rs:191` (internal) — `into_physical_address` calls `self.into_raw_value()`.

`from_address` results (19 external + 5 internal call sites) are accurate. The most
important caller is `FrameAddress::from_raw_value` (frame.rs:100), the upstream verified
boundary.

## Trait Obligations

- Trait `Address` for `PageAligned<T>` — `PageAligned` is itself an `Address`. The two
  in-scope trait/inherent functions must honor the `Address` contract:
  - `into_raw_value(self) -> usize`: pure projection to the underlying raw address value;
    no mutation, no failure. Must equal the abstract address (`self@`).
  - `from_address(addr: T) -> Result<Self, Error>` (inherent constructor; also the body of
    the `Address::from_raw_value` impl): validates page alignment and wraps `addr`.
- Trait `Deref for PageAligned<T>` (Target = `T`) — implicit; lets callers transparently use
  the inner address' methods. Not in scope but explains why many "uses" are auto-deref.

## Caller Expectations

### `PageAligned::from_address(addr: T) -> Result<Self, Error>`

Callers (frame.rs, elf.rs, region.rs, vmem.rs, identity_map.rs, boot_init.rs,
page_table.rs, page_directory.rs, process/manager, plus internal `from_raw_value`,
`align_up`, `align_down`, `into_physical_address`, `into_virtual_address`):

- Callers assume on **`Ok(self)`**: the wrapped address is page-aligned, i.e.
  `result@ % spec_page_size() == 0` (`result.inv()` holds). The value is preserved —
  `result@ == addr@` (no rounding; `from_address` rejects rather than aligns).
  - This is exactly what the upstream verified `FrameAddress::from_raw_value`
    (frame.rs:91-101) relies on: its spec ensures `Ok(fa) => fa.inv()`, delegating to
    `PageAligned::from_address`. Since `FrameAddress@ == PageAligned@`, `from_address`
    must guarantee `inv()` on success.
- Callers assume on **`Err`**: the input was not page-aligned (or the inner conversion
  failed); no `PageAligned` is produced. Most callers propagate with `?`.
- Callers don't care about: the concrete `Error`/`ErrorCode` payload, the exact alignment
  mechanism (`is_aligned(PAGE_ALIGNMENT)`), or that the value is stored as a tuple field.
- What would break callers: if `Ok` could be returned for an unaligned address, or if the
  stored value differed from the input (e.g. silent align-down). Both would invalidate the
  `inv()` guarantee the frame/region/vmem layers build on.

### `PageAligned::into_raw_value(self) -> usize`

Callers (`FrameAddress::into_raw_value` frame.rs:120, `elf.rs:288`, internal
`into_physical_address`):

- Callers assume the returned `usize` **equals the abstract address value** `self@`
  (the page-aligned raw address) — a faithful, total, side-effect-free projection.
  - The upstream `FrameAddress::into_raw_value` spec ensures `result as int == self@`,
    and its body is `self.0.into_raw_value()`. So `PageAligned::into_raw_value` must
    ensure `result as int == self@`.
- Because the receiver already satisfies `inv()`, callers may further assume the result is
  page-aligned (`result % page_size == 0`), though the core expectation is value equality.
- Callers don't care about: how the raw value is extracted from the inner `T`.
- What would break callers: returning anything other than `self@` (e.g. a shifted frame
  number, or a re-aligned value).

### Type `PageAligned<T>`

- 171 type references across the kernel (kframe, mmap, frame, mmio region, phys mm,
  process manager, vmem, elf, page tables…). Used as a **type-level proof of page
  alignment**: once a value has type `PageAligned<VirtualAddress>` /
  `PageAligned<PhysicalAddress>`, callers treat it as a guaranteed-aligned address and
  pass it without re-checking.
- `FrameAddress` wraps `PageAligned<PhysicalAddress>` and derives both its `view`
  (`self.0@`) and `inv` (`self@ % spec_page_size() == 0`) directly from it — so the
  `PageAligned` View/invariant is the foundation the frame layer is verified against.

## Abstract Resource

`PageAligned<T>` represents a **single memory address that is guaranteed to be aligned to a
page boundary** (a validated newtype over an `Address`). Its abstract value is just the
address (an `int`); its meaning is the carried proof that the address is page-aligned.

## Key Invariants (caller perspective)

- View: `PageAligned@ == (inner address)@` — abstract value is the raw address as `int`.
- Invariant: `self@ % spec_page_size() == 0` (`inv()`); every constructible `PageAligned`
  satisfies it.
- `from_address` is **validating, not normalizing**: `Ok(r)` ⇒ `r@ == addr@ ∧ r.inv()`;
  unaligned inputs yield `Err`, never a silently re-aligned value.
- `into_raw_value` is a **total, value-preserving projection**: `result as int == self@`.
- These two functions are the trusted boundary that the already-verified `FrameAddress`
  layer (and the wider `mm` stack) depends on; their guarantees must match
  `FrameAddress`'s existing specs (`Ok => fa.inv()`, `result == self@`).

## Pre-existing Specs (from upstream verification)

- `page.spec.rs` / `page.proof.rs` exist but are empty (`verus! { }`).
- In `page.rs` itself, the `verus!` block already defines for `PageAligned<T: Address +
  View<V = int>>`:
  - `View` with `type V = int`, `view(&self) == self.0@` (closed).
  - `inv(&self) == self@ % spec_page_size() == 0` (open spec).
- The in-scope functions `from_address` and `into_raw_value` currently have **no
  `#[verus_spec]`** annotations.
- Upstream `FrameAddress` (frame.rs) already carries specs that *depend on* this module and
  are marked `#[verus_verify(external_body)]` "until the address layer is verified":
  - `from_raw_value` ensures `Ok(fa) => fa.inv()`.
  - `into_raw_value` ensures `result as int == self@`.

### Assessment

- Coverage: **partial** — only the View + `inv` exist; the two target functions are
  unspecified.
- Strength: the View/`inv` design is **adequate and caller-abstract** (value = address,
  invariant = page alignment); it already matches how `FrameAddress` models itself.
- View design: **caller-abstract**, not biased — `FrameAddress` mirrors it verbatim,
  confirming `int` + page-alignment invariant is the right abstraction.
- Action for verification: add specs to `from_address`
  (`Ok(r) => r@ == addr@ && r.inv()`) and `into_raw_value` (`result as int == self@`) so
  the `external_body` shims on `FrameAddress` can eventually be discharged. Do **not** add
  `external_body` to these in-scope functions unless listed in
  `verus-ai-logs/tcb-allowed.md`.
