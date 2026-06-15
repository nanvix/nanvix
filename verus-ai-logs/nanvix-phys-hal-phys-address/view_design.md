# View Design: `PhysicalAddress` (`hal/mem/types/address/phys.rs`)

> Scope: design the abstract `View` (+ `inv`) for `PhysicalAddress` that the
> in-scope functions will reference in later spec/proof phases.
>
> In-scope (verification-order) targets: the type `PhysicalAddress` (its `View`
> and `inv`), `PhysicalAddress::from_number`, `PhysicalAddress::into_frame_number`,
> and `PhysicalAddress::from_mmio_address`. Every other item — the full `Address`
> trait impl (`from_raw_value`, `align_up/down`, `is_aligned`, `max_addr`,
> `into_raw_value`, `as_ptr`/`as_mut_ptr`), `from_virtual_address`,
> `into_virtual_address`, `from_frame_address`, `from_into_frame_address`, and
> `Debug::fmt` — is **out of scope** and untouched.
>
> A scalar `View` skeleton (`type V = int; view(&self) = self.0@`) already exists
> inline in the module's `verus!` block. This document **reviews and confirms**
> that skeleton against the caller analysis, applies the substitution test to
> each candidate field, and designs the `inv()` and the spec transitions the
> in-scope contracts will use.

---

## Abstract Resource

`PhysicalAddress` is, to its callers, **an opaque integer identifier for a single
byte location in guest-physical memory** — a validated handle whose only
observable state is one mathematical integer, the **raw physical address**.

It is the currency exchanged between the frame allocator (`mm/phys/frame.rs`),
the paging / boot-init code (`mm/virt/boot_init.rs`, `mm/phys/mod.rs`), kernel
module descriptors (`kmod.rs`), and the frame-granular wrappers
(`PageAligned<PhysicalAddress>`, `FrameAddress`). From that single integer every
caller derives the one further concept it ever needs: the **frame** the address
lies in (`addr >> FRAME_SHIFT == addr / FRAME_SIZE`), used to index bitmaps and
refcount arrays and to encode PTE/PDE frame fields.

It is **not** a collection, resource manager, or state machine: it is an
immutable, `Copy`, totally-ordered scalar value whose equality and ordering agree
with integer equality/ordering of the raw address. That it is internally a
newtype over `VirtualAddress` is invisible to callers — the caller analysis is
explicit: callers "don't care that the inner representation is a `VirtualAddress`
newtype … they go through `into_raw_value` / `into_virtual_address` / frame
conversions."

---

## View Struct

The abstract state of a `PhysicalAddress` is **exactly one value**: the raw
numeric address as an unbounded integer. Verus models this with a scalar `View`
(`type V = int`), not a one-field record, because there is only one
caller-observable quantity. The existing inline skeleton is **kept unchanged**:

```rust
impl View for PhysicalAddress {
    type V = int;

    // `closed`: callers may reference `self@`, but the mapping to the inner
    // VirtualAddress field is hidden. The abstract value is "the physical
    // address as an unbounded integer".
    closed spec fn view(&self) -> int {
        self.0@      // newtype identity: delegate to the inner address's view
    }
}
```

`self@ : int` is the entire abstract state. This mirrors the rest of the address
tower — `VirtualAddress@ : int`, `PageAligned<T>@ : int`, `FrameAddress@ : int`,
`FrameNumber@ : int` — confirming the abstraction is caller-driven, not
implementation-driven: every member models a value as a single `int`.

`view()` is **`closed`** so the newtype delegation (`self.0@`) does not leak the
`VirtualAddress` representation; callers still obtain a usable `int` (`p@`) for
arithmetic, frame indexing, and comparison.

### Equivalent "single-field" reading

```rust
pub struct PhysicalAddressView {
    // addr: the numeric physical address this handle denotes, as `int`.
    //       The only state a caller can observe. The "frame it lies in" is a
    //       *derived* quantity (addr / FRAME_SIZE), not a stored field;
    //       in-range-ness is a *property* of this value, expressed by inv().
    addr: int,
}
```

### Derived spec helper (over the View domain)

The frame index is the single derived quantity callers reason about, so it gets a
named helper over the abstract `int` rather than a stored field:

```rust
// The frame that this physical address belongs to. Equals the implementation's
// `addr >> FRAME_SHIFT`, stated as exact integer division so the spec is
// independent of shift-vs-divide.
pub open spec fn spec_frame_number(addr_view: int) -> int {
    addr_view / spec_page_size()
}
```

`spec_page_size() : int` is the pre-existing uninterpreted module constant tied
to `FRAME_SIZE`/`PAGE_SIZE` (the caller analysis establishes
`FRAME_SIZE == spec_page_size()`). `spec_max_frame_number() : int` denotes
`FrameNumber::MAX` (`= MAX_ADDRESS / FRAME_SIZE - 1`); together with
`spec_frame_raw_value(frame: FrameNumber) -> int` (the frame's integer index)
these are the trust-boundary helpers that pin the not-yet-verified `arch`
`FrameNumber` type at the `phys.spec.rs` edge, and they are the vocabulary the
`from_number` / `into_frame_number` contracts use.

---

## Well-formedness Invariant

`PhysicalAddress` carries **one** universal property that every constructible
value satisfies and that callers depend on: its address has a **representable
frame number**. This is exactly the fact that makes `into_frame_number` total —
its internal `FrameNumber::from_raw_value(addr >> FRAME_SHIFT).unwrap()` never
panics — which the frame allocator relies on to use the result directly as a
bitmap / refcount index without a bounds re-check.

```rust
impl PhysicalAddress {
    // `open`: the frame allocator and page-table code rely on the totality of
    // into_frame_number, so the underwriting fact must be visible in their
    // proofs (not hidden behind a closed predicate).
    pub open spec fn inv(&self) -> bool {
        // 0 <= self@ holds structurally (the view comes from a usize); the
        // operative bound is that the frame index fits a FrameNumber.
        spec_frame_number(self@) <= spec_max_frame_number()
    }
}
```

The exact integer form is finalized in the spec phase; the equivalent address-space
phrasing is `0 <= self@ < (spec_max_frame_number() + 1) * spec_page_size()`
(`== self@ < MAX_ADDRESS`). Both express the same caller-visible guarantee: the
address lies within the representable physical address space, so its frame index
is a valid `FrameNumber`.

**Why frame-representability and *not* alignment or RAM-validity.**

- Unlike `PageAligned`/`FrameAddress`, `PhysicalAddress` does **not** carry an
  alignment invariant: `from_mmio_address` deliberately wraps arbitrary,
  possibly-unaligned MMIO addresses (e.g. the LAPIC base `0xFEE0_0000`). Alignment
  is a property of *specific* constructors (`from_number`), not of the type.
- It does **not** carry a tracked-RAM-validity invariant
  (`is_valid_physical_address(self@)`) either: `from_mmio_address` exists
  precisely to bypass the RAM-range validator, since "MMIO GPAs may legitimately
  lie outside tracked RAM." A RAM-validity invariant would be *false* for valid
  MMIO `PhysicalAddress` values.
- Frame-representability is the **weakest property that is both universally true
  (across every constructor, including the unsafe MMIO path) and strong enough**
  for the one totality obligation the in-scope functions impose. Real MMIO GPAs
  sit far below the top of the address space, so the bound holds for them in
  practice; the `unsafe` contract of `from_mmio_address` underwrites it for the
  result.

`inv()` is `pub open` because callers must unfold it to discharge
`into_frame_number`'s `requires`. `internal_inv()` is not needed at this phase:
there is no redundant/derived exec field to keep consistent (the View has one
scalar, mapped by `closed view` to the single inner field); the specification
phase may introduce one if impl bodies reveal a consistency obligation, in which
case `inv()` would conjoin `self.internal_inv()`.

---

## Spec Transition Functions

`PhysicalAddress` is an **immutable value type**: the in-scope functions are pure
constructors / projections, so there is no mutable pre/post state machine. The
"transitions" are deterministic relations between input views and the resulting
abstract value, stated over the View's `int` domain. They are given below as the
contracts the spec phase will attach to the exec functions (no extra `pub spec
fn` is added to `impl PhysicalAddress` beyond `inv`, per the view-design rule —
the reusable helpers live on the View domain).

```rust
// Base address of a frame: from_number multiplies the frame index by the frame
// size. Stated as multiplication, independent of the body's shift-vs-mul.
pub open spec fn spec_from_number(frame_view: int) -> int {
    frame_view * spec_page_size()
}
```

### `from_number(frame: FrameNumber) -> Self`  *(in scope — total constructor)*

```text
ensures
    result@ == spec_from_number(spec_frame_raw_value(frame))
    //       == spec_frame_raw_value(frame) * spec_page_size()
    // hence result@ % spec_page_size() == 0   (frame-aligned base address)
    // and   result.inv()                      (frame index == frame <= MAX)
```

- **Total** (returns `Self`, not `Result`): every `FrameNumber` yields a
  `PhysicalAddress`, as the sole caller (`FrameAddress::from_frame_number`)
  assumes.
- **Value relation, not mechanism**: the result is the frame's base address. The
  caller immediately feeds it to `PageAligned::from_address(..)?`, which calls
  `is_aligned(PAGE_ALIGNMENT)`; this `?` is load-bearing, so the alignment fact
  `result@ % spec_page_size() == 0` must *follow* from the ensures. It does,
  because `result@` is `frame_index * spec_page_size()`. Both alignment and
  `inv()` are consequences of this single ensures (since
  `spec_frame_raw_value(frame) <= spec_max_frame_number()` for any well-formed
  `FrameNumber`), so they are **not** listed as separate clauses (avoids subsumed
  properties).
- **Inverse of `into_frame_number`**: `from_number(f).into_frame_number()` yields
  a frame whose index equals `spec_frame_raw_value(f)` — provable from this
  ensures plus `into_frame_number`'s.

### `into_frame_number(self) -> FrameNumber`  *(in scope — total projection)*

```text
requires self.inv()                 // underwrites totality of the internal unwrap
ensures  spec_frame_raw_value(result) == spec_frame_number(self@)
         // == self@ / spec_page_size()   (equivalently self@ >> FRAME_SHIFT)
```

- **Total under `inv()`**: the receiver's invariant
  (`spec_frame_number(self@) <= spec_max_frame_number()`) guarantees the computed
  index fits a `FrameNumber`, so the body's `FrameNumber::from_raw_value(..)
  .unwrap()` never panics — exactly the no-panic guarantee the allocator depends
  on before using the result as an array/bitmap index.
- **Identifies the containing frame**: the value must equal `self@ / FRAME_SIZE`
  *exactly* (offset-truncating), because callers use it directly as a bitmap
  index, refcount index, and PTE frame field. Same-frame / distinct-frame
  behaviour callers rely on is a *consequence* of this exact integer-division
  equality, not a separate clause.
- **Round trips** (caller intent, derivable from the two ensures):
  `from_number(n).into_frame_number()` has index `spec_frame_raw_value(n)`, and
  for a frame-aligned `p`, `from_number(p.into_frame_number())@ == p@`.

### `from_mmio_address(addr: VirtualAddress) -> Result<Self, Error>`  *(in scope, `unsafe`)*

```text
ensures match result {
    Ok(r)  => r@ == addr@,   // identity wrapping; no RAM-range validation
    Err(_) => true,          // payload unconstrained; arm currently unreachable
}
```

- **Identity, unchecked**: `Ok(r) => r@ == addr@`. This is the whole reason the
  function exists separately from `from_virtual_address` / `from_raw_value`: it
  **bypasses `is_valid_physical_address`**, so it succeeds for addresses the
  range-checked constructors reject. Returning an address whose raw value differs
  from the input, or re-introducing the RAM-range check, would break the
  GVA→GPA→frame mapping and abort boot — so the ensures pins identity and nothing
  more.
- **`unsafe` contract**: the caller promises the address denotes a valid MMIO GPA.
  That obligation underwrites `r.inv()` (frame-representability) so the result may
  later flow into `into_frame_number` via an `is_covered` gate; whether `inv()` is
  attached to the `Ok` arm as an ensures or discharged from the `unsafe`/`requires`
  precondition is settled in the spec phase. Using the `match` form keeps the
  success/failure spec complete by construction.
- **Error path**: the body is effectively `Ok(Self(addr))`, so `Err` never
  occurs; callers `?`-propagate to stay forward-compatible with the fallible
  signature and treat `Err` as a benign skip. The spec leaves the
  `Error`/`ErrorCode` payload unconstrained (callers never inspect it) rather than
  inventing a failure condition the implementation does not exhibit.

### `PhysicalAddress` (the type)  *(in scope)*

The type-level contract is precisely `inv()`: any `PhysicalAddress` value
produced by a verified constructor satisfies
`spec_frame_number(self@) <= spec_max_frame_number()`, and every in-scope
operation returning a `PhysicalAddress` preserves it. Holding the type is the
proof token that `into_frame_number` cannot panic; callers never re-check the
bound.

---

## Design Rationale (substitution test per item)

> Test: *"If the implementation were completely rewritten with a different
> algorithm, would this still make sense?"*

| Item | Meaning | Substitution test |
|------|---------|-------------------|
| `self@ : int` (raw address) | The numeric physical address the handle denotes. | **Passes.** Whether `PhysicalAddress` is a newtype over `VirtualAddress`, a bare `usize`, or a struct with tag bits, "the integer physical address" is the one quantity every caller reasons about — frame indexing, ordering, equality, MMIO identity, raw round-trips. The `closed` view hides *how* the int is stored. |
| `spec_frame_number(self@)` (derived helper) | The frame the address lies in. | **Passes.** Derived purely from `self@` and the module constant; any implementation that means "frame number" by `addr >> FRAME_SHIFT` computes `addr / FRAME_SIZE`, regardless of shift-vs-divide. Not stored, so no rewrite can desync it. |
| `inv()` = frame-representable | The frame index fits a `FrameNumber`. | **Passes.** A property of the *abstract value*, not of any storage layout. Any implementation must maintain it for `into_frame_number` to be total; it is the universal precondition the allocator relies on. |

Why these and not more:

- **Caller-only**: each item is understandable from the function signatures and
  the caller analysis alone — none requires reading a function body.
- **Complete**: the View + helper + `inv()` cover every caller-observable concept
  in the analysis — the address value (`self@`), the frame number
  (`spec_frame_number`), totality of `into_frame_number` (`inv()`), the
  `from_number` alignment-of-base fact (a *consequence* of its ensures), and MMIO
  identity (`r@ == addr@`).
- **Minimal**: each item is used by at least one in-scope contract; the View is a
  single scalar with no redundant or derivable stored state.

Why `closed view` + `open inv`:

- `view()` is **closed** so the newtype delegation to `VirtualAddress` does not
  leak; callers still get a usable `int` for arithmetic and comparison.
- `inv()` is **open** because the frame-representability fact is the public
  promise callers must unfold to discharge `into_frame_number`'s `requires`.

Consistency with siblings: the entire `hal::mem` address family —
`VirtualAddress`, `PageAligned<T>`, `FrameAddress`, and `arch`'s `FrameNumber` —
uses `type V = int` with a `closed` newtype-identity `view`. Mirroring it keeps
the tower uniform so `PhysicalAddress`'s contracts compose directly with
`FrameAddress::from_frame_number` / `into_frame_number` (which delegate straight
through `from_number` / `into_frame_number`) and with `PageAligned::from_address`.

---

## Rejected Alternatives

1. **A struct View `{ addr: int, frame: int }`.** Rejected. `frame` is a pure
   function of `addr` (`addr / spec_page_size()`); storing it duplicates state,
   forces a redundant `inv()` consistency clause, and fails minimality. Kept as
   the derived helper `spec_frame_number(self@)` instead.

2. **Exposing the inner `VirtualAddress`** (`type V = VirtualAddress`, or a field
   for it). Rejected. The caller analysis is explicit that the `VirtualAddress`
   wrapper is an implementation detail; only the integer matters. Mirroring it
   violates the cardinal rule (don't mirror internal fields), leaks the storage
   choice (fails Substitution / Caller-only), and forces callers through `T`'s API
   to recover the number they actually want.

3. **An alignment invariant `self@ % spec_page_size() == 0`** (as on
   `PageAligned`/`FrameAddress`). Rejected as a *type* invariant.
   `from_mmio_address` legitimately produces unaligned addresses, so alignment
   cannot hold for all `PhysicalAddress` values. It is captured where it actually
   holds — as a *consequence* of `from_number`'s ensures.

4. **A RAM-range invariant `is_valid_physical_address(self@)`.** Rejected.
   `from_mmio_address` deliberately bypasses RAM validation for MMIO regions
   outside tracked RAM, so this is false for valid MMIO `PhysicalAddress` values.
   The weaker frame-representability bound is the property that is both
   universally true and sufficient for the totality obligation.

5. **No invariant at all (`inv() == true`).** Rejected. `into_frame_number`
   `.unwrap()`s `FrameNumber::from_raw_value`; without an upper bound on the frame
   index the totality / no-panic guarantee the frame allocator depends on cannot
   be proven. Frame-representability is the minimal bound that discharges it.

6. **Modeling `into_frame_number`'s result as an `int` field in the View.** N/A
   as a View field — there is no second piece of state. The relation is expressed
   by tying the returned `FrameNumber`'s index (`spec_frame_raw_value(result)`) to
   `spec_frame_number(self@)` in the ensures, keeping the View a single scalar.

7. **A `nat` (or `usize`) View instead of `int`.** Rejected. `nat`/`usize`
   desync this type from the rest of the address tower (`type V = int`
   everywhere), forcing `nat`/`int` coercions in cross-type lemmas, and `usize`
   reintroduces overflow reasoning into specs. Non-negativity (`0 <= self@`) holds
   structurally (the view comes from a `usize`) and is recorded in `inv()` where
   needed; `int` is the consistent, cast-free choice.

---

## Quality Review

| Criterion | Result |
|-----------|--------|
| **Substitution** | The single field (`addr : int`) and the derived helper / `inv()` survive any reimplementation; storage layout (newtype-over-`VirtualAddress`) is invisible. ✅ |
| **Caller-only** | A caller understands "the physical address, with a representable frame index" from signatures + caller analysis, no impl reading. ✅ |
| **Complete** | Address value, frame number, totality of `into_frame_number`, `from_number` base-alignment, and MMIO identity are all expressible from `self@` + `inv()` + the transitions. ✅ |
| **Minimal** | One scalar field; `view()`, `inv()`, and each helper are used by an in-scope contract. No unused or derivable stored state. ✅ |
| **No code-as-spec** | `view` is a value; `inv` is an integer-bound property; transitions are arithmetic relations (`* / `) — none restates shift/mul/unwrap mechanics. ✅ |

---

## Resulting View (the abstraction boundary for later phases)

```rust
// phys.spec.rs (cfg(verus_keep_ghost)-gated verus! block)
verus! {

// Pre-existing uninterpreted module constants / FrameNumber trust-boundary
// helpers (declared in phys.spec.rs; pinned to arch when arch is verified):
//   spec_page_size() : int                                  // == FRAME_SIZE
//   spec_max_frame_number() : int                           // == FrameNumber::MAX
//   spec_frame_raw_value(frame: FrameNumber) -> int         // frame's index

impl View for PhysicalAddress {
    type V = int;
    closed spec fn view(&self) -> int { self.0@ }
}

impl PhysicalAddress {
    pub open spec fn inv(&self) -> bool {
        spec_frame_number(self@) <= spec_max_frame_number()
    }
}

// Derived over the View domain (frame the address lies in):
pub open spec fn spec_frame_number(addr_view: int) -> int {
    addr_view / spec_page_size()
}

// Base address of a frame (used by from_number's ensures):
pub open spec fn spec_from_number(frame_view: int) -> int {
    frame_view * spec_page_size()
}

}
```

This is the abstraction boundary all `requires`/`ensures` for `from_number`,
`into_frame_number`, `from_mmio_address`, and the `PhysicalAddress` type itself
reference in the specification and proof phases.
