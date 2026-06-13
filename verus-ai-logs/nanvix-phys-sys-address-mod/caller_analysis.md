# Caller Analysis: `mm::address::mod` (`Address` trait)

## Script Output

The automated finder (`find_callers_lsp.py`) was run on
`src/libs/sys/src/sys/mm/address/mod.rs`:

```
Total exec functions: 0
Public / trait-pub: 0
Private: 0
Types: 0
*No public functions found.*
```

**Why zero functions were reported:** `mod.rs` defines a single public *trait*,
`pub trait Address`, and the tree-sitter single-module parser does not enumerate
trait *method* declarations as callable functions. The verification-order target
functions — `is_aligned`, `into_raw_value`, `from_raw_value` — are **trait
methods**, dispatched through generic bounds (`T: Address`) and concrete
implementors. The script's caller list is therefore not usable as-is and was
enriched manually (per skill Step 1: "the script cannot find callers through
generic code `T: SomeTrait`").

The script confirms the crate (`sys`) is depended on by ~50 crates, but the
`Address` trait itself is consumed within the kernel HAL and the `sys` crate.

## Trait Obligations

- Trait: `Address` (supertraits: `Debug + Clone + PartialEq + Eq + PartialOrd +
  Ord + View<V = int>`). Abstract value is an `int` address (`self@`).
- Implementors found in the repo:
  - `VirtualAddress` — `src/libs/sys/src/sys/mm/address/virt.rs`
  - `PhysicalAddress` — `src/kernel/src/hal/mem/types/address/phys.rs`
  - `PageAligned<T: Address>` — `src/kernel/.../address/aligned/page.rs` (blanket impl, forwards to `T`)
  - `PageTableAligned<T: Address>` — `src/kernel/.../address/aligned/pgtab.rs` (blanket impl, forwards to `T`)

### Target-method semantics expected by all implementors/callers

- `from_raw_value(raw: usize) -> Result<Self, Error>`: validate `raw` and build
  an address whose `@ == raw`. On out-of-range input it must fail with
  `Error::BadAddress` (relied on by tests below).
- `into_raw_value(self) -> usize`: lossless inverse — `result as int == self@`
  (already specified on the trait).
- `is_aligned(&self, align) -> Result<bool, Error>`: returns
  `self@ % align_value == 0` (already specified on the trait).

## Callers and Call Sites

### Generic callers (`T: Address`)

- `PageAligned::<T>::from_address` (page.rs:51) calls `addr.is_aligned(PAGE_ALIGNMENT)?`
  — its own `ensures` (`Ok => spec_aligned(addr@)`, `Err => !spec_aligned(addr@)`)
  depends directly on `is_aligned` returning `self@ % align == 0`.
- `PageAligned::<T>` blanket `Address` impl (page.rs:64–88) forwards
  `into_raw_value`, `from_raw_value` (`Self::from_address(T::from_raw_value(raw)?)`),
  and `is_aligned` to the inner `T`.
- `PageTableAligned<T>` blanket impl (pgtab.rs:31,40–63,115) — identical forwarding
  pattern with `PGTAB_ALIGNMENT`.
- `MemoryRegion::<T>::new` (region.rs:179) calls `start.clone().into_raw_value()`
  to compute the region's end and compare against `T::max_addr()` with
  `checked_add`. Relies on `into_raw_value` being the exact raw address.
- `PageAligned::into_physical_address` (page.rs:202) round-trips
  `PhysicalAddress::from_raw_value(self.into_raw_value())?`.

### Test callers (de-facto specification, `src/kernel/.../address/test.rs`)

Generic over `T: Address`, instantiated for `VirtualAddress`, `PhysicalAddress`,
`PageAligned<VirtualAddress>`, `PageAligned<PhysicalAddress>`:

- `from_raw_value` success at `0` and at `max_addr()` (test:31,61,87).
- `from_raw_value(max_addr()+1)` must fail with `ErrorCode::BadAddress`
  (test:106–122,135,158,177). This pins the error path.
- `is_aligned`: aligned address → `Ok(true)`; unaligned → `Ok(false)`;
  mismatched (larger) alignment → `Ok(false)` (test:194–315).
- Round-trip: addresses built via `from_raw_value` are then checked with
  `is_aligned` (test:196–204,239–247,275–283), implicitly relying on
  `from_raw_value(r)@ == r`.

### Concrete implementor delegation

- `PhysicalAddress` (phys.rs:182,245,262) implements the trait by delegating to
  `VirtualAddress` (`from_virtual_address`, `self.0.is_aligned`,
  `self.0.into_raw_value`).
- `VirtualAddress` (virt.rs) provides the base inherent `from_raw_value` with
  `ensures result@ == raw_addr as int`.

## Caller Expectations

### `from_raw_value`
- Callers assume: on `Ok(a)`, `a@ == raw` (round-trips with `into_raw_value`);
  on out-of-range input, `Err` with `ErrorCode::BadAddress` (tests + the blanket
  impls that propagate the error via `?`).
- Callers don't care about: how validity/range is computed, the internal `usize`
  newtype layout, or any platform-specific max value beyond `max_addr()`.

### `into_raw_value`
- Callers assume: exact, total, lossless extraction — `result as int == self@`.
  `MemoryRegion::new` and the `PageAligned`/`PhysicalAddress` round-trips depend
  on this equality (no truncation, no error).
- Callers don't care about: the representation behind the value; only that
  `from_raw_value(into_raw_value(a))` reconstructs an equal address.

### `is_aligned`
- Callers assume: `Ok(b)` with `b == (self@ % align_value == 0)`. `PageAligned`/
  `PageTableAligned` construction correctness is derived entirely from this.
- Callers don't care about: how alignment is computed internally, only that
  `Err` is reserved for genuinely invalid alignments (current impls never error
  on valid `Alignment` values).

## Abstract Resource

The module manages a **machine address as an abstract integer** (`View = int`):
a typed, range-validated wrapper over a `usize` raw address, with a total order
and lossless raw-value conversion. Callers treat an `Address` purely as its
integer value `self@` plus alignment predicates over it.

## Key Invariants (caller perspective)

- Round-trip: `from_raw_value(r) == Ok(a)` ⇒ `a@ == r`; and `into_raw_value(a) as int == a@`.
- Range: `from_raw_value(r)` succeeds iff `r <= max_addr()`; otherwise
  `Err(BadAddress)`.
- Alignment: `is_aligned(a, k) == Ok(a@ % spec_align_value(k) == 0)`.
- Ordering/equality agree with `@` (needed by `MemoryRegion`, `PageAligned`).
- Blanket implementors (`PageAligned`, `PageTableAligned`) preserve the inner
  type's value (`@` is forwarded unchanged through the wrapper).

## Pre-existing Specs (from upstream verification)

- Source: specs already present on the `Address` trait in `mod.rs`.
- Functions WITH specs: `into_raw_value` (`result as int == self@`),
  `is_aligned` (`Ok(aligned) && aligned == (self@ % spec_align_value(align) == 0)`),
  `clone_address` (`result@ == self@`).
- Functions WITHOUT specs (in scope): `from_raw_value` — **no `ensures`** for
  the success value (`result@ == raw`) or the error path (`Err == BadAddress` /
  out-of-range). This is the main gap relative to caller/test expectations.
- View type: exists — `View<V = int>` is a supertrait requirement.

### Assessment
- Coverage: **partial**. `into_raw_value`/`is_aligned` are specified; the
  in-scope `from_raw_value` is unspecified despite tests pinning its success and
  error contracts.
- Strength: `is_aligned` ensures only the `Ok` arm (no statement that valid
  alignments never error); `from_raw_value` has no ensures at all.
- View design: **caller-abstract** — `int` is the natural integer model callers
  use; no implementation detail leaks. Recommended `from_raw_value` ensures
  (caller-justified): `Ok(a) ⇒ a@ == raw && raw <= max_addr()`,
  `Err ⇒ raw > max_addr()` / `BadAddress`.

## Notes / Constraints

- In-scope functions only: `is_aligned`, `into_raw_value`, `from_raw_value`.
  Other trait methods (`align_up`, `align_down`, `max_addr`, `clone_address`,
  `as_ptr`, `as_mut_ptr`) were not modified.
- No `external_body` added (none listed in `verus-ai-logs/tcb-allowed.md`).
