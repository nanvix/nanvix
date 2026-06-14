# View Design: `mm::phys::upool`

## Abstract Resource

`upool` exposes two caller-facing types:

- **`UserFrame`** — an RAII *owning handle* to **one reference** of a
  reference-counted physical user frame. To a caller, a `UserFrame` is fully
  described by a single piece of abstract state: **the physical address of the
  frame it owns** (`int`). It carries no other observable state — the refcount
  and allocated/free status of that frame live in the *global* frame-allocator
  state (`phys_view().frames : FrameAllocView`), not inside the handle.

- **`Upool`** — a zero-sized (`_private: ()`) allocation *facade*. It owns **no
  abstract state at all**; it is merely the typed entry point that mints
  `UserFrame`s by forwarding to the global `frame` allocator. Its `&mut self`
  receiver is vestigial: the only state `alloc` mutates is the global allocator,
  not the `Upool` value.

The module therefore manages **ownership of frame references**, not frame
storage. The authoritative, mutable state (which frames are allocated, and each
frame's refcount) is the pre-existing, do-not-modify `FrameAllocView` reached
through `phys_view()`.

---

## View Struct

### `UserFrame` — keep the existing `int` view (recommended, unchanged)

```rust
// Already present in upool.rs — endorsed by this design, do not change.
impl View for UserFrame {
    type V = int;

    closed spec fn view(&self) -> int {
        self.addr@           // physical address of the owned frame
    }
}
```

`UserFrame@ : int` is the **complete** abstract state of the handle. The handle's
identity *is* the physical address of the frame it names; everything else a
caller cares about (is it allocated? what is its refcount?) is a property of the
*global allocator at that address*, queried as `phys_view().frames` keyed by
`self@`.

`view()` stays `closed`: callers reference `frame@` (already done by
`PhysMemoryManager::alloc_user_frame` / `alloc_many_user_frames`), but the fact
that it is stored as a `FrameAddress` newtype is hidden.

### `Upool` — no `View` (recommended)

`Upool` has no caller-observable state, so it gets **no `View` impl**. Adding one
would only be able to map to unit, which carries no information and would never
appear in a useful spec. `Upool::alloc`/`Upool::new` contracts are written
entirely against `phys_view()` (see Spec Transitions), so a `Upool` view is dead
weight. (If the verifier later requires *some* view for `&mut self`, a trivial
`type V = (); view() = ()` is acceptable, but it is intentionally omitted here.)

---

## Well-formedness Invariant

```rust
impl UserFrame {
    /// A user-frame handle is well-formed iff the frame it names is
    /// page-aligned (it denotes a real physical frame, not an arbitrary byte).
    pub open spec fn inv(&self) -> bool {
        self@ % spec_page_size() == 0
    }
}
```

This delegates to the alignment property that `FrameAddress::inv()` already
guarantees (`self@ % spec_page_size() == 0`). It is the only abstraction-level
constraint a caller can state about a `UserFrame` *in isolation*: a handle names
a page frame. Stronger facts — "this frame is currently allocated", "its refcount
is `n`" — are **not** invariants of the handle; they are properties of
`phys_view().frames` and must be expressed there, because `leak`/`drop`/`share`
change them without changing the handle.

No `inv()` is defined for `Upool` (no state to constrain).

---

## Spec Transition Functions

`UserFrame`'s own view is **immutable** — no operation changes `self@` (the
address an owned handle names never changes). Therefore the meaningful "state
transitions" of this module are transitions of the **global allocator view**
`phys_view().frames : FrameAllocView`, reached through `phys_view()`.

`FrameAllocView` is on the do-not-modify list, so these transitions are **not**
new methods on it. They are expressed in `upool.spec.rs` either inline in
`ensures` clauses over the existing fields (`allocated_frames`, `free_frames`,
`refcounts`) or, for readability, as standalone helper spec fns that take and
return a `FrameAllocView`. Proposed helpers:

```rust
// in upool.spec.rs — operate on (do not modify) FrameAllocView.

/// `share`: add one reference to an already-allocated frame.
pub open spec fn spec_add_ref(v: FrameAllocView, addr: int) -> FrameAllocView {
    FrameAllocView {
        refcounts: v.refcounts.insert(addr, v.refcounts[addr] + 1),
        ..v
    }
}

/// `drop` of a non-last reference: drop one reference, frame stays allocated.
pub open spec fn spec_drop_ref(v: FrameAllocView, addr: int) -> FrameAllocView {
    FrameAllocView {
        refcounts: v.refcounts.insert(addr, v.refcounts[addr] - 1),
        ..v
    }
}

/// `drop` of the last reference: refcount hits 0, frame returns to free.
pub open spec fn spec_release(v: FrameAllocView, addr: int) -> FrameAllocView {
    FrameAllocView {
        allocated_frames: v.allocated_frames.remove(addr),
        free_frames:      v.free_frames.insert(addr),
        refcounts:        v.refcounts.remove(addr),
    }
}
```

Per-function mapping (target functions in scope):

| Function | Effect on `UserFrame@` | Effect on `phys_view().frames` |
|---|---|---|
| `UserFrame::new(addr)` | `ret@ == addr@` | **none** — names an existing frame; no alloc, no refcount change |
| `UserFrame::address(&self)` | pure; `ret@ == self@` | **none** (pure getter) |
| `UserFrame::leak(self)` | `ret@ == self@` | **none** — suppresses `Drop`; frame stays allocated, refcount unchanged |
| `UserFrame::refcount(&self)` | pure | **none**; on `Ok(c)`, `c == old.refcounts[self@]` |
| `UserFrame::share(&self)` | `ret.ok@ == self@` | `Ok`: `final == spec_add_ref(old, self@)`; `Err`: `final == old` |
| `UserFrame::drop(&mut self)` | n/a (consumed) | non-last ref: `spec_drop_ref`; last ref: `spec_release`; keyed on `self@` |
| `Upool::new()` | n/a | **none** — no allocation, no global mutation |
| `Upool::alloc(&mut self)` | `Ok(uf)`: fresh frame, `uf@ % page == 0`, `uf@ ∈ final.allocated_frames`, `final.refcounts[uf@] == 1` | `Err`: `final == old` (nothing allocated) |

Notes:
- `alloc` chooses *some* free frame; its postcondition is best stated as
  membership (`uf@ ∈ final.allocated_frames`, `uf@ ∉ old.allocated_frames`,
  `final.refcounts[uf@] == 1`) rather than a deterministic transition, because
  the choice of frame is an allocator detail the caller cannot predict.
- `share`/`refcount` require `self@ ∈ old.allocated_frames` (the frame must be
  live to add a reference to / query it) — this matches `frame::share`/
  `frame::refcount` preconditions and is a property of `phys_view()`, not of the
  handle.
- The "no-change" rows (`new`, `address`, `leak`, `refcount`, `Upool::new`,
  and the `Err` arms) are real, required frame conditions: `final(phys_view) ==
  old(phys_view)`. They are what makes `leak`'s no-free guarantee and `share`'s
  failure-atomicity provable.

---

## Design Rationale

### `UserFrame@ : int` (physical address) — the single field

- **Substitution test**: *passes.* If `UserFrame` were re-implemented to store a
  frame *number*, a raw `usize`, a `PageAligned<PhysicalAddress>`, or anything
  else, the abstract identity callers reason about is still "the physical address
  of the owned frame." Every caller (page-table `map`, memcpy destination, PTE
  repointing, allocation-set membership, alignment checks) uses exactly this and
  nothing about the storage form.
- **Caller-only**: every cited caller already uses `frame@`/`view()` as the
  address; none depends on representation.
- **Complete**: combined with `phys_view().frames`, the address is sufficient to
  state every caller-observable property — allocation (`allocated_frames.contains(self@)`),
  refcount (`refcounts[self@]`), alignment (`self@ % page == 0`).
- **Minimal**: one field, used by every spec.
- **WHAT not HOW**: it names a frame; it does not describe how the handle stores
  or releases it.

### Why refcount / allocation status are **not** fields of `UserFrame`'s view

This is the crux of the design and the main substitution-test outcome.

A tempting alternative is to make `UserFrame@` a record like
`{ addr: int, refcount: int }` or `{ addr: int, live: bool }`. **Rejected**:

- A handle does not *own* the refcount; the refcount is shared mutable state of
  the global allocator. `share` on a sibling handle, or `drop` of an aliasing
  handle, changes the count of *this* frame without touching *this* handle — so a
  refcount field would instantly go stale and could never satisfy a frame
  condition. The count is correctly modeled as `phys_view().frames.refcounts[self@]`.
- It fails the substitution test in the other direction: it would bake the
  refcounting *strategy* into the handle's abstraction, even though refcounting
  is an allocator implementation choice (a different implementation could use a
  different ownership-tracking scheme while keeping the same `UserFrame` API).

So the handle's view stays a pure identity (`int`), and all mutation is routed
through the pre-existing, do-not-modify `FrameAllocView`. This keeps the two
abstraction boundaries clean: **`UserFrame@` = *which* frame; `phys_view()` =
*its* state.**

### `Upool` has no view

`Upool` is a zero-sized facade. Its abstract state is empty: `new` allocates
nothing and mutates no global state; `alloc` mutates only the global allocator.
A view could only be unit and would never appear in a spec, so none is defined.
This also keeps `Upool::alloc`'s contract honest — it speaks about
`phys_view()`, the state it actually changes.

### `inv()` = page-alignment only

The single well-formedness fact provable about a handle alone is that it names a
page-aligned frame. Everything else is conditional on global state and belongs in
`ensures`/`requires` over `phys_view()`. Keeping `inv()` minimal avoids coupling
the handle to allocator state it doesn't own.

---

## Rejected Alternatives

| Candidate | Why rejected |
|---|---|
| `UserFrame@ = { addr, refcount }` | Refcount is shared global state, not handle state; goes stale under sibling `share`/`drop`; fails substitution (bakes in refcounting strategy). Model as `phys_view().frames.refcounts[self@]`. |
| `UserFrame@ = { addr, live: bool }` | "Allocated?" is a property of the global allocator at `self@`, not of the handle; a leaked vs. dropped handle both stop existing yet leave the frame in opposite states. Use `phys_view().frames.allocated_frames.contains(self@)`. |
| `UserFrame@` stores `FrameAddress`/`PageAligned<…>`/frame-number | Representation leak; fails substitution. The caller-visible abstraction is the `int` address; keep `view()` `closed`. |
| `Upool@ = FrameAllocView` (mirror the allocator) | `Upool` does not own the allocator; it forwards to the global singleton. Duplicating allocator state in `Upool`'s view would create two sources of truth. Reference `phys_view()` instead. |
| `Upool@ = ()` (explicit unit view) | Harmless but pure dead weight — never used by any spec. Omit unless the verifier forces a view for `&mut self`. |
| New transition methods on `FrameAllocView` (`spec_add_ref`, …) added to `mod.spec.rs` | `FrameAllocView` is do-not-modify. Define the helper spec fns in `upool.spec.rs` taking/returning `FrameAllocView`, or inline the field updates in `ensures`. |
| Adding `inv()` clauses about allocation/refcount to `UserFrame` | Those facts are not invariants of the handle (they change without the handle changing). Keep `inv()` to alignment; state the rest in `requires`/`ensures` over `phys_view()`. |

---

## Quality Review

| Criterion | Result |
|---|---|
| **Substitution** | `int` address survives any storage rewrite; rejected fields explicitly fail it. ✅ |
| **Caller-only** | Address is the only thing callers use; refcount/allocation come from the already-public `phys_view()`. ✅ |
| **Complete** | Address + `phys_view().frames` expresses every caller concept (map, memcpy, PTE, membership, alignment, refcount, share atomicity). ✅ |
| **Minimal** | One field for `UserFrame`; zero for `Upool`; every field used in specs. ✅ |
| **No code-as-spec** | View states *which* frame is owned; transitions are declarative set/map updates on `FrameAllocView`, not a refcount algorithm. ✅ |
