# View Design: `hal::mem::types::address::aligned::page` (`PageAligned<T>`)

> Scope: design/refine the abstract `View` for `PageAligned<T>` that the
> in-scope functions (`PageAligned::from_address`, `PageAligned::into_raw_value`,
> and the `PageAligned<T>` type itself) will reference in later spec/proof phases.
>
> A `View`/`inv` skeleton already exists in the module's `verus!` block. This
> document **reviews and confirms** that skeleton against the caller analysis,
> applies the substitution test to each field, and records the rationale and the
> alternatives that were rejected. The conclusion is that the existing design is
> correct as-is; no change to the View shape is warranted.

---

## Abstract Resource

`PageAligned<T>` is, to its callers, **a memory address (a single mathematical
integer) carrying the static guarantee that the address lies on a page
boundary**. It is the kernel's compile-time witness of page alignment, threaded
through physical/virtual memory management, MMU paging, MMIO regions, and
process stacks. The wrapped `T` (`VirtualAddress` / `PhysicalAddress`) is itself
viewed as an `int` address, so a `PageAligned<T>` adds no new observable state —
only a new *property* (alignment) over the same single address value.

---

## View Struct

The abstract state of a `PageAligned<T>` is exactly one value: the underlying
numeric address, as an unbounded integer. Verus models this with a scalar
`View` (`type V = int`) rather than a record struct, because there is only one
caller-observable quantity.

```rust
impl<T: Address> View for PageAligned<T> {
    type V = int;

    // The abstract address (raw numeric address as an unbounded integer).
    // Delegates to the inner address's ghost projection `spec_addr` — newtype
    // identity.
    closed spec fn view(&self) -> int {
        spec_addr(&self.0)
    }
}
```

> **Bound change (this phase).** The skeleton bounded the impl on
> `T: Address + View<V = int>` and defined `view = self.0@`. That bound is
> **unusable for the in-scope exec contracts**: the `@`-based ensures of
> `into_raw_value` (a method of the generic `impl<T: Address> Address for
> PageAligned<T>`) and of `from_address` must be expressible for a *bare*
> `T: Address`, and the `View<V = int>` bound (a) is not present on those exec
> impls, (b) cannot be added to them without breaking `region.rs`
> (`TruncatedMemoryRegion<T>` uses `PageAligned<T>: Address` for bare
> `T: Address`), and (c) is **unsatisfiable in a normal `cargo build`** because
> every address-family `View` impl is `cfg(verus_keep_ghost)`-gated, so a normal
> build that saw the bound would fail (`PhysicalAddress: View not satisfied`).
>
> The fix keeps the same abstract resource but makes the `View` **unconditional**
> over `T: Address`, delegating to a ghost projection
> `pub uninterp spec fn spec_addr<T: Address>(addr: &T) -> int` (declared in
> `page.spec.rs`, `cfg(verus_keep_ghost)`-gated). `spec_addr` is uninterpreted;
> the exec contract `into_raw_value: result as int == self@` pins it
> operationally (newtype identity), exactly as the sibling
> `FrameAddress::into_raw_value` trust boundary does for the concrete frame
> address. Because `view()` is `closed`, the switch from `self.0@` to
> `spec_addr(&self.0)` is invisible to every consumer (including `FrameAddress`
> and `region.rs`, whose own views are `closed`), and broadening the bound from
> `T: Address + View` to `T: Address` only *adds* availability.

Equivalent "single-field" reading:

```rust
pub struct PageAlignedView {
    // addr: the numeric memory address this token denotes, as `int`.
    //       The only state a caller can observe; the page-boundary fact is a
    //       *property* of this value, expressed by inv(), not a separate field.
    addr: int,
}
```

`view()` is `closed`: callers reference `p@` as an `int`, but the mapping
(`spec_addr(&self.0)`, i.e. delegation through the inner `T`'s ghost
projection) does not leak. Because `spec_addr` is defined for every
`T: Address`, the wrapper has a `View` without requiring `T: View<V = int>`,
which is what makes the in-scope `@`-based contracts expressible for a bare
`T: Address` (see the bound-change note above).

---

## Well-formedness Invariant

```rust
impl<T: Address> PageAligned<T> {
    pub open spec fn inv(&self) -> bool {
        self@ % spec_page_size() == 0
    }
}
```

- `spec_page_size()` is the shared uninterpreted spec constant for the page size
  (declared in `hal::mem`, imported here as `crate::hal::mem::spec_page_size`,
  and tied to the exec `PAGE_SIZE` by an `assume_specification` in the sibling
  `frame.rs`). Using the abstract constant — not a literal `4096` — keeps the
  invariant independent of any concrete page size.
- `inv()` is `pub open`: it is the central caller-visible promise of the type
  ("this address is page-aligned"), so callers and downstream specs must be able
  to unfold it. This is the single property the whole type exists to carry, and
  it matches the existing `inv()` already in the module and the identical
  invariant on the sibling `FrameAddress`.

This is the only well-formedness condition. `int` is unbounded, so no
non-negativity or upper-bound clause is needed at the View level; range facts
(`0 ≤ addr ≤ max_addr`) belong to the inner `T`'s own invariant, not to the
alignment wrapper, and adding them here would duplicate `T`'s contract.

---

## Spec Transition Functions

`PageAligned<T>` is an immutable value newtype: every operation is a pure
projection or a pure validated construction. There is no mutable state and thus
no pre/post state machine — the "transitions" are deterministic relations
between input views and output views. They are stated below as the contracts the
later spec phase will attach (no extra `pub spec fn` is added to
`impl PageAligned`, per the view-design rule; these are the ensures the
functions will carry).

### `from_address(addr: T) -> Result<Self, Error>`  *(in scope)*

A *partial, identity-preserving, validating* constructor.

```text
// Success: identity-preserving and establishes the invariant.
result is Ok(p)  ==>  p@ == spec_addr(&addr)  &&  p.inv()   // p@ % spec_page_size() == 0
// Failure: exactly the unaligned case, no normalization, no side effects.
result is Err(_) ==>  spec_addr(&addr) % spec_page_size() != 0
// Bidirectional success condition (liveness):
result is Ok(_)  <==> spec_addr(&addr) % spec_page_size() == 0
```

Notes:
- `spec_addr(&addr)` is the abstract address of the input `T`; for the wrapped
  result `p`, `p@ == spec_addr(&p.0) == spec_addr(&addr)` since `p.0 == addr`.
- `from_address` **validates, it never rounds/normalizes**: on success the
  address is unchanged (`p@ == addr@`). This matches callers that pre-align with
  `align_down` and then treat `?` as infallible while still relying on the
  alignment guarantee.
- Because the failure condition is the negation of the success condition, the
  liveness clause is derivable and the two are stated together as a single
  bidirectional predicate over the *interface-level* fact `addr@ % page == 0`,
  not over the internal `is_aligned` check.

### `into_raw_value(self) -> usize`  *(in scope, `impl Address`)*

A *total, pure, identity projection* of the abstract address.

```text
result as int == self@
```

Notes:
- Pure newtype identity: no masking, shifting, or transformation. This is what
  callers depend on for in-page offset math
  (`a.into_raw_value() - p.into_raw_value()`) and page walking
  (`a.into_raw_value() + k * PAGE_SIZE`).
- Total (no `Result`) and consumes `self` by value (side-effect free).
- The mirror of `from_raw_value`/`from_address`, giving the round-trip
  `from_raw_value(p.into_raw_value()) == Ok(p)` for any aligned `p`.
- A caller already holding `p.inv()` can further derive
  `result as int % spec_page_size() == 0`; this is *implied* by the identity
  ensures plus the type invariant, so it is **not** added as a separate clause
  (avoids a subsumed property).

### `PageAligned<T>` (the type)  *(in scope)*

The type-level contract is precisely `inv()`: any value of type
`PageAligned<T>` produced by a verified constructor satisfies
`self@ % spec_page_size() == 0`, and every operation returning a `PageAligned`
preserves it. Holding the type is the proof token; callers never re-check
alignment.

---

## Design Rationale

There is exactly one View field. Applying the substitution test:

| Field | Meaning | Substitution test ("rewrite impl with a different algorithm — still meaningful?") |
|-------|---------|-----------------------------------------------------------------------------------|
| `addr: int` (`self@`) | The numeric memory address the token denotes. | **Passes.** Whether the type stores a `usize`, a `u64`, a typed `VirtualAddress`, a base+offset pair, or anything else, "the address it represents" is the one quantity every caller reasons about — offset subtraction, page-multiple addition, raw round-trips, and ordering all read this and nothing else. The internal storage choice is invisible. |

Why this View is right for the callers (from the caller analysis):

- **Offset / page-walk arithmetic** needs the address as a plain integer:
  `into_raw_value` must equal `self@`. ✅ scalar `int` view.
- **Raw-value round-trip** (`from_raw_value ∘ into_raw_value`) needs identity,
  not normalization. ✅ `view = self.0@`, no transformation.
- **The alignment guarantee** is the reason the type exists; it is a *property
  of* the address, captured by `inv()`, not an extra observable field. ✅
- **Ordering/equality** (`eq`/`cmp` forward to the inner address) agree with
  `int` comparison on `self@`. ✅ scalar `int` is totally ordered consistently
  with the inner address.
- **P↔V conversions and `Deref`** preserve the same numeric address and the
  alignment property; they need no additional View state. ✅

Why `closed view` + `open inv`:

- `view()` is **closed** so the delegation to the inner `T`'s view does not leak;
  callers still get a usable `int` (`p@`) for arithmetic and comparison.
- `inv()` is **open** because the page-alignment fact is the public promise
  callers and downstream specs must unfold and rely on.

Consistency with siblings: `FrameAddress` (in `frame.rs`) uses the identical
shape — `type V = int`, `view = self.0@`, `inv = self@ % spec_page_size() == 0`
— and its `into_raw_value` already carries `result as int == self@`. Mirroring
it keeps the `hal::mem` address family uniform and lets `PageAligned`'s eventual
`from_address`/`into_raw_value` contracts compose with `FrameAddress`'s, which
delegate straight to them.

TCB note: neither in-scope function can be body-verified in place within this
phase's scope, so each is given a **trust boundary** that honors the contract
above:

- `PageAligned::from_address` (inherent) is `#[verus_verify(external_body)]` +
  `#[verus_spec(...)]` — mirroring the allow-listed
  `FrameAddress::into_raw_value`. Its body checks page alignment via
  `<T as Address>::is_aligned(PAGE_ALIGNMENT)`, where the `Address` method is
  unspecced and `PAGE_ALIGNMENT` is an `arch` `Alignment` enum constant the
  Verus front-end cannot translate; both are out of scope to spec here.
- `<PageAligned<T> as Address>::into_raw_value` is a method of the external
  `sys::mm::Address` trait. A per-method `external_body`/`#[verus_spec]` would
  require marking the whole `impl Address for PageAligned<T>` verified, which
  currently triggers a Verus front-end panic (`vir/src/traits.rs` assertion); it
  is therefore specced with `assume_specification` in `page.spec.rs`, exactly as
  the codebase already does for the sibling
  `<PageAligned<T> as Address>::from_raw_value` (`kframe.spec.rs`).

Both trust boundaries are recorded in `verus-ai-logs/tcb-allowed.md` and are
discharged when the `sys::mm::Address` trait and the `Alignment` encoding are
verified. The View is deliberately thin (single `int`) so the identity/alignment
ensures will be provable from the inner `T`'s delegated specs at that point.

---

## Quality Review

| Criterion | Result |
|-----------|--------|
| **Substitution** | The one field (`addr: int`) survives any reimplementation; storage layout is invisible. ✅ |
| **Caller-only** | A caller understands "the address, guaranteed page-aligned" with no view of the impl. ✅ |
| **Complete** | Every caller-observable concept — raw value, offset math, round-trip, ordering, alignment — is expressed by `self@` (int) + `inv()`. ✅ |
| **Minimal** | A single field; both `view()` and `inv()` are used by the in-scope contracts. No unused state. ✅ |
| **No code-as-spec** | `view` is a value, `inv` is a modular-arithmetic property; neither restates how alignment is checked or how the address is stored. ✅ |

---

## Rejected Alternatives

1. **A multi-field record View**, e.g.
   `struct { addr: int, page_size: int, page_number: int }`.
   Rejected. `page_size` is a global constant (`spec_page_size()`), not
   per-value state; `page_number = addr / spec_page_size()` is derivable from
   `addr` and adds nothing a caller can't compute. Extra fields would be
   redundant (fails Minimal) and would have to be kept in sync (proof burden).

2. **Carrying alignment as a boolean field** (`is_aligned: bool`) instead of via
   `inv()`. Rejected. Alignment is a *property* that must always hold for a
   well-formed value, not a runtime-varying observation. Encoding it as state
   would permit a meaningless "unaligned `PageAligned`" value and split the
   guarantee across `view` and `inv`. `inv()` is the correct home.

3. **A machine-typed view** (`type V = usize`). Rejected. The skill mandates
   mathematical types in spec world; `usize` reintroduces overflow reasoning
   into offset/page-multiple arithmetic that `int` avoids, and would not compose
   with the inner `T: View<V = int>` and the sibling `FrameAddress` view.

4. **Exposing the inner `T` (`type V = T` or `view = self.0`)**. Rejected. That
   leaks the concrete address type and storage choice (fails Caller-only /
   Substitution) and forces callers through `T`'s API to recover the number they
   actually want. Collapsing to `int` is exactly what every caller uses.

5. **Normalizing the view to the page base**
   (`view = self.0@ - (self.0@ % spec_page_size())`). Rejected. It would be
   tautologically equal to `self.0@` under `inv()`, but it bakes the invariant
   into `view` and, more importantly, contradicts the `into_raw_value`/`from_*`
   identity contract callers rely on (the view must equal the actual raw value,
   not a recomputed base). Keep `view` a faithful identity and let `inv()` state
   alignment separately.

6. **An `exists`-based or witnessed view** (e.g. `exists k :: self@ == k *
   spec_page_size()`). Rejected as a *view*; the witness `k` is computable
   (`self@ / spec_page_size()`), so `exists` hides nothing. The deterministic
   modular-arithmetic form `self@ % spec_page_size() == 0` in `inv()` is the
   directly-usable predicate a caller writes into a proof.

---

## Resulting View (unconditional `View` via `spec_addr` — updated this phase)

```rust
// page.spec.rs (cfg(verus_keep_ghost)-gated)
verus! {
    // Ghost projection of any address to its abstract value (`int`).
    pub uninterp spec fn spec_addr<T: Address>(addr: &T) -> int;
}

// page.rs (cfg(verus_keep_ghost)-gated verus! block)
verus! {

use crate::hal::mem::spec_page_size;

impl<T: Address> View for PageAligned<T> {
    type V = int;
    closed spec fn view(&self) -> int {
        spec_addr(&self.0)
    }
}

impl<T: Address> PageAligned<T> {
    pub open spec fn inv(&self) -> bool {
        self@ % spec_page_size() == 0
    }
}

}
```

This is the abstraction boundary all `requires`/`ensures` for `from_address`,
`into_raw_value`, and `PageAligned<T>` reference.
