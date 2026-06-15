# Caller Analysis: `sys::mm::address::mod` (the `Address` trait)

## Script Output

`find_callers_lsp.py` reported **0 exec functions / 0 pub functions** for this file:

```
Total exec functions: 0
Public / trait-pub:    0
Private:               0
Types:                 0
"No public functions found."
```

This is a **tree-sitter limitation, not an absence of callers**: `mod.rs` defines
no free functions — it declares the **`pub trait Address`** (and re-exports the
`virt` submodule). The verification-target functions in scope
(`is_aligned`, `into_raw_value`, `from_raw_value`) are **trait methods**, which
the single-module LSP pass does not resolve as call sites. Analysis below was
done by hand over the whole `src/` tree.

The script did confirm the crate's reach: `sys` is depended on by 50+ crates
(`kernel`, `arch`, `syscall`, `sysalloc`, `uservm`, `nvx`, `elf`, all daemons,
and the test crates), so this trait is a wide library edge.

## Trait Obligations

- **Trait: `Address`** (`mod.rs:37`) — bound `Debug + Clone + Copy + Eq + Ord`.
  Models a **pointer-sized memory address** as an abstract integer. In-scope
  methods:
  - `from_raw_value(raw: usize) -> Result<Self, Error>` — *associated*
    constructor/validator. `Ok` ⟹ the value is a legal address of this type and
    its abstract value equals `raw`; `Err(BadAddress)` ⟹ the value is outside the
    type's domain. Expected inverse of `into_raw_value`.
  - `into_raw_value(self) -> usize` — total projection to the raw numeric
    address. Pure newtype identity: `result as int == self@`. Never fails.
  - `is_aligned(&self, align: Alignment) -> Result<bool, Error>` — `Ok(true)` iff
    the address is a multiple of `align`. `Err` reserved for failure (concrete
    impls never error in practice).
  - (out of scope, for context) `align_up`, `align_down`, `max_addr`, `as_ptr`,
    `as_mut_ptr`.

### Implementors (the de-facto specification consumers)
- `VirtualAddress` — `virt.rs:174`, **this module**. Thin infallible newtype over
  `usize`; `from_raw_value` always returns `Ok`. `into_raw_value` returns the
  inner field verbatim.
- `PhysicalAddress` — `kernel .../address/phys.rs:215`. `from_raw_value` is
  **fallible** (validates RAM/frame-range via `from_virtual_address`).
- `PageAligned<T: Address>` — `kernel .../aligned/page.rs:79`. `from_raw_value`
  delegates to `T::from_raw_value` then **requires `is_aligned(PAGE_ALIGNMENT)`**;
  `into_raw_value`/`is_aligned` forward to the inner `T`.
- `PageTableAligned<T: Address>` — `kernel .../aligned/pgtab.rs:38`. Same shape,
  `PGTAB_ALIGNMENT`.

## Caller Expectations

### `from_raw_value` (associated constructor)
- **Callers assume**
  - On `Ok(a)`: `a` is a valid address of that type and `a.into_raw_value()`
    round-trips back to the input (`a@ == raw`). Refined types additionally
    guarantee their domain invariant holds — `PageAligned`/`PageTableAligned` ⟹
    aligned, `PhysicalAddress` ⟹ representable frame.
  - On `Err`: `Err(Error::BadAddress)` and no address is produced; the value
    violated the domain (unaligned, or out-of-range).
  - It is the **inverse of `into_raw_value`** — pervasive round-trip pattern, e.g.
    `PageAligned::from_address(PhysicalAddress::from_raw_value(self.into_raw_value())?)`
    (`page.rs:214`), `pd.rs:26`, `page_directory.rs:194`.
  - Generic delegation through `T: Address` works for any implementor:
    `Self::from_address(T::from_raw_value(raw_addr)?)` (`page.rs:99`,
    `pgtab.rs:58`).
  - Errors propagate cleanly with `?` (callers thread `Result<_, Error>`).
- **Callers don't care about**: how validity is checked, the internal newtype
  layout, or whether `VirtualAddress` can actually fail (its `Ok`-always behavior
  is not relied on by generic code).

### `into_raw_value`
- **Callers assume**
  - Returns the **exact raw `usize` address**; pure, total identity
    (`result as int == self@`) — this is pinned verbatim as a trust boundary in
    `kernel .../phys.spec.rs:107` (`assume_specification[<VirtualAddress as
    Address>::into_raw_value] ensures result as int == addr@`).
  - The raw value is used for **pointer casts** (`fbase.into_raw_value() as
    *const c_void`, `dladdr.rs:52`), **address arithmetic**
    (`load_address.into_raw_value() + phdr.p_vaddr`, `dynlib.rs:198`; bounds
    checks, offsets, `checked_add`), region bounds (`region.rs:179`), and as a
    bridge to re-wrap via `from_raw_value`.
  - Total — never fails, no panic.
- **Callers don't care about**: storage details or the `as_ptr`/`as_mut_ptr`
  siblings (which is precisely why this method is held as an
  `assume_specification` rather than body-verified — see note at `virt.rs:260`).

### `is_aligned`
- **Callers assume**
  - `Ok(true)` iff the address is a multiple of `align`; used as an
    **alignment guard** before treating an address as page/pgtab-aligned:
    `if !addr.is_aligned(PAGE_ALIGNMENT)? { return Err(BadAddress) }`
    (`page.rs:67`, `pgtab.rs:30`, `mprotect.rs:74`, `munmap.rs:66`,
    `segment.rs:83`, `heap.rs:48`) and `debug_assert!(start.is_aligned(...))`
    (`heap.rs:194`, `segment.rs:247`).
  - Result is consistent with `align_up`/`align_down` (an aligned address is
    unchanged by them) — exercised by the generic tests
    `test_is_aligned_*<T: Address>` and `test_align_*<T: Address>`
    (`kernel .../address/test.rs`).
  - `?`-propagatable; the boolean is the only payload callers branch on.
- **Callers don't care about**: whether alignment is computed by shift, mask, or
  modulo; the `Err` arm (no concrete implementor returns it).

## Abstract Resource

The `Address` trait models a **pointer-sized location in an address space**,
abstracted as a single mathematical integer (`self@ : int`, `0 <= self@ <=
usize::MAX`). The in-scope methods are its **raw-value boundary** (validate/
construct from a `usize`, project back to a `usize`) and an **alignment query**.
Refinement implementors (`PageAligned`, `PageTableAligned`, `PhysicalAddress`)
layer a domain predicate on top without changing the underlying integer view.

## Key Invariants (caller perspective)

- **Identity / round-trip**: `into_raw_value` is total identity
  (`result as int == self@`); `from_raw_value(into_raw_value(a)) == Ok(a)` and,
  on `Ok`, `from_raw_value(v) ⟹ result@ == v`. This pairing is the single most
  relied-on fact across `kernel`/`arch`/`syscall`.
- **Domain validation on construction**: `from_raw_value` returns `Ok` only when
  the value is a legal address for the type; otherwise `Err(BadAddress)`. For
  refined types `Ok` additionally implies the refinement (aligned / frame-
  representable).
- **Alignment semantics**: `is_aligned(a, k) == Ok(a@ % k == 0)`; consistent with
  `align_up`/`align_down` (idempotent on already-aligned addresses).
- **Totality**: `into_raw_value` and (for the concrete impls) `is_aligned` never
  panic and never spuriously error.

## Pre-existing Specs (from upstream verification)

- **Source**: added while verifying `kernel` (`phys` / `frame` / `page`) and the
  `sys` `virt` constructors top-down.
- **`sys` side (`virt.rs` / `virt.spec.rs` / `mod.spec.rs`)**:
  - `VirtualAddress::new` and the **inherent** `VirtualAddress::from_raw_value`
    carry `#[verus_spec]`: `ensures result@ == value as int && result.inv()`.
  - `inv()` defined: `0 <= self@ <= usize::MAX` (open).
  - **`mod.spec.rs` / `mod.proof.rs` are empty** (`verus! { }`): the `Address`
    *trait* methods themselves carry **no specs** yet.
  - View type for `VirtualAddress`: `int` (defined in `phys.spec.rs`'s sibling
    pattern / used via `self@`).
- **Functions WITH specs**: `VirtualAddress::new`, inherent
  `VirtualAddress::from_raw_value`.
- **Functions WITHOUT specs (in scope)**: the trait methods
  `Address::from_raw_value`, `Address::into_raw_value`, `Address::is_aligned`
  (and the `impl Address for VirtualAddress` bodies). `into_raw_value` is instead
  pinned externally via `assume_specification` in `kernel .../phys.spec.rs`.

### Assessment
- **Coverage**: *partial*. Only the inherent `new`/`from_raw_value` constructors
  are spec'd, written from the `kernel` frame allocator's needs (it needs
  `result@ == value`). The trait-level methods that generic callers
  (`PageAligned<T>`, `MemoryRegion<T>`, the `<T: Address>` tests) dispatch through
  are unspec'd.
- **Strength**: *weak on error paths*. Existing ensures cover only the success
  value-identity. There is no ensures for the `Err(BadAddress)` arm of
  `from_raw_value`, nor for `is_aligned`'s boolean ↔ `self@ % align == 0`
  correspondence — both of which guard callers (`mprotect`/`munmap`/`heap`).
- **View design**: *caller-abstract and sound*. `VirtualAddress@ : int` is the
  right minimal model; it is not biased toward any one consumer. The concern is
  not the View but the **trust-boundary placement**: `into_raw_value` is
  `assume_specification`'d in `kernel` only because the whole `impl Address for
  VirtualAddress` block can't be body-verified (sibling `as_ptr`/`as_mut_ptr`
  use unsupported `usize as *const u8` casts). A View design for this module
  should give the trait methods native ensures (identity for `into_raw_value`,
  `Ok ⟹ result@ == raw` for `from_raw_value`, `Ok(self@ % align == 0)` for
  `is_aligned`) so downstream crates can drop their local `assume_specification`.
