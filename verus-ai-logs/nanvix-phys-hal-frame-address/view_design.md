# View Design: `FrameAddress` (`hal/mem/types/address/frame.rs`)

> Scope: design/confirm the abstract `View` (+ `inv`) for `FrameAddress` that
> the in-scope verification-order targets will reference in later spec/proof
> phases.
>
> In-scope targets: the type `FrameAddress` (its `View` and `inv`),
> `FrameAddress::from_raw_value`, `FrameAddress::into_raw_value`,
> `FrameAddress::from_frame_number`, `FrameAddress::into_frame_number`.
> Out of scope and untouched: `new`, `into_physical_address`,
> `into_page_address`, `Debug::fmt`, `PartialEq::eq`.
>
> A `View`/`inv` skeleton already exists inline in the module's `verus!` block
> (`type V = int; view = self.0@; inv = self@ % spec_page_size() == 0`), plus the
> `spec_page_size()` uninterp fn and the `PAGE_SIZE` `assume_specification`.
> This document **reviews and confirms** that skeleton against the caller
> analysis, applies the substitution test to every candidate field, designs the
> spec transitions the in-scope contracts will use, and records rejected
> alternatives. **Conclusion: the existing View shape is correct as-is; no
> change to its structure is warranted.** This phase only adds the spec
> transition helpers and the rationale.

---

## Abstract Resource

`FrameAddress` is, to its callers, **an opaque, page-aligned physical address
that names one physical frame** — a `Copy` value object (not an owning
resource) whose only observable state is a single mathematical integer: the
raw guest-physical address at which the frame begins.

It is the shared currency between three subsystems:

- the **physical-frame allocator** (`mm::phys` — `frame.rs`, `kframe.rs`),
  which produces/consumes frame addresses and reasons about set membership
  (`frame@ ∈ allocated_frames`);
- the **virtual-memory manager** (`mm::virt` — `vmem.rs`, `boot_init.rs`),
  which does pointer arithmetic on the raw address (`frame@ + offset`) for
  `memcpy`/`memset` and pointer casts; and
- the **hardware MMU / paging layer** (`hal::arch::mmu` — `hwpt.rs`,
  `page_table.rs`, `page_directory.rs`, and CR3 loads in `pm`), which feeds the
  address (or its frame index) into hardware paging structures.

Callers exchange a `FrameAddress` through **two equivalent identities of the
same underlying integer**, and convert freely between them:

- the **raw physical address** — `into_raw_value` / `from_raw_value`; and
- the **frame index** — `into_frame_number` / `from_frame_number`, where the
  index is `address / PAGE_SIZE`.

It is **not** a collection, resource manager, or state machine: it is an
immutable scalar value. That it is internally a newtype over
`PageAligned<PhysicalAddress>` is invisible to callers — the caller analysis is
explicit that callers "do not care that it wraps `PageAligned<PhysicalAddress>`
internally; the newtype representation could change without affecting them."

---

## View Struct

The abstract state of a `FrameAddress` is **exactly one value**: the raw
guest-physical address as an unbounded integer. Verus models this with a
**scalar `View` (`type V = int`)**, not a one-field record struct, because there
is only one caller-observable quantity. The existing inline skeleton is **kept
unchanged**:

```rust
impl View for FrameAddress {
    type V = int;

    // `closed`: callers may reference `self@` (the abstract physical address),
    // but the delegation to the inner PageAligned<PhysicalAddress> is hidden.
    // The abstract value is "the frame's base physical address as an
    // unbounded integer".
    closed spec fn view(&self) -> int {
        self.0@      // newtype identity: delegate to the inner address's view
    }
}
```

`self@ : int` is the entire abstract state. This mirrors the rest of the
address tower — `VirtualAddress@ : int`, `PhysicalAddress@ : int`,
`PageAligned<T>@ : int`, `FrameNumber@ : int` (where present) — confirming the
abstraction is caller-driven, not implementation-driven: every member of the
family models its value as a single `int`.

`view()` is **`closed`** so the two-level newtype delegation
(`FrameAddress → PageAligned<PhysicalAddress> → PhysicalAddress → int`) does not
leak; callers still obtain a usable `int` (`fa@`) for arithmetic, frame
indexing, set membership, and comparison.

### Equivalent "single-field" reading

```rust
pub struct FrameAddressView {
    // addr: the base physical address of the frame this handle denotes, as
    //       `int`. The only state a caller can observe. The "page alignment"
    //       fact and the "frame index" (addr / PAGE_SIZE) are a *property* and
    //       a *derived projection* of this value, not separate fields.
    addr: int,
}
```

We deliberately do **not** materialize this as a record struct; the scalar
`type V = int` is the idiomatic encoding for a single-value abstraction in this
codebase and keeps specs maximally simple.

---

## Well-formedness Invariant

```rust
impl FrameAddress {
    // Page alignment is the one structural guarantee every constructor
    // establishes and every caller relies on without re-checking. Stated over
    // the abstract address `self@`, independent of the inner representation.
    pub open spec fn inv(&self) -> bool {
        self@ % spec_page_size() == 0
    }
}
```

- `inv()` is **`pub open spec fn`** so callers can both assume it (after a
  successful constructor) and see its definition; alignment is an
  abstraction-level property callers must reason about (e.g. when computing
  `fa@ / spec_page_size()` for the frame index, or when establishing that
  `fa@ + offset` stays within the intended frame).
- It is stated purely in terms of `self@` and `spec_page_size()` — no reference
  to the inner `PageAligned`/`PhysicalAddress` fields — so it survives any
  re-representation of the newtype.
- `spec_page_size()` is the existing `uninterp spec fn`, tied to the runtime
  `::arch::mem::PAGE_SIZE` by the existing `assume_specification`
  (`result == spec_page_size()`). Keeping the modulus abstract (rather than a
  literal like `4096`) lets the same `inv()` hold across architectures.

> Refinement note: `inv()` could additionally bound the address
> (`0 <= self@ < spec_max_address()`), but no in-scope caller depends on an
> upper bound through the `int` View — the allocator's bound check is expressed
> against the *frame number* / refcount-array length, not the address. We
> therefore keep `inv()` minimal (alignment only) and let any range facts be
> introduced as constructor postconditions if a later spec genuinely needs
> them. See Rejected Alternatives.

---

## Spec Transition Functions

`FrameAddress` is **immutable** — none of the in-scope functions mutate `self`,
so there are no state-*mutation* transitions in the `..self` sense. What the
contracts need instead are **pure abstract relationships** between an address
and its frame index, and between a raw value and the address it denotes. We
express these as reusable spec helpers on the **View domain** (`int`), so the
constructors/projections can be specified declaratively and the round-trips
fall out as algebraic facts.

```rust
// Address <-> frame-index correspondence, in the abstract int domain.
// Placed as free spec fns (or on a view helper) so every in-scope contract
// references the SAME definition of "the frame index of an address".

// The frame index that an aligned physical address belongs to.
pub open spec fn spec_addr_to_frame_number(addr: int) -> int {
    addr / spec_page_size()
}

// The base physical address of a given frame index.
pub open spec fn spec_frame_number_to_addr(n: int) -> int {
    n * spec_page_size()
}
```

These two are mutual inverses on aligned addresses
(`spec_frame_number_to_addr(spec_addr_to_frame_number(a)) == a` whenever
`a % spec_page_size() == 0`), which is exactly the round-trip callers depend on.

### How the in-scope contracts will use the View (sketch only — not this phase's deliverable)

The actual `#[verus_spec]` text is produced in the spec-design phase; listed
here only to demonstrate the View is sufficient and to fix the vocabulary.

- **`into_raw_value(self) -> usize`** *(already trusted, `external_body`,
  listed in `tcb-allowed.md`)*
  `ensures result as int == self@` — the raw value is the abstract address.
  The View supplies `self@` directly.

- **`from_raw_value(raw_addr: usize) -> Result<Self, Error>`**
  `ensures result matches Ok(fa) ==> fa@ == raw_addr && fa.inv()`
  (success yields newtype identity **and** alignment);
  `result is Err <==> raw_addr % spec_page_size() != 0`
  (failure ⇔ the raw value is not page-aligned — a *dynamic* condition on
  external boot input, so it stays a runtime check + `Err`, not a `requires`).
  Mirrors the sibling `phys.rs` contract
  (`Ok(r) ==> r@ == addr@ && r.inv()`).

- **`from_frame_number(frame_number) -> Result<Self, Error>`**
  `ensures result matches Ok(fa)
       ==> fa@ == spec_frame_number_to_addr(frame_number@) && fa.inv()`;
  the `Err` path corresponds to the frame number being out of representable
  address range (dynamic ⇒ runtime check + `Err`). Here `frame_number@` is the
  frame number's `int` view (its raw index).

- **`into_frame_number(self) -> FrameNumber`** *(total — a `FrameAddress` is
  always aligned by construction)*
  `ensures result@ == spec_addr_to_frame_number(self@)`
  (`== self@ / spec_page_size()`). Combined with the `from_frame_number`
  contract this gives the round-trip
  `from_frame_number(n).into_frame_number()@ == n@` that allocator/page-table
  callers rely on when using the value as a bitmap/refcount index.

> Dependency note: `into_frame_number`/`from_frame_number` mention
> `FrameNumber@`. `FrameNumber` (in `::arch`) has no `View` yet; the
> spec-design phase will reference its raw value via whatever projection is
> available (introducing a thin `View<V = int>`/`spec` for `FrameNumber` if
> needed). This does **not** change the `FrameAddress` View — `FrameAddress@`
> remains the physical address; the frame index is a *derived* `int`.

---

## Design Rationale (substitution test per field)

The View has exactly **one** field, `self@ : int` (the abstract physical
address). Applying the substitution test — *"if the implementation were
completely rewritten with a different algorithm, would this field still make
sense?"*:

| Field | Substitution test | Verdict |
|-------|-------------------|---------|
| `self@ : int` (base physical address) | A `FrameAddress` could store the raw `usize` directly, store a frame index and multiply, pack flags into spare low bits, or wrap a different address type — in **every** such rewrite "the frame's base physical address as an integer" is still the one thing callers read via `into_raw_value`, arithmetic-on, index-by, and compare-with. | ✅ Survives |

Why this single field is also **complete** for every caller-observable concept
in the analysis:

- **Raw physical address** (`into_raw_value`/`from_raw_value`, used for pointer
  arithmetic, pointer casts, CR3 loads) — *is* `self@`.
- **Frame index** (`into_frame_number`/`from_frame_number`, used as
  bitmap/refcount index and PTE/PDE field) — derived as
  `self@ / spec_page_size()`; needs no separate field because it is a pure
  function of the address.
- **Page alignment** (relied on by all callers, never re-checked) — a
  *property* of `self@`, captured by `inv()`, not a field.
- **Set membership** (`mm::phys::frame` specs over `allocated_frames` /
  `free_frames`) — operates on `frame@ : int`, exactly this field.
- **Equality** (`PartialEq`, out of scope) — agrees with integer equality of
  `self@`.

Quality-review checklist (from the view-design skill):

| Criterion | Result |
|-----------|--------|
| **Substitution** | ✅ the single field survives a complete rewrite. |
| **Caller-only** | ✅ "physical address as `int`" is meaningful with zero impl knowledge. |
| **Complete** | ✅ raw value, frame index, alignment, membership, equality all expressible. |
| **Minimal** | ✅ one field, used by every in-scope contract; nothing removable. |
| **No code-as-spec** | ✅ `int` value + `% == 0` alignment describe WHAT, never HOW (no `PageAligned`/`PhysicalAddress` mechanics leak). |

`view()` stays **`closed`** (mapping hidden) and `inv()` stays **`open`**
(alignment visible), per the skill's `view`/`inv` visibility rules. No extra
`pub spec fn`s are added to `impl FrameAddress` beyond `view` and `inv`; the
address↔index helpers live in the spec/View domain as free spec fns.

---

## Rejected Alternatives

1. **A record `struct FrameAddressView { addr: int }`.**
   Rejected: a single-value abstraction is more simply and idiomatically a
   scalar `type V = int` (matches `PhysicalAddress`, `PageAligned`,
   `VirtualAddress`, `FrameNumber`). A record adds a field-access layer to
   every spec for no expressive gain.

2. **Mirroring the implementation:
   `view -> PageAligned<PhysicalAddress>` or a struct field
   `inner: PageAligned<PhysicalAddress>`.**
   Rejected: this is the cardinal anti-pattern — it exposes the exact internal
   representation the caller analysis says callers must not depend on, fails the
   substitution test (a rewrite that drops the `PageAligned` wrapper would break
   the View), and forces callers to understand the inner tower to read `self@`.

3. **Two View fields: `{ addr: int, frame_number: int }`.**
   Rejected: redundant. `frame_number` is `addr / spec_page_size()` — a derived
   quantity. Storing it invites a consistency invariant
   (`frame_number == addr / spec_page_size()`) that adds proof burden with no
   new caller-observable state. Kept as the derived spec helper
   `spec_addr_to_frame_number` instead.

4. **View as the frame index (`type V = int` meaning `addr / PAGE_SIZE`).**
   Rejected: the heaviest-used projection (`into_raw_value`, ≈14 sites) and the
   `mm::phys` membership specs reason about the **address**, not the index; and
   `boot_init.rs` constructs from a raw address. Making the View the index would
   force a `* spec_page_size()` in the most common contracts and lose the direct
   `result as int == self@` identity the existing trusted spec already relies
   on. The address is the more fundamental, lower-friction abstraction; the
   index is recovered by division.

5. **Machine-typed View (`type V = usize`).**
   Rejected: the View lives in spec world; `int` avoids overflow reasoning in
   specs (e.g. `self@ + offset`, `n * spec_page_size()`), per the skill's
   "prefer mathematical types" rule. The exec/`int` bridge is supplied by the
   `into_raw_value` contract (`result as int == self@`).

6. **Putting a range bound (`0 <= self@ < MAX`) into `inv()` now.**
   Rejected (deferred): no in-scope caller derives a fact from an address upper
   bound through the `int` View — bounds checks are expressed against the frame
   number / refcount-array length. Adding it now would be speculative
   over-specification. If a later constructor contract genuinely needs it, it is
   added then (as an `inv()` clause or constructor postcondition), without
   changing the View shape.

7. **Folding alignment into `view()` (e.g. returning a refined type) instead of
   `inv()`.**
   Rejected: alignment is a *predicate over* the value, not part of the value's
   identity. Keeping it in `inv()` (open, callable) is what lets callers assume
   it after `Ok(..)` and discharge `self@ / spec_page_size()` reasoning; a
   "refined view" would complicate equality/arithmetic for no benefit.

---

## Summary

The pre-existing skeleton is confirmed unchanged:

```rust
impl View for FrameAddress {
    type V = int;
    closed spec fn view(&self) -> int { self.0@ }
}
impl FrameAddress {
    pub open spec fn inv(&self) -> bool { self@ % spec_page_size() == 0 }
}
```

with two derived View-domain helpers introduced for the upcoming contracts:

```rust
pub open spec fn spec_addr_to_frame_number(addr: int) -> int { addr / spec_page_size() }
pub open spec fn spec_frame_number_to_addr(n: int) -> int    { n * spec_page_size() }
```

`FrameAddress@` is the frame's base physical address as an unbounded integer —
the single caller-observable quantity — with page alignment as its sole
well-formedness property. This is the abstraction boundary all
`from_raw_value` / `into_raw_value` / `from_frame_number` / `into_frame_number`
specifications will reference.
