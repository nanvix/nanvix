# View Design: `FrameNumber` (`x86/mem/paging/frame/number.rs`)

In-scope (verification-order) targets: the type `FrameNumber` (its `View` +
`inv`), `FrameNumber::from_raw_value`, and `FrameNumber::into_raw_value`. All
other items (the unit tests, the `NULL`/`MAX` associated constants, the derived
`Debug`/`Clone`/`Copy`) are **out of scope** and untouched.

## Abstract Resource

`FrameNumber` is an **opaque, validated physical page-frame index** — a single
mathematical integer guaranteed to lie in `0..=MAX`, where
`MAX = MAX_ADDRESS / FRAME_SIZE - 1`. It is the *numerator* of a physical
address: `address = frame * FRAME_SIZE`. To every caller
(`PageTableEntry`, `PageDirectoryEntry`) its only observable state is that one
index; the module's entire job is to enforce the upper bound at construction so
consumers can compute `index << FRAME_SHIFT` (= `index * FRAME_SIZE`) without
overflowing `usize`.

It is *not* a collection, resource manager, or state machine: it is an
immutable, `Copy`, totally-ordered scalar token. The caller analysis confirms
callers "treat it as an opaque, always-valid token … never inspect or construct
it except through `from_raw_value`/`into_raw_value`", and "don't care about the
internal representation (a single `usize` newtype)".

### Consistency with the already-shipped downstream contract

The (verified) kernel crate already pins the *external* contract of this type in
`src/kernel/src/hal/mem/types/address/phys.spec.rs`. It used two **placeholder**
`assume_specification`s for `FrameNumber::from_raw_value`/`into_raw_value` (plus an
`external_type_specification` `ExFrameNumber`), expressed over two uninterpreted spec functions:

- `spec_frame_raw_value(frame: FrameNumber) -> int` — the frame's integer index,
- `spec_max_frame_number() -> int` — `FrameNumber::MAX`.

The native View defined here is chosen so that, once `arch` is verified, the real
specification **supersedes** those placeholders without breaking downstream
proofs: `frame@` plays the role of `spec_frame_raw_value(frame)`, and
`FrameNumber::spec_max()` plays the role of `spec_max_frame_number()`. As part of this phase the
kernel placeholders were removed and the two kernel helpers re-expressed as `open` definitions over
the arch quantities (`spec_frame_raw_value(frame) := frame@`,
`spec_max_frame_number() := FrameNumber::spec_max() as int`), so all existing kernel proof sites —
which reference the helper names — keep verifying unchanged. The `from_raw_value`/`into_raw_value`
transitions below are exactly the ensures the kernel already assumed.

## View Type

The abstract value of a `FrameNumber` is a **single integer**: the raw frame
index. There is exactly one caller-observable quantity, so the View is a scalar
`int`, not a one-field struct — mirroring the sibling abstractions in the address
tower (`PhysicalAddress@ : int`, `FrameAddress@ : int`, `VirtualAddress@ : int`).

```rust
impl View for FrameNumber {
    type V = int;

    // `closed`: callers may reference self@, but the mapping to the inner usize
    // field is hidden. The abstract value is "the frame index as int".
    closed spec fn view(&self) -> int {
        self.0 as int
    }
}
```

`self@ : int` is the entire abstract state. When `arch` is verified this `view`
is the concrete realization of the kernel's uninterpreted
`spec_frame_raw_value(frame)`, i.e. `spec_frame_raw_value(frame) == frame@`.

### Why `int`, not a struct, and not `nat`

- **Scalar, not struct** (view-design Step 2, *minimize fields*): there is only
  one caller-observable concept — the index. `MAX` is a **module-wide constant**,
  not per-value state, so it belongs in `inv()`/spec helpers, never the View. The
  physical address `index * FRAME_SIZE` is **derived**, so storing it would
  duplicate state and risk inconsistency. Wrapping a lone `int` in a struct adds
  ceremony to every spec for no gain.
- **`int`, not `usize`**: the View lives in spec world; using the machine type
  would reintroduce overflow reasoning into specs. `int` keeps the
  no-overflow-shift obligation (`index * FRAME_SIZE <= usize::MAX`) as a clean
  arithmetic fact in `inv()`.
- **`int`, not `nat`**: non-negativity (`0 <= self@`) holds structurally (the
  view comes from a `usize`) and is restated in `inv()`; `int` is chosen for
  uniformity with the rest of the address tower so cross-type lemmas
  (`PhysicalAddress`, `FrameAddress`) compose without `nat`/`int` coercions.

### Abstract bound

```rust
impl FrameNumber {
    // Models `FrameNumber::MAX = MAX_ADDRESS / FRAME_SIZE - 1`: the largest representable frame
    // index. A module-wide constant, not per-value state. It is `uninterp` (its concrete value
    // bottoms out at the build-time page-size constants, an external boundary that Verus does not
    // interpret) and `nat` (so the bound is non-negative for callers, e.g. `FrameNumber::NULL`).
    // Realizes the kernel's uninterpreted `spec_max_frame_number()`.
    pub uninterp spec fn spec_max() -> nat;
}
```

It is an *associated* spec function on `FrameNumber` (rather than a free function) so that
downstream verified crates reach it via the already-`pub`-exported type
(`arch::mem::paging::FrameNumber::spec_max()`), with no extra module re-exports needed.

The exec constant `FrameNumber::MAX` is left in a plain (non-`#[verus_verify]`) impl block — Verus
treats it as external — and is tied to `spec_max()` by a trusted contract in `number.spec.rs`:

```rust
pub assume_specification[ FrameNumber::MAX ] -> (result: usize)
    ensures
        result as int == FrameNumber::spec_max() as int,
;
```

This names the build-time page-size boundary without interpreting it, mirroring how the kernel
already treats the arch page-size constants (`PAGE_SIZE`/`FRAME_SIZE`) via `assume_specification`.

## Well-formedness Invariant

Every constructible `FrameNumber` satisfies exactly one universal property, and
it is the one every caller depends on: the index is bounded by `MAX`. This bound
is what guarantees `into_raw_value() << FRAME_SHIFT` cannot overflow `usize` —
the single most important guarantee in the caller analysis, relied on by
`PageTableEntry`/`PageDirectoryEntry` `into_raw_value` and `frame_address`.

It is enforced as a `#[verifier::type_invariant]` so the bound holds *unconditionally* for any
`FrameNumber` value a caller holds (matching the kernel's `into_raw_value` contract, which carries
the `0..=MAX` bound with no `requires`):

```rust
impl FrameNumber {
    #[verifier::type_invariant]
    pub open spec fn inv(&self) -> bool {
        0 <= self@ <= Self::spec_max()
    }
}
```


This is the minimal invariant the totality and overflow-safety obligations need.
There is no alignment or "non-null" invariant: `NULL` (= frame `0`) is a valid
in-range value, and frames carry no alignment notion (alignment lives on
addresses, not indices).

## Spec Transition Functions

`FrameNumber` is an immutable value type; the in-scope functions are a validating
constructor and a total projection, so the "transitions" relate inputs to the
resulting abstract `int`.

```rust
// Constructor: succeeds iff the raw value is a representable frame index, and on
// success the abstract value is exactly that input. Stated over the View domain,
// independent of the body being a range check + wrap.
//
//   value as int <= Self::spec_max()
//        ==> result is Some && result->Some_0@ == value as int
//   value as int >  Self::spec_max()
//        ==> result is None
//
// (Realizes the kernel's assumed `from_raw_value` ensures verbatim.)
```

```rust
// Projection: total, value-preserving, and the result is in-range.
//
//   result as int == self@
//   0 <= self@ <= Self::spec_max()           // i.e. self.inv()
//
// (Realizes the kernel's assumed `into_raw_value` ensures verbatim, including
// the in-range fact that underwrites the caller's `<< FRAME_SHIFT`.)
```

**Derived round-trip identity** (no separate clause needed — it follows from the
two transitions): for all `v <= MAX`, `from_raw_value(v)` yields a frame `f` with
`f@ == v`, hence `f.into_raw_value() as int == v`; and `from_raw_value(v) == None`
for all `v > MAX`. The caller analysis's round-trip and out-of-range guarantees
are thus both covered.

## Design Rationale (per field)

| View element | Why it's needed | Substitution test |
|--------------|-----------------|-------------------|
| `self@ : int` (raw frame index) | The sole caller-observable quantity; every caller obtains it via `into_raw_value` and immediately shifts it into a physical address. Constructor result and projection input/output are all stated in terms of it. | **Passes.** Any reimplementation (different representation, a `u64`, an offset, etc.) still exposes a frame index as its abstract value. |
| `inv(): 0 <= self@ <= MAX` | Underwrites totality of `into_raw_value` and the no-overflow `<< FRAME_SHIFT` in PTE/PDE; the type's entire reason to exist is enforcing this bound. | **Passes.** The `0..=MAX` bound is the type's contract regardless of how the check is implemented. |
| `FrameNumber::spec_max()` (bound) | Names the bound used by both `inv()` and the constructor's success/failure split. | **Passes.** `MAX` is a fixed architectural quantity, not an implementation artifact. |

All view-design Step-4 checks pass: **Substitution** (above), **Caller-only**
(index + bound are exactly the caller's mental model), **Complete** (round-trip,
out-of-range rejection, overflow-safety all expressible), **Minimal** (the single
field appears in every transition and the invariant), **No code-as-spec** (the
specs say *what* — "in range", "value-preserving" — not *how* the range check
runs).

## Rejected Alternatives

- **One-field struct `FrameNumberView { index: int }`** — rejected: a lone scalar
  needs no wrapper; the struct only adds `.index` noise to every spec and
  diverges from the sibling address-tower views.
- **Storing the physical address (`index * FRAME_SIZE`) as a field** — rejected:
  derived from the index and the module constant; storing it duplicates state and
  invites inconsistency. It belongs in `PhysicalAddress`/`FrameAddress`, not here.
- **Storing `MAX` as a View field** — rejected: it is a module-wide constant, not
  per-value state; it lives in `FrameNumber::spec_max()` and `inv()`.
- **`view : usize` (mirror the inner field)** — rejected: this is exactly the
  "mirroring internal fields" the methodology forbids; it drags machine-overflow
  reasoning into specs. The value *is* the abstraction, but its spec-world form
  must be `int`.
- **`view : nat`** — rejected: non-negativity is captured in `inv()`, and `int`
  composes with the existing `int`-valued address abstractions without coercions.
- **A "non-null" invariant (`self@ > 0`)** — rejected: `NULL` is frame `0`, an
  explicitly valid value; null-ness is a caller concern, not a type invariant.
- **Open `view()` exposing `self.0`** — rejected: `closed` keeps the
  newtype-to-`int` mapping out of callers' proofs (they reason only through the
  transitions), matching the sibling `PhysicalAddress`/`FrameAddress` views.
