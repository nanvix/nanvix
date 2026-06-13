# View Design: `hal::platform::microvm` (`gva_to_gpa`)

In-scope (verification-order) target: the single free function
`gva_to_gpa(gva: usize) -> usize` in
`src/kernel/src/hal/platform/microvm/mod.rs`. Every other item in the module
(the `Platform` struct, `init`, `parse_bootinfo`, the MMIO/control-register
helpers, `virt_to_phys`, `is_valid_physical_address`, …) is **out of scope** and
left untouched.

## Abstract Resource

To callers, this slice of the module is the MicroVM platform's
**guest-virtual → guest-physical address translation**: a single, pure, total
mathematical map over the `usize` address space. On MicroVM the guest runs
identity-mapped, so that map *is the identity* (`gpa == gva`).

It is emphatically **not** a collection, a resource manager, or a state machine:

- `gva_to_gpa` is a free function, not a method, and touches **no** module state.
  It does not read or write `Platform`, the frame allocator, the page tables, or
  any `static mut`. The `Platform` struct is irrelevant to this function.
- It is `usize -> usize`, total and infallible (no `Result`/`Option`, no panic,
  no trap), and deterministic (output depends only on the argument).

The sole in-tree caller — `book_mmio_regions` in `mm/phys/mod.rs:114` — uses it
to reinterpret a frame-aligned virtual MMIO address as the physical frame address
that backs it, then feeds the result to `PhysicalAddress::from_mmio_address` and
`frame::is_covered` / `frame::book`. The only thing the caller relies on is that
the returned GPA corresponds **frame-for-frame** to the input GVA.

## View Type

There is **no stateful View, and no `impl View` block**, because there is no type
under verification and no caller-observable mutable state. `gva_to_gpa` is a pure
function; the thing to abstract is the *translation map itself*, not an object's
state. This mirrors the `PhysicalAddress` precedent in this codebase, where the
abstraction collapsed to a bare `int` rather than a one-field struct once the
substitution/minimality tests removed every candidate field.

The abstraction is therefore a single **pure spec function** that names the
translation map. It lives in the (currently empty) `mod.spec.rs`:

```rust
// The MicroVM platform's guest-virtual -> guest-physical translation map.
//
// MicroVM runs the guest identity-mapped, so the abstract map is the identity
// over the entire usize address space. This is a platform-level invariant
// (the VMM maps GVA==GPA), NOT an implementation artifact: any reimplementation
// of the translation on MicroVM must still yield the identity, so the equality
// is part of the contract, hence `open`.
//
// Total: defined for every address. Deterministic: a function of `gva` alone.
pub open spec fn spec_gva_to_gpa(gva: int) -> int {
    gva
}
```

### Why a spec function over `int`, not a struct (substitution test)

Per the view-design skill (Step 2, *minimize fields*): every field must
correspond to a caller-observable abstract concept that would **survive a
complete reimplementation**. Running the substitution test over every candidate
field rejects all of them, leaving no struct:

| Candidate field | Substitution test | Verdict |
|---|---|---|
| the input/output address pair | not state — it is the function's argument/result, already named by `spec_gva_to_gpa` | reject (duplicate) |
| an offset / base (`gpa = gva + offset`) | encodes one *strategy* (offset map); a real GVA→GPA walk would not have an offset; on MicroVM the offset is fixed at 0 | reject (implementation detail) |
| a `Map<int, int>` of translations | a translation is a *total function*, not a stored finite table; materializing it as `Map` invents internal bookkeeping no implementation keeps | reject (code-as-spec) |
| `memory_size` / address-space bounds | not consulted by `gva_to_gpa` (it is total over all `usize`); belongs to `is_valid_physical_address` / `max_physical_address`, which are out of scope | reject (wrong function) |
| any `Platform` field (`arch`, `physical_memory_layout`, …) | the function never reads `Platform`; including it would be mirroring an unrelated internal struct | reject (caller-invisible / unrelated) |

Nothing remains to put in a struct. The entire caller-observable abstraction is
"the translation is the function `g ↦ g`", which a single `open spec fn`
expresses directly. Wrapping a lone identity in a struct would add ceremony to
every spec with zero gain.

### `open` vs `closed`

`spec_gva_to_gpa` is `pub open`. The caller (`book_mmio_regions`) must derive
**frame correspondence** — that the GPA it books is the frame backing the MMIO
GVA — and on MicroVM that fact *is* `result == gva`. Hiding the body behind
`closed` would leave the caller unable to prove the booked frame is the right
one. Because the identity is a platform-level guarantee (the VMM's identity
mapping), exposing it does not leak an implementation choice; it states the
contract.

## Well-formedness Invariant

There is **no instance `inv()`**, because there is no instance: a pure, stateless
function carries no well-formedness state to constrain. The standard
`pub open spec fn inv(&self) -> bool` does not apply here.

What plays the role of "well-formedness" is a set of **properties of the map**
that every caller relies on. They are not free-standing facts: each is pinned to
`gva_to_gpa`'s contract in the spec phase (the only way they prove anything), and
each is, for the identity map, trivially discharged:

```rust
// Totality / infallibility: spec_gva_to_gpa is defined for every address, and
// the exec function returns it directly with no error/panic path. Captured by
// the unconditional (no `requires`) ensures below.

// Determinism / purity: spec_gva_to_gpa is a mathematical function of `gva`
// alone — same input yields same output by construction.

// Frame correspondence (the caller's load-bearing property): translating a
// frame-aligned address and stepping by FRAME_SIZE yields the matching
// sequence of physical frames. For the identity map this is immediate, and it
// is *subsumed* by the ensures `result == gva` (advancing the GVA by FRAME_SIZE
// advances the GPA by FRAME_SIZE). Stated as a derivable property, not a
// separate clause:
//   forall|g: int| spec_gva_to_gpa(g + FRAME_SIZE) == spec_gva_to_gpa(g) + FRAME_SIZE
// and injectivity:
//   forall|a: int, b: int| spec_gva_to_gpa(a) == spec_gva_to_gpa(b) ==> a == b
```

These are listed as design intent; in the spec phase they collapse to corollaries
of `result == gva` and need no standalone statement.

## Spec Transition Function

`gva_to_gpa` mutates nothing, so there is no state transition. Its "transition"
relates the input address to the resulting abstract address, expressed over the
View's `int` domain via the spec function above. The contract added to the exec
function is:

```rust
// No `requires`: the function is total over the whole usize address space.
// ensures: the result is exactly the platform translation map applied to gva,
// i.e. the identity on MicroVM. Frame correspondence and infallibility follow.
pub fn gva_to_gpa(gva: usize) -> (result: usize)
    ensures
        result as int == spec_gva_to_gpa(gva as int),   // == gva
{ ... }
```

Equivalently `result == gva`. The indirection through `spec_gva_to_gpa` names the
platform abstraction (WHAT: "apply the GVA→GPA map") rather than restating the
body (HOW: "return the argument"), keeping the spec declarative and giving future
non-identity platforms a single named hook to redefine.

## Design Rationale

- **Abstraction = the translation map, not an object.** The caller reasons about
  one thing: "what physical frame backs this virtual MMIO address?" The map
  answers exactly that, and nothing in the module's state participates.
- **One named spec function, `int`-typed.** Minimal (one symbol), abstract
  (`int`, overflow-free — identity cannot overflow anyway), and caller-usable:
  `result == gva` drops straight into `book_mmio_regions`' proof to show the
  booked frame matches the MMIO frame.
- **No `requires`.** Totality is a caller expectation (the loop calls it with no
  guard); an empty precondition records that faithfully.
- **`open` body.** Frame correspondence is only provable if the identity is
  visible; the identity is a platform contract, so exposing it is correct, not a
  leak.
- **Properties subsumed, not multiplied.** Determinism, injectivity, and
  frame-stepping all follow from `result == gva`; per the minimality principle
  they are documented but not emitted as extra ensures clauses.

## Rejected Alternatives

1. **A `MicrovmView` / `PlatformView` struct with platform state.** Rejected:
   `gva_to_gpa` reads no `Platform` field. Modeling `arch`,
   `physical_memory_layout`, etc. would mirror an unrelated internal struct and
   violate caller-only + minimality.
2. **An offset field (`gpa = gva + base`).** Rejected by the substitution test:
   it bakes in the "linear offset map" strategy. A real page-walk implementation
   has no single offset; on MicroVM the offset is identically 0, so the field is
   dead. The identity is better stated directly.
3. **`Map<int, int>` of translations.** Rejected as code-as-spec: a translation
   is a *total function*, and a finite `Map` both misrepresents it (it is defined
   on all of `usize`) and invents bookkeeping no implementation maintains.
4. **Address-space bounds (`memory_size`, `max_physical_address`) in the View.**
   Rejected: `gva_to_gpa` is total and never consults bounds; range validation is
   the job of `is_valid_physical_address` / `max_physical_address`, which are out
   of scope. Adding bounds here would over-specify and couple unrelated functions.
5. **`closed` spec function hiding the identity.** Rejected: the caller could no
   longer derive frame correspondence, defeating the only reason the function is
   verified. The identity is a contract, not a hidden detail.
6. **Separate ensures clauses for injectivity / determinism / frame-stepping.**
   Rejected as subsumed: all are corollaries of `result == gva`; emitting them
   would be redundant (minimality / "subsumed properties" anti-pattern).
