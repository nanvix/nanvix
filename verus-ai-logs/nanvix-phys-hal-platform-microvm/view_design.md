# View Design: `hal::platform::microvm` (target: `gva_to_gpa`)

## Abstract Resource

To callers, the in-scope facet of this module is the MicroVM platform's
**guest-virtual → guest-physical address translation**: a *pure, total,
deterministic* function from a `usize` address to a `usize` address. On the
MicroVM platform the guest runs in a flat, identity-mapped address space, so
this translation is the **identity map** (`GPA == GVA`).

Crucially, this facet carries **no mutable state**. `gva_to_gpa` is a free
function (`#[inline(always)] pub fn gva_to_gpa(gva: usize) -> usize`), not a
method on `Platform`, and it reads/writes no globals. There is therefore no
caller-observable *state* to model — only a caller-observable *mathematical
map*. The honest View for this scope is a **stateless (unit) View** whose
content is a single pure translation function plus the algebraic properties the
caller relies on.

> This is a deliberate, documented outcome of the methodology — not an
> oversight. Inventing fields here (e.g. memory bounds, a translation table,
> `Platform`'s internals) would fail the substitution and minimality tests
> below. See **Rejected Alternatives**.

---

## View Struct

```rust
/// Caller-visible abstraction of the MicroVM platform's address-translation
/// facet. The translation `gva_to_gpa` is a pure, stateless function, so this
/// View carries no fields: there is no mutable abstract state a caller observes
/// across calls. The substance of the abstraction is the spec function
/// `spec_gva_to_gpa` and its properties, defined below on this type.
pub struct MicrovmTranslationView {}
```

The reusable abstraction lives **on the View type** (per view-design: "no extra
`pub spec fn` on `impl MyType` beyond `inv`/`view`; put helpers on the View"):

```rust
impl MicrovmTranslationView {
    /// Abstract guest-virtual → guest-physical translation as a mathematical
    /// map over addresses. On the MicroVM platform the guest address space is
    /// flat and identity-mapped, so this is the identity. Addresses are modeled
    /// as `nat` (raw, non-negative machine addresses live in spec world).
    pub open spec fn spec_gva_to_gpa(self, gva: nat) -> nat {
        gva
    }
}
```

`spec_gva_to_gpa` is `open` because **identity is the caller-relevant contract**
on MicroVM (the caller depends on the result equaling the input to walk distinct
frames). Hiding it would force callers to re-derive injectivity/monotonicity
they are already told to assume.

---

## Well-formedness Invariant

```rust
pub open spec fn inv(&self) -> bool {
    // The translation facet is stateless: there is no internal bookkeeping to
    // constrain. Well-formedness of the abstraction (totality, determinism,
    // injectivity) is structural — it is a property of `spec_gva_to_gpa`, a
    // total Verus spec function, not an invariant over mutable fields.
    true
}
```

The properties a caller cares about are **not** state invariants; they are
intrinsic to the spec function and exposed as caller-facing facts (next
section):

- **Totality** — `spec_gva_to_gpa` is a total Verus spec fn; defined for every
  `gva: nat`. (Mirrors the exec function never panicking / always returning.)
- **Determinism / purity** — a spec function is by construction a function of
  its argument only. (Mirrors "same input ⇒ same output, no side effects".)
- **Injectivity** — distinct page-aligned inputs map to distinct outputs, so
  the MMIO frame walk visits distinct frames (caller requirement). Provable
  from identity (see lemma).

---

## Spec "Transition" Functions

`gva_to_gpa` is a **pure query**, not a state mutation, so there is no
`spec_<method>(self, ..) -> View` transition (nothing changes). The View instead
exposes the query abstraction and the algebraic facts later `ensures` clauses
will reference.

### Query abstraction (what `gva_to_gpa` returns)

The exec contract for the in-scope function will read:

```rust
// on `pub fn gva_to_gpa(gva: usize) -> usize`
ensures
    result as nat == MicrovmTranslationView::default().spec_gva_to_gpa(gva as nat),
    result == gva,   // identity, the MicroVM platform contract the caller relies on
```

(`result == gva` is the directly-usable form for the caller; the
`spec_gva_to_gpa` form ties it to the View vocabulary for downstream specs.)

### Caller-relevant property (injectivity / frame-distinctness)

A View-level helper + lemma the caller's MMIO loop can cite, stated abstractly
(not by reading the body):

```rust
impl MicrovmTranslationView {
    /// Injectivity of the translation over the address space: distinct guest
    /// virtual addresses yield distinct guest physical addresses. This is what
    /// lets `book_mmio_regions` advance `start` by `FRAME_SIZE` and be sure it
    /// walks distinct physical frames (no aliasing / double-booking).
    pub open spec fn injective(self) -> bool {
        forall|a: nat, b: nat|
            self.spec_gva_to_gpa(a) == self.spec_gva_to_gpa(b) ==> a == b
    }
}

// proof obligation discharged in mod.proof.rs:
// pub proof fn lemma_translation_injective(v: MicrovmTranslationView)
//     ensures v.injective()
```

---

## Design Rationale (per field / per element)

This View has **zero fields by design**. The rationale below covers each
element actually present and why each passes the quality checks.

| Element | Why present | Substitution test |
|---|---|---|
| `MicrovmTranslationView` (unit struct) | Provides a named home for the translation abstraction so all later `requires`/`ensures` share one vocabulary, while honestly modeling that the facet has no state. | ✅ Survives any rewrite — a pure function has no state under *any* algorithm. |
| `spec_gva_to_gpa(gva) -> gpa` | The single caller-observable concept: the GVA→GPA map. The caller feeds the result into `from_mmio_address` and walks it per frame. | ✅ Any correct MicroVM implementation (offset arithmetic, identity page-table walk, direct return) yields the same map, because GVA==GPA is a *platform* fact, not an algorithm choice. The spec describes WHAT (the map), not HOW. |
| `injective()` + lemma | The caller's tight `while` loop relies on distinct page-aligned inputs not aliasing. Stated as a `forall` over the abstract map — simpler than the code. | ✅ Holds for any address-preserving (or otherwise injective) realization the platform might adopt. |
| `inv() == true` | Stateless facet ⇒ no field constraints. Kept for interface uniformity with other module Views. | ✅ Trivially stable across rewrites. |

**Identity vs. over-abstraction.** The caller analysis notes a non-identity map
(offset / real page-table walk) *would not break the caller* provided
injectivity + valid encoding hold. That is a statement about caller
*robustness*, not about what is *correct on MicroVM*. We are verifying the
MicroVM platform, where the guest is identity-mapped; therefore `result == gva`
is the true, platform-defined contract and the strongest fact the caller can
use directly. Encoding identity (an `open` spec) is correct and still passes the
substitution test, because the test concerns *algorithm* rewrites, and every
correct MicroVM algorithm computes the identity. Injectivity is additionally
exposed as the weaker, platform-independent property downstream proofs cite, so
the abstraction degrades gracefully if the platform contract is ever relaxed.

---

## Quality Review

| Criterion | Result |
|---|---|
| **Substitution** | ✅ No field encodes an implementation strategy; the map and its properties survive any MicroVM-correct rewrite. |
| **Caller-only** | ✅ Every element (`spec_gva_to_gpa`, `injective`) is something `book_mmio_regions` reasons about; none mirrors `Platform`'s internals. |
| **Complete** | ✅ Captures all caller-observable concepts from the analysis: totality, purity/determinism, identity, injectivity, valid encoding (the last is enforced at the caller boundary via `from_mmio_address`, see note). |
| **Minimal** | ✅ Every element is referenced by the `gva_to_gpa` contract or its supporting lemma. Zero unused fields. |
| **No code-as-spec** | ✅ The View states WHAT the translation is (`gva`, injective), not how any algorithm computes it. |

**Note on "valid address encoding."** The caller requires the result be
acceptable to `VirtualAddress::from_raw_value` + `PhysicalAddress::from_mmio_address`.
Because the translation is identity over `usize`, the output occupies the same
representable range as the input and trivially satisfies any
`usize`-representability precondition; no extra View field is needed. The
*coverage* question ("is the GPA backed by tracked RAM?") is explicitly the
caller's responsibility (`frame::is_covered`) and is therefore **not** modeled
here — modeling it would over-specify `gva_to_gpa`.

---

## Rejected Alternatives

1. **Mirror `Platform { arch, _pit, physical_memory_layout }`.**
   Rejected — these are internal/implementation fields of a *different*,
   out-of-scope abstraction (full platform init). `gva_to_gpa` neither reads nor
   depends on them. Fails Substitution and Caller-only.

2. **A translation table `Map<nat, nat>`.**
   Rejected — implies a specific lookup-table implementation strategy and is
   unobservable to callers. A total pure function (`spec_gva_to_gpa`) is the
   correct abstraction for a total map. Fails Substitution (over-specifies HOW)
   and Minimal.

3. **Physical-memory bounds / `max_physical_address` / a `covered: Set<nat>`
   of tracked RAM.**
   Rejected — `gva_to_gpa` makes *no claim* about whether its output is backed
   by RAM (the caller filters with `frame::is_covered`, and MMIO GPAs such as
   the LAPIC at `0xFEE0_0000` legitimately lie outside RAM). These concepts
   belong to the out-of-scope functions `is_valid_physical_address` /
   `is_valid_physical_region` / `max_physical_address`. Including them would
   over-constrain the in-scope function and fails Minimal.

4. **A stateful View with a `mappings_installed` / page-table flag.**
   Rejected — encodes a hardware MMU/page-table strategy. On MicroVM the
   translation is unconditional identity regardless of page-table state; the
   caller observes no such flag. Fails Substitution and Caller-only.

5. **Make `spec_gva_to_gpa` a `closed` spec that only guarantees injectivity
   (hide identity).**
   Considered for maximal robustness, but rejected as the *primary* abstraction:
   the caller analysis lists identity as a MicroVM caller invariant, and the
   directly-usable caller fact is `result == gva`. Hiding identity would force
   callers to re-derive monotonicity/frame-distinctness. We instead expose
   identity (`open`) **and** the weaker `injective()` property, giving callers
   both the strong platform fact and a graceful-degradation handle.

6. **No View type at all (just a free spec fn).**
   Rejected for interface consistency: later phases reference "the View" as the
   shared spec vocabulary for the module. A named unit `MicrovmTranslationView`
   provides that anchor at zero modeling cost, while still honestly encoding
   "stateless."
