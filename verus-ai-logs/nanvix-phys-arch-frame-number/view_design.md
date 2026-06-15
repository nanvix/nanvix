# View Design: `FrameNumber` (`arch/src/x86/mem/paging/frame/number.rs`)

> Scope: design the abstract `View` (+ `inv`) for `FrameNumber` that the
> in-scope verification-order targets will reference in later spec/proof phases.
>
> In-scope targets: the type `FrameNumber` (its `View` and `inv`),
> `FrameNumber::from_raw_value`, `FrameNumber::into_raw_value`.
> Out of scope and untouched: nothing else lives in this module — only the two
> `#[test]` functions, which are de-facto callers, not targets.
>
> Unlike the sibling `FrameAddress`, **no `View`/`inv` skeleton exists yet**:
> `number.rs` carries no `#[verus_spec]`, and `number.spec.rs` /
> `number.proof.rs` are empty `verus! { }` blocks. This is therefore a *fresh*
> design rather than a confirmation. It is also a **real, in-module**
> verification target (its bodies will be verified — it is **not**
> `external_body`/trusted). The boundary contracts recorded for `FrameNumber`
> in `verus-ai-logs/tcb-allowed.md` are the *callers'* trust assumptions stated
> in `phys.spec.rs`; this View is the in-module abstraction those assumptions
> must agree with.

---

## Abstract Resource

`FrameNumber` is, to its callers, **the abstract identity of one physical page
frame**: a bounded non-negative integer index in `0 ..= MAX` (where
`MAX = MAX_ADDRESS / FRAME_SIZE − 1`). It is a `Copy` value object — not an
owning resource, collection, or state machine — whose only observable state is
a single mathematical integer: the raw frame index.

It is the validated currency three subsystems exchange:

- the **physical-frame allocator / refcount layer** (`mm::phys::frame.rs`),
  which produces a raw allocator index, validates it through `from_raw_value`,
  and later uses `into_raw_value` as an index into its `refcount` array;
- the **hardware paging layer** (`pde.rs`, `pte.rs`, `page_table.rs`,
  `page_directory.rs`), which decodes a frame field out of a page-table word
  (`>> FRAME_SHIFT` → `from_raw_value`) and re-encodes it
  (`into_raw_value` → `<< FRAME_SHIFT`); and
- the **address tower** (`PhysicalAddress::into_frame_number` /
  `from_number` in `phys.rs`), which converts between a frame index and a
  physical address (`index * FRAME_SIZE`).

Callers reason about a `FrameNumber` through exactly **two equivalent identities
of the same underlying integer**:

- the **raw frame index** — `from_raw_value` (validate `usize → Option<Self>`)
  and `into_raw_value` (project `Self → usize`); and
- the **sentinel** `NULL` (= frame `0`) and the **inclusive bound** `MAX`.

That it is internally a `usize` newtype is invisible to callers: the analysis is
explicit that callers "don't care about the internal representation (a `usize`
newtype) — only the round-trip and the range boundary matter."

---

## View Struct

The abstract state of a `FrameNumber` is **exactly one value**: the raw frame
index as an unbounded integer. Verus models this with a **scalar `View`
(`type V = int`)**, not a one-field record struct, because there is only one
caller-observable quantity. This matches the rest of the address/frame family
(`PhysicalAddress@ : int`, `VirtualAddress@ : int`, `PageAligned<T>@ : int`,
`FrameAddress@ : int`), confirming the abstraction is caller-driven, not
implementation-driven.

```rust
impl View for FrameNumber {
    type V = int;

    // `closed`: callers may reference `self@` (the abstract frame index as an
    // unbounded integer), but the newtype mapping `self.0 as int` is hidden.
    // The abstract value is "the index of the physical frame this handle names".
    closed spec fn view(&self) -> int {
        self.0 as int   // newtype identity: the inner usize, lifted to int
    }
}
```

`self@ : int` is the entire abstract state. `view()` is **`closed`** so the
`usize`-newtype mapping does not leak; callers still obtain a usable `int`
(`f@`) for indexing, the `index * FRAME_SIZE` / `index << FRAME_SHIFT`
arithmetic their no-overflow proofs need, comparison, and equality. The
round-trip back to a concrete `usize` is supplied by the `into_raw_value`
contract (`result as int == self@`), not by exposing the mapping.

### Equivalent "single-field" reading

```rust
pub struct FrameNumberView {
    // index: the index of the physical frame this handle names, as `int`.
    //        The only state a caller can observe. "In range `0 ..= MAX`" is a
    //        *property* of this value (captured by `inv`), not a separate field;
    //        the corresponding physical address (`index * FRAME_SIZE`) and PTE
    //        field (`index << FRAME_SHIFT`) are *derived projections*, not state.
    index: int,
}
```

We deliberately do **not** materialize this record; the scalar `type V = int`
is the idiomatic encoding for a single-value abstraction in this codebase and
keeps specs maximally simple.

---

## Well-formedness Invariant

```rust
impl FrameNumber {
    // The single structural guarantee every constructor establishes and every
    // caller relies on without re-checking: the index is a representable frame
    // number. Stated over the abstract index `self@`, independent of the inner
    // representation.
    pub open spec fn inv(&self) -> bool {
        &&& 0 <= self@
        &&& self@ <= spec_max_frame_number()
    }
}
```

- `inv()` is **`pub open spec fn`** so callers can both *assume* it (after a
  successful `from_raw_value`, or for any `FrameNumber` they hold) and *see* its
  definition. The range bound is the abstraction-level property callers must
  reason about — it is the **load-bearing** fact behind every caller's
  no-overflow obligation: `index << FRAME_SHIFT` must not lose bits
  (`pde.rs`/`pte.rs`), `index * FRAME_SIZE <= usize::MAX` must hold
  (`phys.rs::from_number`), and `index` must be a genuine `refcount` index
  (`mm/phys/frame.rs`). All of these are discharged from `self@ <=
  spec_max_frame_number()`.
- It is stated purely in terms of `self@` and `spec_max_frame_number()` — no
  reference to the inner `usize` — so it survives any re-representation of the
  newtype.
- `0 <= self@` is included for explicitness in the `int` domain (the inner type
  is unsigned, so it always holds, but stating it lets callers use `self@` as a
  `nat`-like index without re-deriving non-negativity).

### `spec_max_frame_number()`

`inv()` references a module-level spec constant for the inclusive upper bound,
mirroring the exec `const FrameNumber::MAX = mem::MAX_ADDRESS / mem::FRAME_SIZE
− 1`:

```rust
// The inclusive maximum frame index, in the int domain. Abstract counterpart
// of `FrameNumber::MAX`. Defined in number.spec.rs; tied to the exec `MAX`
// (and to `spec_frame_size()` / `spec_max_address()`) in the spec phase.
pub open spec fn spec_max_frame_number() -> int { ... }
```

This is the **same name** the upstream callers already use for this bound in
`phys.spec.rs` / `tcb-allowed.md` (`FrameNumber::into_raw_value` "projects index
in `0 ..= spec_max_frame_number()`"; `from_raw_value` "`Some` iff `value <=
spec_max_frame_number()`"). Designing the in-module View against the identical
constant guarantees the module's real contracts and the callers' assumed
contracts coincide — no translation layer is needed. Keeping the bound abstract
(rather than a literal) lets the same `inv()` hold across `MAX_ADDRESS` /
`FRAME_SIZE` choices.

---

## Spec Transition Functions

`FrameNumber` is **immutable** — neither in-scope function mutates `self`, so
there are no `..self` state-mutation transitions. What the contracts need
instead are the two **pure abstract relationships** between a raw `usize` index
and the `FrameNumber` that denotes it. These are expressed directly with the
View (`self@`) and the bound (`spec_max_frame_number()`); no auxiliary
transition struct is warranted for a scalar newtype.

### How the in-scope contracts will use the View (sketch only — not this phase's deliverable)

The actual `#[verus_spec]` text is produced in the spec-design phase; listed
here only to demonstrate the View is sufficient and to fix the vocabulary.

- **`from_raw_value(value: usize) -> Option<Self>`** — the validating
  constructor.
  ```text
  ensures
      // Bidirectional success/failure condition (the *only* failure signal):
      (result is Some) <==> value <= spec_max_frame_number(),
      // Success preserves the index exactly and is well-formed:
      result matches Some(f) ==> f@ == value as int && f.inv(),
  ```
  `value as int` and `spec_max_frame_number()` are exactly the View vocabulary.
  This both (a) matches the callers' assumed `tcb-allowed` contract and (b)
  supports `phys.rs:211`'s total `unwrap()` (when the caller already knows
  `value <= MAX`, `Some` is guaranteed). The failure condition is **dynamic**
  (an arbitrary runtime `usize`), so it stays `None`, not a `requires`.

- **`into_raw_value(self) -> usize`** — the projection, inverse of the
  constructor.
  ```text
  ensures
      result as int == self@,                 // newtype-identity projection
      0 <= result as int <= spec_max_frame_number(),   // (implied by inv())
  ```
  `result as int == self@` is the load-bearing identity; the range follows from
  `self.inv()` and is what makes the callers' `<< FRAME_SHIFT` / `* FRAME_SIZE`
  no-overflow proofs go through.

- **Round-trip identity** (relied on by the unit tests and `phys.rs`):
  `from_raw_value(v) == Some(f) ==> f.into_raw_value() == v` falls out
  algebraically from the two contracts above
  (`f@ == v as int` and `into_raw_value(f) as int == f@`), so it needs **no**
  separate spec function.

- **Constants** (`NULL`, `MAX`): `NULL@ == 0` (always in range, since
  `0 <= spec_max_frame_number()`), and `MAX as int == spec_max_frame_number()`.
  Both are simple facts over the View; `NULL`'s validity follows from `inv()`.

---

## Design Rationale (substitution test per field)

The View has exactly **one** field, `self@ : int` (the abstract frame index).
Applying the substitution test — *"if the implementation were completely
rewritten with a different algorithm, would this field still make sense?"*:

| Field | Substitution test | Verdict |
|-------|-------------------|---------|
| `self@ : int` (frame index) | A `FrameNumber` could store the raw `usize` directly (current), store the physical address and divide by `FRAME_SIZE`, store a shifted PTE field and unshift, or pack into a wider word — in **every** such rewrite "the index of the physical frame, as an integer" is still the one thing callers read via `into_raw_value`, validate via `from_raw_value`, index-by, arithmetic-on, and compare. | ✅ Survives |

Why this single field is also **complete** for every caller-observable concept
in the analysis:

- **Raw frame index** (`from_raw_value`/`into_raw_value`; used as a `refcount`
  index, and to derive `* FRAME_SIZE` / `<< FRAME_SHIFT`) — *is* `self@`.
- **Range membership `0 ..= MAX`** (relied on by every no-overflow / bounds
  proof, never re-checked by callers) — a *property* of `self@`, captured by
  `inv()`, not a field.
- **Round-trip identity** (`from_raw_value(v).unwrap().into_raw_value() == v`) —
  derived from the constructor/projection contracts over `self@`; no field.
- **Sentinel `NULL` / bound `MAX`** — the specific values `0` and
  `spec_max_frame_number()` of `self@`; no field.
- **`Copy`/`Clone` duplication** (each copy projects the same raw value) —
  agrees with equality of `self@`; no field.

Quality-review checklist (from the view-design skill):

| Criterion | Result |
|-----------|--------|
| **Substitution** | ✅ the single field survives a complete rewrite. |
| **Caller-only** | ✅ "frame index as `int`, in `0 ..= MAX`" is meaningful with zero impl knowledge. |
| **Complete** | ✅ raw index, range, round-trip, `NULL`/`MAX`, equality all expressible. |
| **Minimal** | ✅ one field, used by both in-scope contracts; nothing removable. |
| **No code-as-spec** | ✅ an `int` value plus a range predicate describe WHAT, never HOW (no `usize` mechanics, no `>> FRAME_SHIFT` decode steps leak). |

`view()` stays **`closed`** (mapping hidden) and `inv()` stays **`open`** (range
visible), per the skill's `view`/`inv` visibility rules. No extra `pub spec fn`s
are added to `impl FrameNumber` beyond `view` and `inv`; the upper bound lives in
the spec domain as the free spec fn `spec_max_frame_number()`.

---

## Rejected Alternatives

1. **A record `struct FrameNumberView { index: int }`.**
   Rejected: a single-value abstraction is more simply and idiomatically a
   scalar `type V = int` (matches `PhysicalAddress`, `FrameAddress`,
   `PageAligned`, `VirtualAddress`). A record adds a field-access layer to every
   spec for no expressive gain.

2. **Mirroring the implementation: `type V = usize` / a field
   `inner: usize`.**
   Rejected on two grounds. (a) It exposes the exact internal representation the
   caller analysis says callers must not depend on, and fails the substitution
   test (a rewrite storing the address or PTE field instead of a raw index would
   break it). (b) The View lives in spec world; `int` avoids overflow reasoning
   in specs (e.g. `self@ * spec_frame_size()`, `self@ << spec_frame_shift()`),
   per the skill's "prefer mathematical types" rule. The exec/`int` bridge is
   the `into_raw_value` contract (`result as int == self@`).

3. **View as the *physical address* (`type V = int` meaning
   `index * FRAME_SIZE`).**
   Rejected: this is the abstraction of the sibling `FrameAddress`, not of
   `FrameNumber`. The heaviest-used callers (`refcount` indexing in
   `mm/phys/frame.rs`, PTE/PDE encode/decode) reason about the **index**, and
   `from_raw_value` is handed a raw index. Making the View the address would
   force a `/ FRAME_SIZE` in nearly every contract and lose the direct
   `result as int == self@` identity the round-trip depends on. The index is the
   more fundamental, lower-friction abstraction here; the address is recovered by
   multiplication (and is `FrameAddress`'s job).

4. **View as the *PTE-encoded word* (`index << FRAME_SHIFT`).**
   Rejected: that is a paging-layer encoding detail of just two call sites
   (`pde.rs`, `pte.rs`), not the frame's identity. It would entangle the View
   with `FRAME_SHIFT` and lose meaning for the allocator/address callers. The
   shift is a derived projection of `self@`, not state.

5. **Omitting the range bound from `inv()` (empty / `true` invariant).**
   Rejected: the bound `self@ <= spec_max_frame_number()` is the single
   load-bearing well-formedness fact — without it, callers' no-overflow proofs
   for `index << FRAME_SHIFT` and `index * FRAME_SIZE` cannot be discharged, and
   `into_raw_value`'s assumed `tcb-allowed` range contract could not be met. It
   is safe to require because `from_raw_value` (the only public constructor) and
   `NULL` both establish it.

6. **Naming the bound differently from the callers' `spec_max_frame_number()`
   (e.g. a fresh `spec_frame_number_max()`).**
   Rejected: the upstream callers' assumed contracts in `phys.spec.rs` /
   `tcb-allowed.md` are already stated against `spec_max_frame_number()`. Reusing
   that exact name makes the in-module real contract and the assumed contract
   definitionally identical, so verifying this module directly discharges the
   assumption the callers trusted — no bridging lemma needed.

7. **Open `view()` (exposing `self.0 as int`).**
   Rejected: the skill mandates `view()` be `closed` so the newtype mapping does
   not leak; callers get all they need (`f@`, plus `into_raw_value`'s
   `result as int == self@` bridge) without depending on the inner `usize`
   representation.

---

## Summary

A fresh, scalar View is introduced for the previously unspecced `FrameNumber`:

```rust
impl View for FrameNumber {
    type V = int;
    closed spec fn view(&self) -> int { self.0 as int }
}
impl FrameNumber {
    pub open spec fn inv(&self) -> bool {
        &&& 0 <= self@
        &&& self@ <= spec_max_frame_number()
    }
}
// in number.spec.rs, tied to the exec `FrameNumber::MAX`:
pub open spec fn spec_max_frame_number() -> int { ... }
```

`FrameNumber@` is the index of a physical frame as an unbounded integer — the
single caller-observable quantity — with representable-range membership
(`0 ..= spec_max_frame_number()`) as its sole well-formedness property. This is
the abstraction boundary all `from_raw_value` / `into_raw_value` specifications
will reference, and it coincides by construction with the `spec_max_frame_number()`
contract the upstream callers already trust.
