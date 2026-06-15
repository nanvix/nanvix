# View Design: `sys::mm::address::mod` (the `Address` trait)

## Abstract Resource

To callers, an `Address` is **one pointer-sized location in an address space**,
nothing more. The only state a caller reasons about is the *numeric address
itself* — a single mathematical integer in `[0, usize::MAX]`. Everything else
(newtype layout, how validity is checked, how alignment is computed, the
`as_ptr`/`as_mut_ptr` siblings) is implementation detail.

The three in-scope trait methods are the **raw-value boundary** of that resource:

- `into_raw_value` — project the abstract address back to its raw `usize`.
- `from_raw_value` — validate/construct an address from a raw `usize`.
- `is_aligned` — a pure query: "is this address a multiple of `align`?"

None of these mutate state. `Address` is `Copy` + `Eq` + `Ord`: an immutable
value type. There is therefore **no mutating transition** to model — only a
projection, a constructor, and a query.

## View Struct

The abstract state is a single scalar, so the View is **not** a struct. The View
of every `Address` implementor is the associated `View::V = int`:

```rust
// In `impl View for <Impl>` (each implementor supplies this; VirtualAddress
// already does in virt.rs):
//
//   type V = int;
//   closed spec fn view(&self) -> int;   // the numeric address
//
// Caller-visible meaning: `self@ : int` is the pointer-sized numeric address.
```

`view()` is `closed` (the mapping from the concrete newtype field to `int` is
private); `self@` is the public handle callers use. This matches the inherited
design already shipped for `VirtualAddress`.

### Shared spec vocabulary (free spec fns in `mod.spec.rs`)

Because the View is the primitive `int`, reusable helpers live as free spec
functions in the module's spec file rather than as methods on a wrapper:

```rust
// Integer alignment value of an `Alignment` (the power of two it names).
// To be backed by the alignment module's own view; declared here as the
// vocabulary the Address specs depend on.
pub open spec fn align_value(a: Alignment) -> int;   // > 0, a power of two

// Alignment predicate over the abstract address. This is the single fact
// `is_aligned` reports and that callers branch on.
pub open spec fn addr_is_aligned(addr: int, a: Alignment) -> bool {
    addr % align_value(a) == 0
}
```

## Well-formedness Invariant

The trait-level invariant is the **common denominator across every implementor**
— the pointer-sized bound. Refinement implementors (`PageAligned`,
`PageTableAligned`, `PhysicalAddress`) add their domain predicate (aligned /
frame-representable) in *their own* `inv()`; that extra predicate must never
appear here, or it would be false for `VirtualAddress`.

```rust
pub open spec fn inv(&self) -> bool {
    self.internal_inv()                  // placeholder: true until impl is seen
    && 0 <= self@ <= usize::MAX as int   // pointer-sized address-space bound
}
```

- `inv()` is `pub open` — callers unfold the bound directly in address-arithmetic
  proofs (`checked_add`/offset/bounds reasoning).
- `internal_inv()` is `pub closed`, left as `true` during view-design; the
  specification phase fills in any implementation-consistency constraints once
  bodies are visible.

This is exactly the inherited `VirtualAddress::inv` (`0 <= self@ <= usize::MAX`),
promoted to the abstract address level so all implementors share it.

## Spec Transition Functions

There are **no state mutations** — addresses are immutable `Copy` values — so
there are no `spec_<method>` update functions. The View vocabulary the three
in-scope methods need is instead a *projection*, a *constructor value*, and a
*query*, expressed directly in their `ensures`:

```rust
// into_raw_value(self) -> usize   (total projection; never fails)
//   ensures  result as int == self@
//
// from_raw_value(raw_addr: usize) -> Result<Self, Error>  (constructor/validator)
//   ensures
//     result matches Ok(a)  ==> a@ == raw_addr as int && a.inv(),
//     result matches Err(e) ==> e == Error::BadAddress
//   (round-trip corollary, usable by callers:
//        into_raw_value then from_raw_value is the identity on valid addresses,
//        and from_raw_value then into_raw_value yields the input raw value.)
//
// is_aligned(&self, align: Alignment) -> Result<bool, Error>  (pure query)
//   ensures
//     result matches Ok(b) ==> b == addr_is_aligned(self@, align)
//   (concrete implementors never take the Err arm; no abstract state for it.)
```

These three clauses are the vocabulary every downstream crate wants: they let
`kernel`/`arch`/`syscall` drop their local `assume_specification` for
`into_raw_value` (`result as int == addr@`) and gain native specs for the error
and alignment paths they currently guard by hand
(`mprotect`/`munmap`/`heap`/`PageAligned`).

## Design Rationale

**Field: the address `int` (`self@`)** — the sole abstract state.
- *Why needed*: every caller reasons about the numeric address — pointer casts
  (`into_raw_value() as *const c_void`), arithmetic (`load_address + p_vaddr`),
  region bounds, alignment, and round-tripping through `from_raw_value`. All of
  it is expressible as facts about this one integer.
- *Substitution test*: ✅ If the newtype were re-implemented (different field
  layout, packed bits, tagged representation, a different validity algorithm),
  "the numeric address it denotes" is still exactly the right abstract value.
  Nothing about `int` ties to a storage strategy.

**Invariant: `0 <= self@ <= usize::MAX`** — the pointer-sized bound.
- *Why needed*: callers doing address arithmetic rely on non-negativity and the
  upper bound to discharge overflow/`checked_add` obligations; it is the one
  property *every* implementor guarantees.
- *Substitution test*: ✅ Any pointer-sized-address implementation maintains it
  regardless of algorithm. It is the universal floor, not an artifact of one impl.

**`align_value` / `addr_is_aligned` vocabulary** — abstract alignment.
- *Why needed*: `is_aligned`'s only payload is the boolean `self@ % k == 0`;
  callers branch on it and need it consistent with `align_up`/`align_down`.
  Naming it once keeps every guard spec (`page.rs`, `pgtab.rs`, `mprotect`,
  `munmap`, `segment`, `heap`) reading the same way.
- *Substitution test*: ✅ "multiple of the alignment" is algorithm-independent —
  true whether the impl uses shift, mask, or modulo.

**Inherited-vs-changed**: Inherited from upstream `VirtualAddress` verification:
`type V = int`, `view() = self.0 as int` (closed), and `inv() = 0 <= self@ <=
usize::MAX`. **Kept verbatim and promoted** to the abstract `Address` level so it
is the shared contract for all implementors. **Added**: the trait-method ensures
vocabulary (`into_raw_value` identity, `from_raw_value` Ok/Err semantics,
`is_aligned` ↔ `addr_is_aligned`) and the `align_value`/`addr_is_aligned`
helpers — none of which existed (`mod.spec.rs`/`mod.proof.rs` were empty stubs).
**Removed**: nothing; the inherited View had no leaking fields.

## Rejected Alternatives

- **Wrapper struct `AddressView { value: int, .. }`** — over-faithful
  anti-pattern. The abstract state is a single scalar; a struct adds an unwrap
  layer, breaks the already-shipped `type V = int`, and forces every existing
  `self@`/`result as int == self@` site (including the kernel trust boundary) to
  be rewritten. Rejected.

- **`usize` view instead of `int`** — the skill prefers `usize` for addresses,
  but the codebase's deployed contract is `int`: `VirtualAddress: View<V = int>`
  and the kernel's `assume_specification ... ensures result as int == addr@`.
  Switching would invalidate that trust boundary and the existing `virt.spec.rs`
  for no proof benefit (callers already reason over `int` for arithmetic). The
  `inv()` bound `0 <= self@ <= usize::MAX` recovers exactly the
  non-negativity/boundedness `usize` would have given. `int` kept for
  consistency and soundness of the existing boundary.

- **Folding refinement predicates (aligned / frame-representable) into the trait
  `inv()`** — would make `inv()` false for plain `VirtualAddress` and fail the
  substitution test. Those belong in each refined implementor's own `inv()`,
  layered on top of this abstract address. Rejected.

- **A `kind`/type-tag or capacity-style field** — not caller-observable; callers
  never inspect which address flavor they hold at the abstract level, they rely
  on the type system for that. Adds nothing to any spec. Rejected.

- **Modeling `is_aligned`'s `Err` arm in the View** — no concrete implementor
  returns it and it carries no abstract state; specifying it would invent a
  distinction callers cannot observe. Left implicit (`Ok` arm only). Rejected.

## Quality Review

| Criterion | Result |
|-----------|--------|
| Substitution | ✅ `int` address + pointer bound survive any reimplementation. |
| Caller-only | ✅ "numeric address" + "is it aligned" need no impl knowledge. |
| Complete | ✅ Casts, arithmetic, bounds, round-trip, alignment, and error paths all expressible. |
| Minimal | ✅ One scalar of state; every helper/clause is used by an in-scope method's spec. |
| No code-as-spec | ✅ Captures WHAT (the address, the alignment predicate), never HOW (shift/mask/modulo, validity algorithm). |
