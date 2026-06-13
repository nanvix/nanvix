# View Design: `hal::mem::types::address::frame` (`FrameAddress`)

## Abstract Resource

`FrameAddress` is a **handle to exactly one page-aligned physical memory
frame**. To every caller it denotes a single mathematical quantity: *the
physical base address of that frame*. From this one number a caller derives the
only two things it ever wants:

- the **raw physical address** — to do pointer arithmetic in `[0, PAGE_SIZE)`
  and to program the MMU (`into_raw_value`, `from_raw_value`);
- the **frame number** — `address / PAGE_SIZE` — to encode/decode PTE/PDE frame
  fields and to index refcount arrays (`into_frame_number`, `from_frame_number`).

Everything else — that it wraps `PageAligned<PhysicalAddress>`, how alignment is
checked, the concrete `Error` payload — is internal and caller-irrelevant.

In-scope functions: `FrameAddress` (type), `from_raw_value`, `into_raw_value`,
`from_frame_number`, `into_frame_number`.

---

## Inherited View (from upstream verification of the manager/MMU layers)

```rust
impl View for FrameAddress {
    type V = int;
    closed spec fn view(&self) -> int { self.0@ }   // the physical frame address
}

impl FrameAddress {
    pub open spec fn inv(&self) -> bool {
        self@ % spec_page_size() == 0               // page-aligned
    }
}
```

This is treated as an **input to evaluate against all callers**, not a finished
design.

### Evaluation against *all* callers

| Caller / function | Uses `self@` as… | Verdict |
|---|---|---|
| `into_raw_value` (19 sites: `vmem.rs`, `hwpt.rs`, `kframe.rs`, `mm/phys/manager.rs`) | the physical base address for pointer math + MMU: `result as int == self@` | ✅ address |
| `from_raw_value` (3 sites: `boot_init.rs`, `mm/phys/manager.rs`) | `fa@ == raw_addr`, and `Ok ⇒ inv()` | ✅ address |
| `into_frame_number` (7 sites: `page_table.rs`, `page_directory.rs`, `mm/phys/frame.rs::free`) | the frame index `self@ / PAGE_SIZE`: `result·PAGE_SIZE == self@` | ✅ address |
| `from_frame_number` (9 sites: page tables/dirs, `vmem.rs`, `mm/phys/frame.rs::alloc_any`) | `fa@ == n·PAGE_SIZE`, `Ok ⇒ inv()` | ✅ address |
| `Debug::fmt` (out of scope) | hex of the raw page-aligned address (`into_raw_value`) | ✅ address |
| `PartialEq::eq` (out of scope) | equality ⇔ same physical frame ⇔ equal `self@` | ✅ address |

Every caller agrees the abstract state is the **physical frame address**, an
`int`. No caller needs the inner `PageAligned<PhysicalAddress>`, the alignment
mechanism, or the error value. The frame-number identity is *derived* from the
address (`self@ / PAGE_SIZE`), not a second independent piece of state.

**Decision: KEEP `type V = int` and `inv()` unchanged.** Both pass the
substitution test from every caller's perspective. Nothing is renamed, added, or
removed.

---

## View Struct

The View is a primitive, not a struct:

```rust
impl View for FrameAddress {
    type V = int;                       // the physical address of the frame
    closed spec fn view(&self) -> int { /* maps impl field 0 -> address */ }
}
```

- `view()` stays **`closed`**: public so callers write `fa@`, closed so the
  field-level mapping (`self.0@`) does not leak the `PageAligned<PhysicalAddress>`
  representation.
- The value space is `int` — the mathematical physical address. The whole
  `PhysicalAddress` / `PageAligned` address algebra already exposes its view as
  `int` (`View::V = int`), so `int` is the consistent, cast-free choice and
  matches the sibling `KernelFrame` / `UserFrame` / `PhysicalAddress` views.

A wrapper struct (`FrameAddressView { addr: int }`) was considered and
**rejected** — see Rejected Alternatives.

---

## Well-formedness Invariant

```rust
impl FrameAddress {
    // Caller-visible well-formedness: the frame address is page-aligned.
    // Stated purely on the int view via the shared spec_page_size(); leaks no
    // implementation detail.
    pub open spec fn inv(&self) -> bool {
        self@ % spec_page_size() == 0
    }
}
```

- `inv()` is **`pub open`** so callers can both establish it (on `Ok` from the
  fallible constructors) and consume it (when handing a `FrameAddress` to the
  MMU / allocator paths).
- It captures exactly the one constraint every caller relies on: a
  `FrameAddress` always denotes a page-aligned frame, so `self@ / PAGE_SIZE` is
  exact and adding byte offsets in `[0, PAGE_SIZE)` stays inside the frame.
- `spec_page_size()` is the shared `pub uninterp spec fn` already used across the
  address types; reusing it keeps "page-aligned" identical everywhere.
- **No `internal_inv()` is defined.** The closed view `self@ == self.0@` exposes
  the alignment fact directly on the `int` view, so there is no separate
  implementation-consistency clause to hide; an `internal_inv()` placeholder
  would be permanently `true`, and the spec-design guidance discourages extra
  `pub spec fn`s on `impl FrameAddress` beyond `view`/`inv`. This mirrors the
  verified siblings `KernelFrame`/`UserFrame`.

No `pub spec fn` is added to `impl FrameAddress` beyond `view` and `inv`.

---

## Spec Transition Functions

**There are none — and that is a deliberate, substitution-tested result.**

`FrameAddress` is `Copy` and **immutable**: its abstract view is fixed at
construction and never changes. None of the in-scope functions mutate `self`;
they are constructors or by-value queries. So no `spec_<method>` transition
functions on the View are warranted. The per-function contracts below are
*design vocabulary* for the specification phase (written as ensures directly on
the `int` view), not View transitions:

| Function | Kind | Abstract contract (design intent for spec phase) |
|---|---|---|
| `into_raw_value(self) -> usize` | query | `result as int == self@` *(pre-existing)* |
| `from_raw_value(usize) -> Result<Self,_>` | constructor | `Ok(fa) ⇒ fa.inv() && fa@ == raw_addr as int`; `Err ⇒` nothing produced *(pre-existing ensures keeps only `Ok ⇒ inv()`; add `fa@ == raw_addr`)* |
| `into_frame_number(self) -> FrameNumber` | query | `result.into_raw_value() as int * spec_page_size() == self@` (inverse of `from_frame_number`) |
| `from_frame_number(FrameNumber) -> Result<Self,_>` | constructor | `Ok(fa) ⇒ fa.inv() && fa@ == frame_number.into_raw_value() as int * spec_page_size()`; `Err ⇒` nothing produced |

Round-trip identities the callers depend on (also spec-phase ensures, not
transitions):

- `from_raw_value(x).into_raw_value() == x` (for aligned, in-range `x`);
- `from_frame_number(n).into_frame_number() == n`.

The frame number is a **derived** quantity (`self@ / spec_page_size()`), so it is
expressed through `FrameNumber`'s own view in the ensures rather than stored as a
second View field — keeping the View minimal (single `int`).

---

## Design Rationale

- **`type V = int` (physical address).** Substitution test: rewrite the
  implementation to store a `u64`, a frame index, or a tagged pointer — the
  caller-observable state is still "which physical frame", i.e. the same address.
  Every one of the 4 in-scope functions and the two trait impls reduce to this
  one number (raw value *is* the address; frame number is `address / PAGE_SIZE`;
  `Debug` prints the address; `==` compares the address). It is biased toward no
  single caller. ✅ passes substitution, caller-only, complete, minimal,
  no-code-as-spec.
- **`inv()` = page-aligned.** Substitution test: any representation of a valid
  frame must sit on a page boundary, or the frame-number division and bounded
  pointer arithmetic that *all* callers perform would be wrong. It is a property
  of the abstract address, not of the storage. ✅
- **Single-field (primitive) View.** The frame number adds no information beyond
  the address (bijection via `· / · spec_page_size()`), so introducing a second
  field would be redundant state that every spec would have to keep consistent —
  an Over-Faithful anti-pattern. One `int` is complete and minimal.

---

## Rejected Alternatives

- **Wrapper struct `FrameAddressView { addr: int }`.** Rejected: a single scalar
  needs no struct; `type V = int` is lighter, avoids a `.addr` projection in
  every spec, and matches the sibling address-family views
  (`PhysicalAddress`, `KernelFrame`, `UserFrame` all use `V = int`). View
  consistency across the address layer is worth more than nominal typing here.

- **Two-field View `{ addr: int, frame: int }`** (address *and* frame number).
  Rejected: the two are mutually derivable (`frame == addr / spec_page_size()`,
  `addr == frame * spec_page_size()` under `inv()`), so `frame` is redundant
  state. It would force every ensures to maintain a consistency clause and invite
  drift — the Over-Faithful / non-minimal anti-pattern. The frame number is
  surfaced where needed via `FrameNumber`'s own view in the conversion contracts.

- **`type V` = frame number (index) instead of address.** Rejected: 19+
  call sites use `into_raw_value` for pointer math and MMU programming and need
  the *address*; making the index primary would push a `* PAGE_SIZE` into the
  hottest specs and is undefined for the (out-of-scope but real) sub-page
  reasoning callers do. Address is the more fundamental, lower-loss choice;
  frame number derives cleanly from it.

- **Keeping the View `usize` instead of `int`** (per the view-design address
  exception). Rejected: there are no `PPtr`/`PointsTo` specs on `FrameAddress`;
  the surrounding `PhysicalAddress`/`PageAligned` algebra and the inherited
  `view` already work in `int`. `int` is cast-free here and consistent with the
  rest of the address family.

- **Adding `internal_inv()`.** Rejected: it would be permanently `true` (the
  closed `int` view exposes alignment directly; there is no hidden redundant
  field to reconcile), and it would violate the "no extra `pub spec fn` beyond
  `view`/`inv`" guideline. Mirrors the verified `KernelFrame`/`UserFrame`.

- **Adding `spec_*` transition functions.** Rejected: `FrameAddress` is `Copy`
  and immutable; no in-scope function mutates an existing value, so there is no
  abstract state transition to name. Constructor/query contracts live as ensures
  on `view`/`inv`, not as View transitions.
