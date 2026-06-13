# Caller Analysis: `mm::address::virt` (`VirtualAddress`)

## Script Output
See: `verus-ai-logs/nanvix-phys-sys-virt-address/find_callers_lsp_output.md`
(raw output of `scripts/find_callers_lsp.py`).

Scope (verification-order target functions):
`VirtualAddress::into_raw_value`, `VirtualAddress::from_raw_value`,
`VirtualAddress::new`, and the type `VirtualAddress` itself. Other functions in
the file are out of scope and were not analyzed for expectations.

### Summary of script findings (in-scope items only)
| Item | Visibility | External callers |
|------|-----------|-----------------:|
| `VirtualAddress::new` (inherent) | pub const | 8 |
| `VirtualAddress::from_raw_value` (inherent) | pub | 5 |
| `Address::into_raw_value` for `VirtualAddress` | trait-pub | 3 |
| type `VirtualAddress` | pub | 32 references |

`VirtualAddress` is a single-field newtype `pub struct VirtualAddress(usize)`.
The crate `sys` is depended on by ~50 crates (kernel, daemons, user apps), so
the type is part of a widely-shared public ABI surface.

## Trait Obligations
- Trait: `mm::Address` (implemented for `VirtualAddress`).
  - `into_raw_value(self) -> usize` — expected semantics: return the underlying
    machine address as a plain integer, exactly as it was stored. Callers use it
    as the *inverse* of construction.
  - The trait also declares a `from_raw_value(usize) -> Result<Self, Error>`
    variant (infallible here, always `Ok`), `max_addr`, alignment helpers,
    `as_ptr`/`as_mut_ptr`, `clone_address` — out of scope but relevant context:
    the contract of `Address` is "a thin, total wrapper around a `usize`
    address that can always be converted back to its raw value."

## Caller Expectations

### `VirtualAddress::new(value: usize) -> Self`
Call sites: `config.rs` (memory-layout constants `KERNEL_BASE`, `KERNEL_END`,
`USER_BASE`, `USER_END`, `USER_STACK_BASE`, `USER_MMAP_BASE`, `USER_MMAP_END`),
`pm/thread_create_args.rs` (`NULL_USER_FN = VirtualAddress::new(0)`), and
internally by `from_raw_value`, `align_up`, `align_down`, `add`, `From<u32>`.
- Callers assume:
  - It is a `const fn` usable in `const` initializers (compile-time constants).
  - It is **total / infallible** — accepts *any* `usize` (including `0` and
    `usize::MAX`) with no validation, masking, or panic.
  - It is a pure wrapper: the stored value equals the argument, so a later
    `into_raw_value`/`From<…>` returns the same bits (`view() == value`).
- Callers don't care about:
  - The internal representation (that it is a tuple newtype `(usize)`).
  - Any normalization/canonicalization of the address — none is expected.

### `VirtualAddress::from_raw_value(raw_addr: usize) -> Self`
Call sites: `mm/mmio.rs:126` (`MmioRegionInfo::base` reconstructs a
`VirtualAddress` from a stored `u32` widened to `usize`), `pm/sync.rs:30,58`
(`From<usize>` for `MutexAddress`/`ConditionAddress`),
`pm/thread_create_args.rs:44,47` (`Default` fills `user_fn`/`user_stack_base`
with address `0`). Internally used by `checked_add`/`checked_sub`/`Address::from_raw_value`.
- Callers assume:
  - Same total/infallible wrapping as `new` (it simply forwards to
    `VirtualAddress::new(raw_addr)`).
  - Round-trip identity: `from_raw_value(x).into_raw_value() == x` for all
    `x: usize`. This is the property `sync.rs` relies on, since it wraps a
    `usize` and later unwraps it via `From<…> for usize`.
- Callers don't care about:
  - Distinction between `new` and `from_raw_value` — they are interchangeable
    constructors from the caller's view.

### `Address::into_raw_value(self) -> usize`
Call sites: `mm/mmio.rs:67` (`u32::try_from(base.into_raw_value())` to range-
check/narrow the base to 32 bits), `pm/sync.rs:37,65` (`From<MutexAddress>` /
`From<ConditionAddress> for usize` unwrap the stored address).
- Callers assume:
  - It returns exactly the `usize` that was used to construct the address
    (no offsetting, masking, or loss) — the inverse of `new`/`from_raw_value`.
  - It is total and consuming (`self` by value), never fails or panics.
  - The result is suitable for further numeric handling (e.g. `u32::try_from`,
    storing back into a `usize` field).
- Callers don't care about:
  - How the value is stored internally; only that the bits round-trip.

### type `VirtualAddress`
Used as a field type (`MutexAddress`, `ConditionAddress`, `ThreadCreateArgs`,
`MmioRegionInfo::base`) and as the type of `const` layout values.
- Callers assume:
  - It is `Copy`/`Clone`/`Eq`/`Ord` (derived) — usable as a plain value, in
    comparisons and as struct fields, and constructible in `const` context.
  - It is layout-compatible with `usize` (size asserted equal to `u32` on
    32-bit targets via `static_assert::assert_eq_size!`).
- Callers don't care about:
  - That it is a newtype around `usize` specifically vs. any other identical
    representation.

## Abstract Resource
This module manages a **virtual memory address**: an opaque, total newtype
wrapper around a machine-word (`usize`) integer. From the caller's perspective
it is just "a `usize` with a distinct type tag" that can always be built from a
raw value and always converted back to that exact raw value.

## Key Invariants (caller perspective)
- **Round-trip identity:** for all `x: usize`,
  `VirtualAddress::new(x).into_raw_value() == x` and
  `VirtualAddress::from_raw_value(x).into_raw_value() == x`.
- **Constructor equivalence:** `new` and `from_raw_value` produce the same value
  for the same input (`from_raw_value` delegates to `new`).
- **Totality:** all three functions are infallible for every `usize` input;
  no panic, no validation, no masking, no overflow behavior.
- **Purity / no hidden state:** the value depends only on its construction
  argument; equality of two `VirtualAddress` values iff their raw values are equal.
- **View:** the natural abstract model is the integer value of the wrapped
  `usize` (the existing `View for VirtualAddress` maps `self.0 as int`), and the
  three in-scope functions should be specified relative to that single integer.

## Pre-existing Specs (from upstream verification)
- Source files `virt.spec.rs` and `virt.proof.rs` exist but are **empty**
  (`verus! { } // verus!`).
- `grep verus_spec virt.rs` → no matches; no `#[verus_spec]` annotations on any
  function.
- View type: **exists** — `impl View for VirtualAddress { type V = int; ... }`
  (a `closed spec fn view` returning `self.0 as int`).
- Functions with specs: none. Functions WITHOUT specs: all (including the four
  in-scope items).
- Assessment: no upstream caller has constrained these functions yet, so the
  View/spec design is unbiased and can be derived directly from the
  caller-expectation invariants above. The existing `int` View matches the
  abstract resource and is suitable for specifying `new`/`from_raw_value`
  (ensures `result@ == value`) and `into_raw_value` (ensures `result == self@`).
