# View Design: `VirtualAddress` (`mm/address/virt.rs`)

> Scope: design the abstract `View` (+ `inv`) for `VirtualAddress` that the
> in-scope functions will reference in later spec/proof phases.
>
> In-scope (verification-order) targets: the type `VirtualAddress` (its `View`
> and `inv`), `VirtualAddress::new`, the **inherent** `VirtualAddress::from_raw_value`
> (`usize -> Self`), and `Address::into_raw_value` (`self -> usize`). Every other
> item — `align_up`/`align_down`/`is_aligned`, `checked_add`/`checked_sub`, the
> rest of the `Address` trait impl (the fallible `from_raw_value`, `max_addr`,
> `as_ptr`/`as_mut_ptr`), `Debug::fmt`, `Add`/`AddAssign`, and the `From`
> conversions — is **out of scope** and untouched.
>
> A scalar `View` skeleton (`type V = int; closed spec fn view(&self) -> int`)
> already exists inline in the module's `verus!` block. This document **reviews
> and confirms** that skeleton against the caller analysis, applies the
> substitution test to each candidate field, and designs the `inv()` and the
> spec transitions the in-scope contracts will use.

---

## Abstract Resource

`VirtualAddress` is, to its callers, **an opaque pointer-sized integer naming a
single byte location in a virtual address space**. From the caller's
perspective it *is* its raw `usize` value: it carries no ownership, no extra
state, and no internal bookkeeping — it only adds type-safety (distinguishing
virtual from physical/frame addresses) and a few address-arithmetic
conveniences.

It is the currency exchanged across the memory-management stack: the virtual
memory manager (`vmem.rs`), the kernel-image/MMIO region descriptors
(`kimage.rs`, `mmio/region.rs`), the ELF loader and dynamic linker, syscall
paths (`munmap`, segment setup), and the user allocator (`sysalloc`). Almost
every use reduces to the same loop: **observe** the raw integer
(`into_raw_value`), **compute** with it (offsets, bounds, alignment,
pointer casts, virt→phys translation), then **reconstruct** an address
(`new` / `from_raw_value`). The whole module therefore stands or falls on one
property: the abstract value a caller stores *is* the integer they get back.

It is **not** a collection, resource manager, or state machine: it is an
immutable, `Copy`, totally-ordered scalar whose equality and ordering agree with
integer equality/ordering of the raw address. That it is internally a
`struct VirtualAddress(usize)` newtype is invisible to callers — they observe
only "one `usize`", through `new` / `from_raw_value` / `into_raw_value`.

---

## View Struct

The abstract state of a `VirtualAddress` is **exactly one value**: the raw
numeric address. Verus models this with a scalar `View` (`type V = int`), not a
one-field record, because there is only one caller-observable quantity. The
existing inline skeleton is **kept unchanged**:

```rust
impl View for VirtualAddress {
    type V = int;

    // `closed`: callers may reference `self@`, but the mapping to the inner
    // `usize` field is hidden. The abstract value is "the virtual address as an
    // integer".
    closed spec fn view(&self) -> int {
        self.0 as int      // newtype identity: the wrapped usize, as int
    }
}
```

`self@ : int` is the entire abstract state. This mirrors the rest of the address
tower — `PhysicalAddress@ : int`, `PageAligned<T>@ : int`, `FrameAddress@ : int`,
`FrameNumber@ : int` — confirming the abstraction is caller-driven, not
implementation-driven: every member models a location as a single `int`.

`view()` is **`closed`** so the `VirtualAddress(usize)` newtype representation
does not leak; callers still obtain a usable `int` (`v@`) for arithmetic,
comparison, and pointer derivation.

### Equivalent "single-field" reading

```rust
pub struct VirtualAddressView {
    // addr: the numeric virtual address this handle denotes, as `int`.
    //       The only state a caller can observe. Alignment, page index, and
    //       "is this a valid pointer" are *not* stored — they are derived
    //       properties / responsibilities of specific operations, not of the
    //       type.
    addr: int,
}
```

#### Why `int` and not `usize` for `type V`

The `spec-design` guidance prefers `usize` for addresses, but the **entire
address tower already commits to `int`** (the inline skeleton, plus
`PhysicalAddress`/`PageAligned`/`FrameAddress`/`FrameNumber`). Keeping `int`
here preserves cross-type uniformity (offset subtraction
`dst@ - src@`, virt↔phys reasoning, range comparisons all live in one numeric
domain without `usize`↔`int` casts) and avoids editing the already-committed
skeleton. The usize-ness of the value is recovered where it actually matters —
as the well-formedness bound in `inv()` (below) — rather than baked into the
view type.

---

## Well-formedness Invariant

`VirtualAddress` carries **one** universal property that every constructible
value satisfies and that callers depend on: its numeric value **fits in a
`usize`** (the pointer-sized address space).

```rust
impl VirtualAddress {
    // `open`: callers performing address arithmetic and round-trips need to see
    // that `self@` is a representable usize (so e.g. `into_raw_value` can return
    // it, and reconstructions stay in range). The fact must be unfoldable in
    // their proofs, not hidden.
    pub open spec fn inv(&self) -> bool {
        0 <= self@ <= usize::MAX as int
    }
}
```

This is the *only* invariant the type carries, and it is the weakest one that is
both universally true (it holds structurally because the inner field is a
`usize`) and useful to callers:

- It makes the `into_raw_value` projection — which returns a `usize` — well
  typed against `self@ : int`: the post-condition `into_raw_value(self) as int
  == self@` is only meaningful because `self@` is in `usize` range.
- It is what lets the dominant **observe → compute → reconstruct** pattern stay
  sound: `new(src.into_raw_value() + n)` round-trips because both endpoints lie
  in `[0, usize::MAX]`.

**Why no other invariant.** The caller analysis is explicit that
`VirtualAddress` "carries no extra state, ownership, or invariant":

- **No validity / range gate.** `new` and inherent `from_raw_value` are
  *infallible* and apply no validation, masking, or transformation — every
  `usize` is a valid `VirtualAddress` (`max_addr == usize::MAX`). An
  `is_valid(...)` invariant would be false for legitimately-constructed
  addresses.
- **No alignment.** Alignment is a property of *specific* operations
  (`align_up`/`align_down`/`is_aligned`, all out of scope), applied explicitly
  by callers when needed — never an invariant of the type.
- **No page-index representability.** Unlike `PhysicalAddress` (which must have a
  representable frame number to keep `into_frame_number` total), none of the
  in-scope `VirtualAddress` functions impose a totality obligation that requires
  bounding a derived index. The plain usize-range bound suffices.

`internal_inv()` is not needed at this phase: the View is a single scalar mapped
by `closed view` from the one inner field, so there is no redundant/derived exec
field to keep consistent. If the specification phase reveals such an obligation
once impl bodies are visible, `inv()` would conjoin `self.internal_inv()`; for
now it is effectively `true`.

---

## Spec Transition Functions

`VirtualAddress` is an **immutable value type**: the in-scope functions are pure
constructors / a pure projection, so there is no mutable pre/post state machine.
The "transitions" are deterministic identity relations between input values and
the resulting abstract value, stated over the View's `int` domain. They are
given below as the contracts the spec phase will attach to the exec functions.
Per the view-design rule, **no extra `pub spec fn` is added to
`impl VirtualAddress` beyond `inv` and `view`** — the relations are simple
enough to state inline (identity), so no named helper is warranted.

### `new(value: usize) -> Self`  *(in scope — total, `const`, infallible)*

```text
ensures
    result@ == value as int,   // faithful storage: stores `value` verbatim
    result.inv()               // follows from value: usize  (0 <= value <= usize::MAX)
```

- **Total & infallible**: returns `Self`, never validates. Every `usize` yields a
  `VirtualAddress`. The `const fn` shape is preserved (callers use it in `const`
  context, e.g. `const MMAP_BASE: VirtualAddress = VirtualAddress::new(...)`).
- **Value relation, not mechanism**: the post-state is *exactly* the input value
  as an integer — no masking, no alignment, no offset. This is the half of the
  round-trip law `new(a).into_raw_value() == a` that the pervasive
  reconstruct-after-arithmetic callers (`vmem.rs`, `manager/mod.rs`) depend on.
- `result.inv()` is a **consequence** of `value : usize`, so it need not be
  argued separately by the caller — but it is stated so the result is immediately
  usable where `inv()` is required.

### `from_raw_value(raw_addr: usize) -> Self`  *(inherent; in scope — total, infallible)*

```text
ensures
    result@ == raw_addr as int,   // identical to `new`: stores `raw_addr` verbatim
    result.inv()
```

- **Interchangeable with `new`**: the caller analysis states callers "treat them
  as interchangeable"; the spec makes that explicit — same post-state, same
  totality. Used both directly and as a function pointer
  (`.map(VirtualAddress::from_raw_value)`), so the `usize -> Self` shape and the
  identity post-state are both load-bearing.
- This pins the construction half of the round-trip law
  `from_raw_value(a).into_raw_value() == a` relied on at
  `syscall/safe/mem/segment.rs` and throughout the loaders.

> The **trait** `Address::from_raw_value(usize) -> Result<Self, Error>` is *out
> of scope* (0 direct callers); it merely wraps the inherent constructor in
> `Ok`. When later specified it would read `Ok(r) => r@ == raw_addr as int,
> Err(_) => unreachable`, deriving directly from the inherent contract above.

### `into_raw_value(self) -> usize`  *(in scope — pure, total projection)*

```text
ensures result as int == self@   // inverse of construction; pure observation
```

- **The inverse / faithful read-back**: the single most-used operation (102
  sites). `result as int == self@` is the other half of every round-trip law and
  the foundation of *all* downstream address arithmetic (offsets, bounds,
  alignment, pointer casts, virt→phys translation).
- **Pure, non-consuming**: `VirtualAddress` is `Copy`, so this neither mutates
  nor invalidates the receiver; repeated calls yield the same `usize`. There is
  no pre/post state to frame — the contract is a single functional equality on
  the return value.
- **No `requires` needed beyond what the type guarantees**: the result fits a
  `usize` because the inner field *is* a `usize`; `self.inv()`
  (`0 <= self@ <= usize::MAX`) records that fact and makes `result as int ==
  self@` exact rather than truncating. Listing `self.inv()` as a `requires` is
  optional in the spec phase (it holds for every constructible value); the
  equality is the operative guarantee callers use.

### Round-trip corollaries (caller intent — derivable, not separate clauses)

From the three contracts above, the two laws every arithmetic caller relies on
follow immediately and need **not** be restated:

```text
new(a).into_raw_value()            == a   // for all a: usize
from_raw_value(a).into_raw_value() == a   // for all a: usize
into_raw_value(v) ... new(_)@      == v@  // reconstruct preserves the value
```

Likewise, **`Ord`/`Eq` agreement with the integer** (callers use `<`, `==`,
`min`, range checks) is a consequence of `self@` being the address value and the
derived comparison operators ordering by it — not an in-scope contract to add
here (the comparison operators are out of scope), but the View is deliberately
chosen so that this agreement is *expressible* (`a < b  <==>  a@ < b@`) when a
caller needs it.

---

## Design Rationale

| Field | Why it's needed | Substitution test |
|-------|-----------------|-------------------|
| `addr : int` (the scalar `self@`) | It is the *entire* observable state: every caller constructs from it, reads it back, and computes with it. The round-trip laws — the module's reason to exist — are stated purely in terms of it. | **Passes.** "The numeric virtual address" is meaningful for *any* implementation. Rewrite the newtype as two `u32` halves, an offset-from-base, or a tagged integer, and "the address this value denotes, as an integer" still names the same caller-visible quantity. |

`inv()` (`0 <= self@ <= usize::MAX`) is included because the projection returns a
`usize` and the reconstruct pattern needs values to stay in range; it is the
weakest property that is universally true and used by callers. `view()` is
`closed` to hide the newtype; `inv()` is `open` so callers can unfold the bound
in arithmetic proofs.

---

## Rejected Alternatives

- **A one-field record `struct VirtualAddressView { addr: int }`.** Rejected in
  favour of the scalar `type V = int` already used by the skeleton and the whole
  address tower. A record adds a field-access layer (`self@.addr`) with no extra
  expressive power for a single quantity, and would diverge from
  `PhysicalAddress`/`PageAligned`/etc.

- **`type V = usize` instead of `int`.** Tempting per the address-keeps-usize
  guidance, but rejected for tower-wide consistency and to avoid editing the
  committed skeleton: offset subtraction and virt↔phys reasoning are cleaner in a
  single `int` domain, and the usize bound is preserved precisely where it is
  needed via `inv()`. (If the project later standardises addresses on `usize`,
  this is the one line to revisit — the contracts would lose their `as int`
  casts.)

- **A `valid: bool` / range invariant
  (`is_valid_virtual_address(self@)`).** Rejected: `new` and inherent
  `from_raw_value` are infallible and validate nothing — every `usize` is a valid
  `VirtualAddress` (`max_addr == usize::MAX`). Any validity invariant would be
  *false* for legitimately constructed values and would force phantom error paths
  the implementation does not have.

- **An `alignment` field or alignment invariant.** Rejected: alignment is a
  property produced/checked by specific (out-of-scope) operations, applied
  explicitly by callers, never a standing property of the type. The caller
  analysis: callers "don't care about alignment (callers align explicitly
  afterward when needed)."

- **A derived `page_number` field / representability invariant** (analogous to
  `PhysicalAddress`'s frame-number bound). Rejected: none of the in-scope
  `VirtualAddress` functions impose a totality obligation on a derived page index
  (no `into_page_number`-style projection is in scope), so the plain usize-range
  bound is sufficient and the weaker, simpler invariant is preferred.

- **Modelling `new` and `from_raw_value` with distinct spec transitions.**
  Rejected: callers treat them as interchangeable infallible constructors, and
  both stores are pure identity. A single shared identity relation (`result@ ==
  arg as int`) captures both; inventing two named helpers would be redundant.

- **Adding `pub spec fn` helpers (e.g. `spec_new`, `spec_into_raw`) on
  `impl VirtualAddress`.** Rejected per the view-design rule (only `inv` and
  `view` live on the type). The relations are trivial identities best stated
  inline in the contracts; no reusable spec vocabulary is needed.
