# View Design: `PageAligned<T>` (`hal/mem/types/address/aligned/page.rs`)

## Abstract Resource

`PageAligned<T>` is a **single memory address carrying a type-level proof that it
is aligned to a page boundary** — a validated newtype over an `Address`. To a
caller, its only observable state is the address value itself (an `int`); its
*meaning* is the page-alignment guarantee that lets the frame / region / vmem /
page-table layers treat it as aligned without re-checking.

In-scope (verification-order) targets: `PageAligned::from_address`,
`PageAligned::into_raw_value`, and the type `PageAligned` (its `View` + `inv`).

---

## View Type

The abstract value of a `PageAligned<T>` is a **single mathematical integer**: the
raw page-aligned address. A `struct` View with named fields is **not** used here —
there is exactly one caller-observable quantity, so the View is a scalar.

```rust
impl<T: Address + View<V = int>> View for PageAligned<T> {
    type V = int;

    // `closed`: callers may reference self@, but the mapping to the inner
    // field is hidden. The abstract value is "the address as an int".
    closed spec fn view(&self) -> int {
        self.0@
    }
}
```

`self@ : int` is the entire abstract state. This is exactly the model the already
verified `FrameAddress` mirrors (`FrameAddress@ == self.0@`, where
`self.0 : PageAligned<PhysicalAddress>`), confirming the abstraction is
caller-driven, not implementation-driven.

### Why `int`, not a struct

Per the view-design skill (Step 2, *minimize fields*): every field must correspond
to a caller-observable abstract concept, and there is only one here — the address.
The page size / alignment is a **module-wide constant** (`spec_page_size()`), not
per-value state, so it belongs in `inv()`, not the View. Wrapping a lone `int` in a
one-field struct would add ceremony to every spec with no gain in expressiveness.

---

## Well-formedness Invariant

```rust
impl<T: Address + View<V = int>> PageAligned<T> {
    // `open`: callers (FrameAddress, region, vmem, page tables) must be able to
    // read and rely on the page-alignment fact directly in their own proofs.
    pub open spec fn inv(&self) -> bool {
        self@ % spec_page_size() == 0
    }
}
```

`inv()` states the one carried property: **every constructible `PageAligned`
value is page-aligned**. This is the proof obligation `from_address` must
establish and the fact `FrameAddress::inv` (`self@ % spec_page_size() == 0`)
delegates to.

---

## Spec Transition Functions

`PageAligned` is an immutable validated newtype: neither in-scope function mutates
state. The "transitions" are therefore relations between an input address value
and the resulting abstract value, expressed as spec helpers on the View's `int`
domain.

```rust
// Success condition for the validating constructor, stated purely on the
// abstract address value. `from_address` validates, it does NOT normalize:
// success requires the *input* to already be page-aligned.
pub open spec fn spec_aligned(addr_view: int) -> bool {
    addr_view % spec_page_size() == 0
}
```

### `from_address(addr: T) -> Result<Self, Error>` — validating constructor

```text
ensures
    match result {
        Ok(r)  => spec_aligned(addr@) && r@ == addr@ && r.inv(),
        Err(_) => !spec_aligned(addr@),
    }
```

- **Value-preserving, not rounding**: `Ok(r) => r@ == addr@`. The address is
  carried through unchanged; an unaligned input is *rejected* (`Err`), never
  silently aligned down/up.
- **Establishes the invariant**: `Ok(r) => r.inv()` — the fact every downstream
  layer (`FrameAddress::from_raw_value`, region, vmem) builds on.
- **Bidirectional failure**: `Err <=> !spec_aligned(addr@)`. The error condition is
  the abstract negation of the success condition, not a list of code checks; the
  concrete `Error`/`ErrorCode` payload is deliberately unconstrained (callers don't
  inspect it — they propagate with `?`).

### `into_raw_value(self) -> usize` — total projection

```text
ensures
    result as int == self@
```

- A faithful, total, side-effect-free projection of the abstract address.
- Matches the upstream `FrameAddress::into_raw_value` spec (`result as int ==
  self@`), whose body is literally `self.0.into_raw_value()`.
- Because the receiver already satisfies `inv()`, callers may additionally derive
  `result % spec_page_size() == 0`, but value-equality is the core promise; no
  separate clause is needed (it is implied by `inv(self)` + this ensures).

---

## Design Rationale (per the substitution test)

> *If the implementation were completely rewritten with a different algorithm,
> would this still make sense?*

**Abstract value `self@ : int` = the raw aligned address.** ✅ Survives rewrite.
If `PageAligned` stored a frame index (`addr >> page_shift`), a tuple
`(base, offset)`, or any other encoding, the *address it denotes* is unchanged, so
`view() -> int` (the address) is still the right abstraction. `into_raw_value`'s
callers (`elf.rs:288`, `FrameAddress::into_raw_value`) consume this as a byte
address `usize`, so the View must denote the address, not an internal encoding.

**Invariant `self@ % spec_page_size() == 0`.** ✅ Survives rewrite. Page alignment
is the defining property of *any* `PageAligned`, independent of how the value is
stored or how `from_address` checks it (`is_aligned(PAGE_ALIGNMENT)`, a mask, a
modulo). It is the single guarantee 171 type-reference sites rely on.

**`from_address` modeled as validate-and-preserve.** ✅ The contract
(`Ok => r@ == addr@`, `Err <=> unaligned`) is declarative: it pins *what*
(reject-or-preserve) without prescribing *how* the alignment test is performed.
Swapping the check mechanism leaves the spec valid.

Both `view` and `inv` already exist in the source and the analysis judges them
**adequate and caller-abstract**; this design ratifies them and supplies the two
missing function-level transitions so the `external_body` shims on `FrameAddress`
can eventually be discharged.

---

## Quality Review

| Criterion | Result |
|-----------|--------|
| **Substitution** | ✅ `int` address + page-alignment `inv` both survive a full reimplementation. |
| **Caller-only** | ✅ Address value and "is page-aligned" are exactly what callers reason about; no inner-field, `Error` payload, or check-mechanism leaks. |
| **Complete** | ✅ The two caller-observable concepts (the address; its alignment) are both represented; `from_address`/`into_raw_value` specs cover every caller expectation in the analysis. |
| **Minimal** | ✅ One scalar value + one invariant; `self@` is used by both target specs and by `FrameAddress`; `inv()` is used by `from_address` and all downstream layers. |
| **No code-as-spec** | ✅ `inv` is a modulo predicate, not the `is_aligned` algorithm; `from_address` spec is a relation on `addr@`, not a transcription of the validation body. |

---

## Rejected Alternatives

- **View as a frame index (`self.0@ / spec_page_size()`).** Rejected: callers
  (`into_raw_value` at `elf.rs:288`, `FrameAddress::into_raw_value`) expect the
  raw byte **address**, not a frame number. A frame-index View would force every
  caller to multiply back by the page size and would mismatch `FrameAddress`'s
  existing `self@`-as-address model. Fails *direct usability*.

- **A multi-field struct `{ addr: int, alignment: int }`.** Rejected: `alignment`
  is the module constant `spec_page_size()`, not per-value caller-observable state.
  Encoding it as a field would duplicate a constant into every value and complicate
  every spec. It belongs in `inv()`. Fails *minimal*.

- **A `bool is_aligned` field in the View.** Rejected: it would always be `true`
  (guaranteed by `inv()`), so it carries no information and is subsumed by the
  invariant. Fails *minimal* / *no redundant fields*.

- **Exposing the inner `T` (e.g. `view() -> T::V` nested, or a `PhysicalAddress`
  wrapper).** Rejected: `T: View<V = int>` already collapses to `int`; surfacing
  the wrapper type would leak the newtype's internal layering without adding any
  caller-visible concept. The address-as-`int` View is the common denominator both
  `PhysicalAddress` and `VirtualAddress` instantiations need.

- **Modeling `from_address` as normalizing (align-down on input).** Rejected: the
  caller analysis is explicit that `from_address` *validates, not normalizes*
  (`Ok(r) => r@ == addr@`; unaligned inputs `Err`). A normalizing model would let
  `Ok` be returned for an unaligned input with a silently changed value, breaking
  the `inv()`-preservation that `FrameAddress`/region/vmem depend on. Fails
  *sufficient to reject bugs*.

- **Leaving the `Error` payload constrained (e.g. `Err(BadAddress)`).** Rejected as
  over-specification: no caller inspects the variant (all propagate with `?`).
  Pinning it would couple callers to an implementation detail and fail the
  substitution test.
