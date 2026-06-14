# View Design: `arch::x86::mem::paging` (`mod.rs`)

## Status / TL;DR

**No non-trivial `View` is needed for this module.**

The single in-scope function, `invlpg`, is a free-standing `unsafe fn` whose
only effect is on the CPU's Translation Lookaside Buffer (TLB) — hardware state
that lives entirely outside Verus' memory model and is invisible to every
caller's Rust-visible state. There is no owned data structure, no `self`, and no
returned value to abstract. The faithful design is therefore a **degenerate
(empty) View** documenting an *external trust boundary*, with the function
specified as a total, side-effect-only `unsafe fn` (no `requires`, trivial
`ensures`). This matches the inherited upstream contract
(`assume_specification[...invlpg](vaddr: usize)` with no `requires`/`ensures`).

This document records that conclusion in full so the specification phase can
implement it without re-deriving it, and so the reasoning survives review.

---

## Abstract Resource

To callers, this module is **not** a container or a stateful manager. It is a
thin, trusted shim over a single hardware instruction. The abstract resource it
touches is the **CPU's TLB**: a hardware cache of virtual→physical address
translations. `invlpg(vaddr)` provides exactly one operation — *"invalidate the
cached translation for the page containing `vaddr`"* — used to keep the TLB
coherent after software updates a page-table (PTE) or page-directory (PDE)
entry.

Crucially, the TLB:

- is **not part of Rust's / Verus' memory model** — no `PointsTo`, no heap cell,
  no value a caller can read back;
- is **not owned** by any value in this module — `invlpg` is a free function with
  no `self`;
- has **no caller-observable success/failure** — the return type is `()` and the
  instruction is defined for every operand (it is a no-op when no matching entry
  exists).

So from the caller's perspective there is *no abstract state to carry across the
call*. The "state" affected is purely hardware microarchitectural state that the
verification model deliberately does not represent.

Pattern (from the view-design quick reference): **System boundary / IO →
Trusted Shim (external-style boundary) + Environment abstraction.** The
environment here (the TLB) is modeled as *unobservable*, which collapses the
abstraction to the empty/unit View.

---

## View Struct

```rust
/// Abstract state observable by callers of `arch::x86::mem::paging`.
///
/// This module exposes only `invlpg`, whose sole effect is on the CPU TLB —
/// hardware state outside Verus' memory model and invisible to Rust-visible
/// caller state. There is consequently no abstract state to represent, and the
/// View is intentionally empty (a unit/marker type).
pub struct PagingView {
    // Intentionally empty: the only operation (`invlpg`) affects unobservable
    // hardware TLB state and returns no value. No field survives the
    // substitution test (see Design Rationale), so adding any field would be an
    // abstraction leak.
}
```

There is no `self`-carrying type in this module, so `view()` does **not** attach
to an exec data structure. `PagingView` exists only to give the design a named
anchor and to make the "deliberately empty" decision explicit and reviewable. If
the specification phase prefers, this can be realized directly as the unit type
`()`; the empty struct is chosen here purely for documentation/naming clarity.

### `view()` / `internal_inv()` notes

- **`view()`** — Not applicable in the usual `&self -> PagingView` form, because
  no exec value owns this state. Were it materialized, it would be the constant
  empty view. (Signature/intent only; nothing to map because there are no impl
  fields backing observable state.)
- **`internal_inv(&self) -> bool`** — Placeholder, `true`. There is no
  implementation data structure whose consistency must be cross-checked, and the
  spec phase may see only the `asm!`-bearing body (an external/opaque
  operation). Leave as `true`.

---

## Well-formedness Invariant

```rust
pub open spec fn inv(&self) -> bool {
    // The empty View is always well-formed: there is no abstract state, hence
    // no constraint a caller could observe or rely on. Includes the (trivial)
    // internal invariant for uniformity with the module template.
    self.internal_inv()
}
```

`inv()` reduces to `true`. There is no abstraction-level well-formedness
property to expose because there is no abstract state. (Callers do carry their
*own* invariants — page-table well-formedness, mapping counts, allocator state —
but `invlpg` provably preserves all of them precisely *because* it touches none
of them; that preservation is a property of `invlpg`'s empty footprint, not of
this View.)

---

## Spec Transition Functions

```rust
// `invlpg` performs NO observable abstract-state transition.
//
// It does not mutate any Rust-visible state, returns `()`, and its only effect
// is on the (unmodeled) hardware TLB. There is therefore nothing for a spec
// transition to describe. No `spec_invlpg` is defined — defining one would have
// to either (a) return `self` unchanged (vacuous) or (b) invent fictional state
// (an abstraction leak).
```

Consequently, the `invlpg` contract carries **no `ensures` about abstract
state**. Its specification is:

- `requires`: none (any `usize` is accepted; the ring-0 obligation is the
  `unsafe` caller's responsibility and is not Verus-checkable here).
- `ensures`: trivial (`true`) — a successful return conveys only "the
  invalidation instruction was issued for `vaddr`".

This is exactly the inherited upstream shape:
`src/kernel/src/mm/virt/identity_map.spec.rs:151` declares
`pub assume_specification[ ::arch::mem::paging::invlpg ](vaddr: usize);` with no
`requires`/`ensures`. The module's own `mod.spec.rs` is empty (`verus! { }`).

### Implementation note for the spec phase

`invlpg` is in scope for verification and is **not** on the
`assume_specification` path inside its *own* module. Because its body is a single
`core::arch::asm!` block (an inherently un-modeled hardware effect), the
specification phase should annotate the original function with a trivial
contract (no `requires`, `ensures(true)`). If proving the `asm!` body against
even a trivial `ensures` is not expressible, the legitimate trust-boundary
mechanism is the inherited `assume_specification` for the hardware instruction —
**not** `external_body` (which is disallowed on the current module unless listed
in `tcb-allowed.md`). Confirm `invlpg`'s status against `tcb-allowed.md` before
choosing the mechanism.

---

## Design Rationale

Per the skill, every candidate field is run through the substitution test:
*"If the implementation were completely rewritten with a different algorithm,
would this field still make sense?"* For a TLB-flush shim, the only honest answer
for every conceivable field is **no** — the field would either describe
unobservable hardware or duplicate caller-side state this function never touches.

- **(empty)** — The View has no fields. Justification: the single operation has
  no return value and no Rust-visible footprint, so there is no caller-observable
  abstract concept to name. Adding any field would violate *Minimal* (no spec
  references it) and *No-code-as-spec / Abstraction Leak* (it would describe HOW
  the hardware works, not WHAT a caller observes). This is the correct outcome
  of the substitution test, not an omission.

### Inherited-View evaluation (per skill "Pre-existing View" step)

The caller analysis reports an inherited upstream spec but **no inherited View**:
the upstream kernel verification used a bare `assume_specification` with no
`requires`/`ensures` and explicitly recorded "View design: N/A". Evaluated
against *all* callers (`identity_map.rs`, `page_table.rs` ×5, `page_directory.rs`):

- **Every caller** uses `invlpg` identically — flush the TLB after writing/
  clearing a PTE/PDE — and **none** reads a result or relies on any abstract
  state from the call. No caller needs a field the others lack.
- The inherited empty contract therefore **passes the substitution test for all
  callers**. Nothing to keep/rename/add/remove at the field level because there
  are no fields. We **keep** the empty contract and formalize the "no View"
  decision as the explicit empty `PagingView` above.

### Quality review (Step 5)

| Criterion | Result |
|-----------|--------|
| **Substitution** | ✅ No field exists, so none could fail a rewrite; the empty View is rewrite-invariant by construction. |
| **Caller-only** | ✅ The (empty) View needs no implementation knowledge to understand. |
| **Complete** | ✅ Every caller-observable concept (there are none beyond "instruction issued") is representable — trivially. |
| **Minimal** | ✅ Zero fields; cannot be smaller. |
| **No code-as-spec** | ✅ Captures WHAT (an opaque, total TLB side effect) not HOW (`asm!`, AT&T syntax, register/flags options). |

---

## Rejected Alternatives

1. **`tlb: Set<usize>` / `Map<usize, ...>` modeling cached translations.**
   Rejected. The TLB is hardware state outside Verus' memory model; no caller
   can read it back, and `invlpg`'s only effect on it is unobservable. Modeling
   it would (a) fail the *Caller-only* test (callers never see TLB contents),
   (b) be an **Abstraction Leak** (spec describing microarchitecture), and
   (c) force a fictional `spec_invlpg` that "removes `vaddr`" from a set no one
   can query — an *Over-Faithful* anti-pattern with zero caller value.

2. **A `flushed: Seq<usize>` log / counter of issued invalidations.**
   Rejected. This describes HOW the function was used, not any state a caller
   observes, and no caller reads it. Fails *Minimal* (referenced by no useful
   spec) and is operational, not declarative.

3. **`last_vaddr: usize` (the most recent argument).**
   Rejected. Mirrors an input parameter, conveys nothing a caller can't already
   see at the call site, and would not survive a rewrite that, e.g., flushed the
   whole TLB. Pure *Over-Faithful* leakage.

4. **A boolean `valid` / error/status field.**
   Rejected. `invlpg` returns `()` and has no failure mode — the instruction is
   defined for any operand. There is no status to abstract; *Under-specified*
   does not apply because there is genuinely nothing more to say.

5. **Encoding the ring-0 / kernel-mode safety obligation as a `requires`.**
   Rejected as a View/spec field. Privilege level is not Rust-visible state and
   is not Verus-checkable from this function; it is the `unsafe` caller's
   contract (all call sites wrap it with a SAFETY note). Putting it in `requires`
   would invent an unprovable precondition rather than describe abstract state.

6. **A non-empty marker carrying `NUM_HIERARCHY_PAGES` / `PteWord` constants.**
   Rejected. Those `pub const`/`pub type` items are compile-time configuration
   exported by the module, not mutable abstract state of any value, and they are
   unrelated to `invlpg`. They belong to the consuming page-table/PDE/PTE
   modules' designs, not to a View for this function.
