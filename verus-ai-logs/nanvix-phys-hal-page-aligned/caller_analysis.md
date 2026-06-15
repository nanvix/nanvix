# Caller Analysis: `PageAligned<T>` (`hal/mem/types/address/aligned/page.rs`)

## Script Output
See: `verus-ai-logs/nanvix-phys-hal-page-aligned/find_callers_lsp_output.md`
(raw `find_callers_lsp.py` report, rust-analyzer LSP, intra-crate `kernel`).

Module summary: 17 exec functions, all `pub`/`trait-pub`, 0 private, 1 type
(`PageAligned`). No external crates depend on `kernel`, so all callers are
intra-crate.

### Verification-order targets (only functions in scope)
| Function | Visibility | External call sites | Internal call sites |
|----------|-----------|--------------------:|--------------------:|
| `PageAligned::from_address` | `pub` | 20 | 5 |
| `PageAligned::into_raw_value` (`impl Address`) | trait-pub | 62 | 1 |
| `PageAligned<T>` (type) | `pub` | 176 references | — |

These three are the heaviest-used surface of the module: `from_address` is the
sole aligned-constructor, `into_raw_value` is the dominant read accessor, and
`PageAligned<T>` is threaded through almost every memory-management subsystem
(`mm::phys`, `mm::virt`, `pm::process`, `hal::io::mmio`, `hal::arch::…mmu`).

## Trait Obligations

- Trait `Address` (`src/libs/sys/src/sys/mm/address/mod.rs`) — `PageAligned<T>`
  implements it by delegating to the inner `T: Address`. Caller-relevant members:
  - `into_raw_value(self) -> usize` — expected to return the underlying numeric
    address with **no transformation** (newtype identity). Callers subtract two
    raw values to get in-page offsets and add `PAGE_SIZE` multiples to walk pages,
    so the result must equal the abstract address.
  - `from_raw_value(raw) -> Result<Self, Error>` — delegates to
    `from_address(T::from_raw_value(raw)?)`; inherits the page-alignment check.
  - Supertraits `Eq`/`Ord`/`Debug` — `PageAligned` forwards `eq`/`cmp` to the
    inner value, so ordering/equality must agree with the underlying address.
- Trait `Deref<Target = T>` — compiler-dispatched; lets callers transparently use
  inner-`T` (`VirtualAddress`/`PhysicalAddress`) methods on a `PageAligned`.

## Caller Expectations

### `PageAligned::from_address(addr: T) -> Result<Self, Error>`
Representative callers: `hal/mem/types/region.rs:340`, `mm/virt/vmem.rs:978/1053/1356`,
`mm/elf.rs:284`, `hal/mem/types/address/frame.rs:78/86`,
`hal/arch/shared/mem/mmu/page_{directory,table}.rs`.

- Callers assume on **success** (`Ok`): the wrapped address is page-aligned, i.e.
  `result@ % PAGE_SIZE == 0`. This is the invariant the whole type exists to carry;
  downstream code (e.g. offset math, frame/page conversions, MMU mapping) relies on
  it without re-checking.
- Callers assume the returned value's address is **unchanged** — `from_address`
  validates, it does not round/normalize. The result's raw value equals `addr`'s
  raw value (`result@ == addr@`).
- Many callers pre-align with `align_down(PAGE_ALIGNMENT)` then call
  `from_address(...)?` (e.g. `vmem.rs`, `identity_map.rs:417`); they treat the `?`
  as "can't fail because I just aligned", but still rely on the alignment guarantee.
- Callers assume on **failure** (`Err`): `Error::BadAddress` when `addr` is not
  page-aligned, with no side effects (value-type, nothing to roll back).
- Callers **don't care about**: the internal newtype representation
  (`struct PageAligned<T>(T)`), the exact error message string, or how the
  alignment check is implemented (`addr.is_aligned(PAGE_ALIGNMENT)`).

### `PageAligned::into_raw_value(self) -> usize` (impl `Address`)
Representative callers: `mm/virt/vmem.rs` offset math (`:979,1054,1186,1357,1363`),
`hal/io/mmio/allocator.rs:189/193`, `hal/arch/.../xapic.rs`, `ioapic.rs:78`,
`pm/process/manager/unsafe.rs:381/382`, `mm/virt/manager.rs:633`.

- Callers assume the returned `usize` **equals the abstract address**
  (`result as int == self@`) — pure identity projection, no masking/shifting. This
  is critical: `vaddr.into_raw_value() - page_aligned.into_raw_value()` must yield
  the true in-page offset (`0..PAGE_SIZE`), and `addr.into_raw_value() + k*PAGE_SIZE`
  must address the k-th following page.
- Because `self` is page-aligned, callers further assume `result % PAGE_SIZE == 0`
  (used implicitly when the result is fed to MMIO base registers, `*mut u8` casts at
  `vmem.rs:1419`, and re-wrapped via `from_raw_value`).
- Callers assume it is **total** (no `Result`) and **side-effect free** (consumes
  `self` by value).
- Callers **don't care about** whether the value comes from `VirtualAddress` or
  `PhysicalAddress` inner storage — only that it round-trips with `from_raw_value`.

### Type `PageAligned<T>` (176 references)
- Callers use it as a **type-level proof token**: holding a
  `PageAligned<VirtualAddress>` / `PageAligned<PhysicalAddress>` means "this address
  is page-aligned" and that fact need not be re-verified. It appears in many public
  signatures (`region.start()`, `MmioRegion::base()`, `UserStack::base()/top()`,
  `Vmem` user/kernel address checks, process-manager APIs).
- Conversions callers rely on: `into_inner()` (unwrap to inner `T`, 21 sites),
  `into_virtual_address()` / `into_physical_address()` (alignment-preserving
  P↔V conversion), and `Deref` to reach inner-address helpers.
- Callers **don't care about** the single-field tuple-struct layout, only that the
  alignment invariant is preserved by every operation that returns a `PageAligned`.

## Abstract Resource
`PageAligned<T>` is a **validated wrapper around a memory address (`int`) that
guarantees the address lies on a page boundary**. It is the kernel's compile-time
witness of page alignment, threaded through physical/virtual memory management,
MMU paging, MMIO regions, and process stacks.

## Key Invariants (caller perspective)
- **Alignment:** for any `p: PageAligned<T>`, `p@ % spec_page_size() == 0`. Every
  constructor (`from_address`, `from_raw_value`, `align_up`, `align_down`,
  `into_virtual_address`, `into_physical_address`) preserves it; this matches the
  existing `inv()` in the module's `verus!` block.
- **Identity / round-trip:** `into_raw_value(self) as int == self@`, and
  `from_address(a)` (on success) yields `result@ == a@`. So
  `from_raw_value(p.into_raw_value()) == Ok(p)` for aligned `p`.
- **Order consistency:** `eq`/`cmp` agree with the inner address ordering
  (`p1 <= p2  ⇔  p1@ <= p2@`).
- **Failure is pure:** `from_address`/`from_raw_value` return
  `Err(BadAddress)` exactly when the input is unaligned, with no mutation.

## Pre-existing Specs (from upstream verification)
- Source: `View`/`inv` added on `PageAligned<T>` during prior verification of the
  `hal::mem` address types (mirrors `FrameAddress` in
  `hal/mem/types/address/frame.rs`).
- View type: **exists** — `impl<T: Address + View<V=int>> View for PageAligned<T>`
  with `type V = int` and `view(&self) == self.0@` (the raw address as `int`).
- Invariant: `inv(&self) == (self@ % spec_page_size() == 0)`.
- `page.spec.rs` / `page.proof.rs` are present but **empty** (`verus! { }`); no
  per-function `requires`/`ensures` or `assume_specification` are attached yet.
- Functions with `#[verus_spec]` ensures: **none** in this module.
- Functions WITHOUT specs: all 17 (including the in-scope `from_address`,
  `into_raw_value`).

### Assessment
- Coverage: **partial** — only the View + `inv` skeleton exists; the in-scope
  functions have no `ensures`.
- Strength: **weak** — no error-path or identity ensures yet. The sibling
  `FrameAddress::into_raw_value` already models the intended contract
  (`#[verus_spec] ensures result as int == self@`, under an allow-listed
  `external_body`); `PageAligned::into_raw_value` should expose the same identity
  ensures, and `from_address` should ensure `result@ == addr@` plus the alignment
  invariant on the `Ok` branch.
- View design: **caller-abstract** — `view = self@ = int` (raw address) is exactly
  what all callers depend on (offset arithmetic, raw-value round-trips, ordering);
  it is not biased toward any single caller and matches the surrounding address
  modules.

### Notes / constraints
- No new `external_body` may be added here: neither `from_address` nor
  `into_raw_value` is listed in `verus-ai-logs/tcb-allowed.md`. (The allow-list
  covers `FrameAddress`/`mm::phys` items, not `PageAligned`.) Specs for the
  in-scope functions must be discharged by proof, not trusted bodies.
- Out-of-scope (do not modify): `into_inner`, `from_raw_value`, `align_up`,
  `align_down`, `is_aligned`, `max_addr`, `as_ptr`, `as_mut_ptr`, `eq`, `fmt`,
  `cmp`, `partial_cmp`, `deref`, `into_physical_address`, `into_virtual_address`.
