# View Design: `VirtualAddress` (`src/libs/sys/src/sys/mm/address/virt.rs`)

> Phase output. Designs the abstract `View` boundary for `VirtualAddress`.
> Built **only** from `caller_analysis.md` and the body-removed public API
> (`body_removed_source.rs`); implementation bodies were not read.
>
> In-scope (verification-order) targets: the type `VirtualAddress` (its `View` +
> `inv`), `VirtualAddress::new`, the inherent `VirtualAddress::from_raw_value`,
> and `Address::into_raw_value` for `VirtualAddress`. **All other functions**
> (`align_up`/`align_down`/`is_aligned`/`checked_add`/`checked_sub`, the rest of
> the `Address` trait impl, `Debug`, `Add`/`AddAssign`, the `From<…>`
> conversions) are **out of scope** and untouched.

---

## 1. Abstract Resource

`VirtualAddress` is an **opaque, total integer tag for a byte location in the
virtual address space**. To every caller (memory-layout constants in `config.rs`,
`MutexAddress`/`ConditionAddress` in `pm/sync.rs`, `ThreadCreateArgs`,
`MmioRegionInfo::base`) its only observable state is a **single mathematical
integer** — the raw machine-word address that was used to build it.

It is *not* a collection, resource manager, or state machine. It is an immutable,
`Copy`, totally-ordered scalar that is:

- **total / infallible** — every `usize` (including `0` and `usize::MAX`) is a
  valid `VirtualAddress`; no validation, masking, normalization, or panic;
- **a pure wrapper** — the stored value equals the construction argument, so the
  value round-trips back to the exact same bits;
- **representation-agnostic** — the fact that it is internally the newtype
  `struct VirtualAddress(usize)` is invisible to callers (confirmed by the caller
  analysis: "callers don't care that it is a tuple newtype").

The contract the View must support, distilled from the caller analysis:

1. **Round-trip identity** — for all `x: usize`,
   `new(x).into_raw_value() == x` and `from_raw_value(x).into_raw_value() == x`.
2. **Constructor equivalence** — `new` and `from_raw_value` produce the same
   value for the same input.
3. **Totality** — all three in-scope functions are infallible for every input;
   no precondition, no failure arm.
4. **Purity** — the abstract value depends only on the construction argument;
   two `VirtualAddress`es are equal iff their raw values are equal.

The View must express all four **without naming `usize`, the tuple field `.0`, or
any storage detail** — they live entirely over the abstract integer.

---

## 2. The View

The abstract value of a `VirtualAddress` is a **single integer**: the raw
address. There is exactly one caller-observable quantity, so the View is a scalar
`int`, not a one-field struct. This View **already exists** in the source and is
kept unchanged:

```rust
impl View for VirtualAddress {
    type V = int;

    // `closed`: callers may reference `self@`, but the mapping to the inner
    // `usize` field is hidden. The abstract value is "the address as int".
    closed spec fn view(&self) -> int {
        self.0 as int
    }
}
```

`self@ : int` is the entire abstract state. This mirrors the already-verified
sibling abstractions in the address tower (`PhysicalAddress@ == self.0@`,
`PageAligned<T>@ == self.0@`, `FrameAddress@ == self.0@`), all of which model an
address as one `int` — confirming the abstraction is caller-driven, not
implementation-driven.

### 2.1 Why `int`, not a struct

Per the view-design skill (Step 2, *minimize fields*): every field must
correspond to a caller-observable abstract concept, and there is exactly one —
the address value. There is **no second quantity** (`VirtualAddress` carries no
alignment, no frame index, no length). Wrapping a lone `int` in a struct would
add ceremony to every spec for no gain.

### 2.2 Why `int`, not `nat` or `usize`

- **`int`, not `usize`** — the View lives in spec world; `int` is mathematical
  and never wraps, so specs avoid carrying `<= usize::MAX` guards. The single
  concrete↔abstract cast (`self.0 as int`) is confined to `view()`.
- **`int`, not `nat`** — for consistency with the existing `type V = int` shared
  across `VirtualAddress`, `PhysicalAddress`, `PageAligned`, and `FrameAddress`.
  Non-negativity is structural (the value originates from a `usize`) and need not
  be re-encoded in the View's element type.

---

## 3. Well-formedness Invariant

`VirtualAddress` is a **total** wrapper: *every* `usize` is a legal value, so the
type carries **no semantic invariant**. Unlike its sibling `PhysicalAddress`
(which needs `spec_frame_number(self@) <= MAX` to make `into_frame_number`
total), none of the in-scope functions (`new`, `from_raw_value`, `into_raw_value`)
can fail or panic, so there is no underwriting property to expose.

```rust
impl VirtualAddress {
    // `open`: visible to callers. Trivially true — a VirtualAddress imposes no
    // constraint on its raw value (the type is a total newtype over usize).
    // The structural bound 0 <= self@ <= usize::MAX holds automatically because
    // the value originates from a usize; it is not an enforced invariant and is
    // therefore not restated here.
    pub open spec fn inv(&self) -> bool {
        true
    }
}
```

**Why `true` and not a bound.** Stating `0 <= self@ <= usize::MAX` would be
honest but redundant: it is a structural consequence of the value coming from a
`usize`, holds for *every* constructible value, and is never needed as a
precondition by any in-scope function (all are total). Adding it would violate
minimality (an `inv()` clause no spec consumes). If a later phase finds a caller
that genuinely needs the upper bound surfaced as a fact, it is added then; the
abstract resource itself imposes nothing.

---

## 4. Spec Transition Functions

`VirtualAddress` is an immutable value type; the in-scope functions are
constructors and one projection, so the "transitions" relate the input value to
the resulting abstract value, expressed over the View's `int` domain. The
relationship is the **identity wrap** in both directions, so no named `spec_*`
transition function is warranted — the relation is stated directly in each
`ensures` as a single equality. (A `spec_new(value) -> int { value as int }`
helper would only rename the identity and is rejected under minimality.)

### `VirtualAddress::new(value: usize) -> Self` — total `const` constructor

```text
ensures result@ == value as int
```

- **Total & `const`**: every `usize` yields a `VirtualAddress`; usable in `const`
  initializers (the `config.rs` layout constants, `NULL_USER_FN`).
- **Identity wrap**: the stored abstract value is exactly the argument — no
  masking, normalization, or validation. This single equality is the whole
  contract callers rely on (and underwrites round-trip below).
- **No `inv()` ensures needed**: `inv()` is `true`, so it is established
  vacuously and listed nowhere.

### `VirtualAddress::from_raw_value(raw_addr: usize) -> Self` — total constructor

```text
ensures result@ == raw_addr as int
```

- **Interchangeable with `new`**: same total identity-wrap contract (the analysis
  states it forwards to `new`). The two constructors are observationally equal:
  `new(x)@ == from_raw_value(x)@` for all `x`, which follows directly from the two
  ensures — callers treat them as the same constructor.
- This is the **inherent** `from_raw_value` returning `Self` (the in-scope
  target). The `Address`-trait `from_raw_value(usize) -> Result<Self, Error>` is
  out of scope; in this module it is infallible (always `Ok`) and would carry
  `ensures Ok(r) => r@ == raw_addr as int`, but it is not specified in this phase.

### `Address::into_raw_value(self) -> usize` — total projection (inverse)

```text
ensures result as int == self@
```

- **Total & consuming**: takes `self` by value, never fails or panics; the result
  is suitable for further numeric handling (`u32::try_from`, storing back into a
  `usize` field) exactly as `mmio.rs` and `pm/sync.rs` use it.
- **Exact inverse of construction**: returns the bits that were stored, with no
  offsetting, masking, or loss. Stated as `result as int == self@` so it is
  independent of the storage layout.
- **Round-trip** (caller intent — `pm/sync.rs` relies on it; provable from the
  ensures above, not a separate clause):
  `from_raw_value(x).into_raw_value() == x` and
  `new(x).into_raw_value() == x` for every `x: usize`. From `new`'s ensures,
  `new(x)@ == x as int`; from `into_raw_value`'s ensures,
  `new(x).into_raw_value() as int == new(x)@ == x as int`, hence equal as `usize`.

---

## 5. Design Rationale (substitution test per element)

The View has a single field, `self@ : int`, plus a trivial `inv()`. Applying the
test — *"if the implementation were completely rewritten with a different
algorithm, would this still make sense?"*:

- **`self@ : int` (raw address)** — ✅ survives any rewrite. Whether
  `VirtualAddress` is a tuple newtype over `usize`, a bare `usize`, a struct with
  named field, or carries tag bits, "the integer virtual address" is the one
  value every caller reasons about (layout constants, equality/ordering of
  addresses, unwrapping back to a `usize`). The `closed` view hides *how* the
  integer is stored (the `.0` field never appears in caller specs).
- **`inv() == true`** — ✅ a statement about the *abstract* value, not storage:
  any implementation of a total address newtype maintains no constraint, because
  by construction every machine word is a legal address. A rewrite to a different
  representation would still impose nothing.
- **The three `ensures` (identity wrap / inverse)** — ✅ each is a value relation
  (`result@ == arg as int`, `result as int == self@`), not a description of
  mechanism. A reimplementation that stores the address differently but means the
  same value satisfies them unchanged.

Quality-review checklist (view-design Step 4):

| Criterion | Verdict |
|-----------|---------|
| **Substitution** | ✅ `self@` and `inv()==true` survive a complete rewrite. |
| **Caller-only** | ✅ a caller understands "the address as an integer" with no impl knowledge. |
| **Complete** | ✅ covers every caller-observable concept: address value, round-trip identity, constructor equivalence, totality (all expressible via `self@` + the ensures). |
| **Minimal** | ✅ one field, each in-scope spec references `self@`; `inv()` carries no unused clause. |
| **No code-as-spec** | ✅ specs state value equalities (WHAT), never the wrapping operation (HOW). |

---

## 6. Rejected Alternatives

- **A struct View `VirtualAddressView { addr: int }`** — rejected. A single
  caller-observable quantity does not warrant a struct; the scalar `int` (already
  in the source and shared across the address tower) is simpler and keeps every
  spec free of `.addr` projection. Fails nothing but adds ceremony.

- **Exposing the inner `usize`/`.0` field in the View** — rejected. The caller
  analysis is explicit that the tuple-newtype representation is an implementation
  detail; mirroring it would violate the cardinal rule (don't mirror internal
  fields) and break specs if the representation changed. The `closed` `view()`
  deliberately hides the `as int` cast.

- **A non-trivial invariant (e.g. an alignment or canonical-form bound)** —
  rejected. `VirtualAddress` performs **no** validation or canonicalization;
  `new(usize::MAX)` and `new(0)` are both legal and used. Any alignment/range
  property belongs to *specific* operations (`align_up`, out of scope), not to
  the type. Imposing one would falsely constrain the total newtype.

- **A representability bound `0 <= self@ <= usize::MAX` inside `inv()`** —
  rejected as an enforced invariant: it is structurally automatic (the value
  comes from a `usize`) and consumed by no in-scope spec, so it fails minimality.
  Recorded as a comment in `inv()` instead, to be promoted only if a future
  caller demonstrably needs it.

- **Named `spec_new` / `spec_from_raw_value` transition functions** — rejected.
  The transition is the identity wrap (`value as int`); a named helper would only
  rename `view()`'s own mapping and add an indirection. The ensures state the
  equality directly, which is what a caller writes into a proof.

- **A `nat` View** — rejected for tower consistency (`type V = int` everywhere in
  `address/*`); non-negativity is structural and need not change the element type.

- **Specifying the `Address`-trait `from_raw_value`/the other trait methods in
  this phase** — out of scope. Only the inherent `new`/`from_raw_value` and
  `into_raw_value` are verification-order targets; the trait `from_raw_value`
  (`-> Result`, infallible here) and the alignment/`checked_*`/conversion methods
  are deferred and intentionally left unspecified.
