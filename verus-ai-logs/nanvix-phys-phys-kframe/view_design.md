# View Design: `mm::phys::kframe` (`KernelFrame`)

> Status: the View already exists inline in `kframe.rs`
> (`impl View for KernelFrame { type V = int; view = self.base@ }`).
> This document **reviews and refines** that design against the caller
> analysis and the `view-design` / `spec-design` skills. Conclusion: the
> existing `int` view is correct and minimal; keep it unchanged. The
> material below records the rationale, the well-formedness predicate, and
> the spec transitions that later phases will attach to `new` / `base` / `drop`.

## Abstract Resource

`KernelFrame` is an **owned, move-only RAII handle to a single page-sized
physical frame** that has been allocated for kernel use and identity-mapped
into the kernel address space. To every caller, the only abstract fact about
a `KernelFrame` is **the base physical address of the frame it owns**. A live
handle corresponds to exactly one address in the global frame allocator's
`phys_view().frames.allocated_frames`; dropping it returns that address to
`free_frames`.

Callers never inspect storage layout, never copy the handle, and never read
its fields — they consume it through `@` / `base()` (address identity), through
`Deref`/`DerefMut`/`clear` (byte access, out of scope/TCB), and through `drop`
(RAII free).

## View Struct

The abstract value of a `KernelFrame` is a single physical address, so the
View is the scalar `int` — **not** a struct. This is the existing,
already-committed definition (do not change it):

```rust
impl View for KernelFrame {
    type V = int;                       // the base physical frame address

    closed spec fn view(&self) -> int {
        self.base@                      // FrameAddress::view() : int
    }
}
```

Why a scalar rather than a `KernelFrameView { base: int, ... }` struct:

- A `KernelFrame` carries exactly one piece of caller-observable abstract
  state (its address). A one-field struct would add ceremony with no
  expressive gain and would force every caller contract to write
  `frame@.base` instead of the more direct `frame@`.
- The surrounding `mm::phys` contracts already name frames purely by their
  `int` address — `phys_view().frames.allocated_frames.contains(frame@)`,
  `kernel_frames_contiguous(frames@, base)`, `frame_addr_of(i)`. An `int`
  view drops straight into that established vocabulary with no conversion.

`view()` is `closed` (the `self.base@` mapping is an implementation detail);
the *type* `int` is public so callers can reason with it.

## Well-formedness Invariant

The only well-formedness property callers rely on is **page-alignment** of the
address (used wherever frames feed into `frame_addr_of` / page-stride
contiguity reasoning). That property is already guaranteed structurally:
`KernelFrame.base : FrameAddress`, and `FrameAddress` carries

```rust
pub open spec fn inv(&self) -> bool {
    self@ % spec_page_size() == 0
}
```

so any well-formedness predicate on the handle reduces to the field's own
invariant:

```rust
// Recommended (optional) handle-level well-formedness, if a phase needs it:
pub open spec fn inv(&self) -> bool {
    self.base.inv()        // i.e. self@ % spec_page_size() == 0
}
```

Notes:

- There is **no internal bookkeeping** to fold in (no `internal_inv`): the
  struct has a single field whose own type-invariant carries the alignment.
- The "this frame is currently allocated" fact (`phys_view().frames
  .allocated_frames.contains(self@)`) is a relationship to **global** allocator
  state, not a self-contained predicate on the handle, so it belongs in the
  `new`/`drop` transition contracts below, **not** in `inv()`. Keeping it out
  of `inv()` avoids coupling the handle's well-formedness to a single fixed
  `phys_view()` snapshot (the same limitation the `frame::free` shim documents).

## Spec Transition Functions

`KernelFrame`'s abstract value is immutable for the handle's lifetime — there
are no value-mutating methods — so no `spec_*` transition is defined **on the
View type itself**. The relevant transitions are over the *global* frame
subsystem (`phys_view().frames`, a `FrameAllocView`) and are expressed by
reusing the already-defined allocator vocabulary. The intended contracts for
the in-scope functions:

### `new(base: FrameAddress) -> Result<Self, Error>`

```text
ensures
  // Address identity on success: the handle owns exactly the input address.
  match result {
      Ok(frame) => frame@ == base@,
      Err(_)    => true,          // failure says nothing about a (non-existent) handle
  }
  // Ownership is all-or-nothing: on Err no frame was consumed, so the global
  // allocator state is unchanged and `base` remains the caller's to free.
  //   (on Err)  phys_view() == old(phys_view())
```

Substitution-stable facts only: `frame@ == base@` (WHAT — the abstract value),
and *no consumption on `Err`*. The identity-mapping side effect
(`virt::identity_map_page`), the page-alignment-check ordering, and the
`PageAligned`/`PhysicalAddress` conversion path are all HOW and are deliberately
absent from the contract — no caller depends on them abstractly.

### `base(&self) -> FrameAddress`

```text
ensures
  result@ == self@        // pure read; abstract value of the returned address == handle's view
```

A trivial accessor (per `spec-design` Part 1: "trivial accessors need
`ensures result == self@.field` and no more"). Here the field *is* the view,
so the clause collapses to `result@ == self@`. No state change.

### `drop(&mut self)` — `Drop for KernelFrame`

```text
// no_unwind, opens_invariants none   (mirrors the `frame::free` shim contract)
ensures
  phys_view().inv()       // the phys subsystem invariant is preserved across the free
```

Dropping returns `self@` to the global allocator (`free(self.base)`), moving it
from `allocated_frames` toward `free_frames`. Per the caller analysis and the
`free` shim, the **exact** refcount/allocated→free set transition is *not*
expressible against a single fixed `phys_view()` (the shim documents this), so
the caller-meaningful guarantees are: invariant preservation, never panics,
never unwinds (`no_unwind`), and `opens_invariants none`. These are precisely
what make `Vec<KernelFrame>::clear()` a sound bulk-free and enable RAII rollback
in `alloc_many_kernel_frames` / `alloc_kpages`.

## Design Rationale

Applying the **substitution test** ("if the implementation were completely
rewritten with a different algorithm, would this survive?") to the one piece of
abstract state:

| Abstract element            | Survives rewrite? | Why it is caller-observable                                                                 |
|-----------------------------|-------------------|---------------------------------------------------------------------------------------------|
| `view() : int` = base addr  | ✅ Yes            | Any implementation of a kernel-frame handle must own *some* frame; its address is the only thing allocator contracts (`allocated_frames.contains`, `kernel_frames_contiguous`) and `KernelPage`/`KernelStack` callers ever name. Independent of how the address is stored. |

Why each in-scope contract clause is caller-written, not code-mirrored:

- `new ⇒ result@ == base@`: a caller (`alloc_kernel_frame`,
  `alloc_many_kernel_frames`) writes this directly to assert
  `allocated_frames.contains(frame@)` and the ascending page-stride contiguity
  predicate over a returned run. Could be written from the signature + module
  purpose alone — it never reads the body.
- `new (Err) ⇒ no consumption`: both call sites manually `frame::free(base)` on
  the error path; the spec must license that by guaranteeing `new` did not take
  ownership of `base` on failure.
- `base ⇒ result@ == self@`: `KernelPage::base` / `frame_address` consume the
  returned address as the frame's abstract identity; the clause is used as-is.
- `drop ⇒ inv() preserved + no_unwind`: the RAII rollback callers
  (`frames.clear()`, `kframes.clear()`) rely on bulk drop being panic-free and
  invariant-preserving.

Consistency with neighbours: this mirrors the `int`-address abstraction used
throughout `mm::phys` (`FrameAllocView` keys frames by `int`, `frame_addr_of`,
`phys_view().frames`), so no new vocabulary is introduced and no field is biased
toward a single caller.

## Rejected Alternatives

1. **Wrap the address in a struct `KernelFrameView { base: int }`.**
   Rejected — a single-field struct adds indirection (`frame@.base`) with zero
   added expressiveness, and diverges from the surrounding `int`-keyed allocator
   contracts. Fails the *minimal* criterion.

2. **Add an `identity_mapped: bool` (or a mapped-virtual-address) field.**
   Rejected — fails the substitution test: identity-mapping is *how* `new`
   makes `Deref`/`clear` sound, not abstract state any caller names. The caller
   analysis is explicit: "no caller depends on it abstractly." It is an
   internal precondition for the (TCB, out-of-scope) byte-access methods, not
   part of the handle's value.

3. **Include a reference count / shared-ownership field on the handle.**
   Rejected — refcounts live in the *global* `FrameAllocView.refcounts`, keyed
   by address. A `KernelFrame` is a move-only single-owner handle; exposing a
   refcount on it would mirror allocator internals and fails *caller-only* and
   *substitution*.

4. **Bake "currently allocated" into a handle `inv()`
   (`phys_view().frames.allocated_frames.contains(self@)`).**
   Rejected for `inv()` — it couples the handle's well-formedness to a single
   fixed `phys_view()` snapshot, the exact limitation the `frame::free` shim
   documents. The allocation relationship belongs in the `new`/`drop`
   *transition* contracts, where it is stated against the live `phys_view()`,
   not in a standalone invariant.

5. **Model the address with a machine type (`usize`) instead of `int`.**
   Rejected — the View lives in spec world; `int` avoids overflow reasoning and
   matches every existing `mm::phys` spec (`Set<int>`, `Map<int, int>`,
   `frame_addr_of(i) <= usize::MAX as int`). Representability is already implied
   by `FrameAddress::inv` plus the allocator invariant.

6. **Spec the success-only path of `new` and leave `Err` as `true`-everything
   (one-sided error spec).**
   Rejected — both callers depend on the *failure* semantics (no frame
   consumed; caller still owns `base`). The error path is given equal rigor via
   the "no consumption on `Err`" clause, per `spec-design` error-path
   principles.
