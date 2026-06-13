# View Design: `mm::phys::kframe` (`KernelFrame`)

## Abstract Resource

`KernelFrame` is an **owning handle to exactly one page-sized physical frame**.
To the outside world it is a single value: *which physical frame this handle
owns*. The only caller-observable abstract state is that frame's **physical
address**. Everything else — the identity-mapping side effect, how the address
is stored, the page-table mechanics, the byte contents reachable through
`Deref` — is internal or out of scope.

In-scope functions: `KernelFrame::new` (constructor), `KernelFrame::base`
(query), `KernelFrame::drop` (destructor).

---

## Inherited View (from upstream `mm::phys` manager verification)

```rust
impl View for KernelFrame {
    type V = int;
    closed spec fn view(&self) -> int { self.base@ }   // physical frame address
}
```

This View was added when the manager layer (`alloc_kernel_frame`,
`alloc_many_kernel_frames`) was verified top-down. It is treated here as an
**input to evaluate**, not a finished design.

### Evaluation against *all* callers

| Caller | Uses `self@` as… | Verdict |
|---|---|---|
| `manager::alloc_kernel_frame` | the allocated address: `kf@ == frame_addr@`, `lemma_kernel_alloc_one(..., result->Ok_0@)` | ✅ address |
| `manager::alloc_many_kernel_frames` | `kernel_frames_contiguous`, `kernel_addr_set(frames@)` — set/sequence of owned addresses | ✅ address |
| `virt::kpage::KernelPage::base` | `kframe.base().into_page_address().into_virtual_address()` — needs the address + page-alignment | ✅ address |
| `virt::kpage::KernelPage::frame_address` | returns `kframe.base()` verbatim | ✅ address |
| `Drop` sites (manager error path, `KernelStack::drop`) | identity of the frame being freed exactly once | ✅ address |

Every caller agrees: the abstract state is the **physical frame address**, an
`int`. No caller needs anything else.

**Decision: KEEP `type V = int` unchanged.** It already passes the
substitution test from every caller's perspective (see Rationale). Nothing is
renamed, added, or removed. The View definition is left exactly as inherited.

---

## View Struct

The View is a primitive, not a struct:

```rust
impl View for KernelFrame {
    type V = int;                       // the physical address of the owned frame
    closed spec fn view(&self) -> int { /* maps impl field -> address */ }
}
```

- `view()` stays **`closed`**: public so callers write `kf@`, closed so the
  field-level mapping (currently `self.base@`) does not leak.
- The value space is `int` — the mathematical physical address. (Per the
  view-design exception, an address could keep `usize`; here the upstream View
  and the whole `FrameAddress`/`Inner` address algebra already work in `int`
  via `View::V = int`, so `int` is the consistent, cast-free choice.)

A wrapper struct (`KernelFrameView { addr: int }`) was considered and
**rejected** — see Rejected Alternatives.

---

## Well-formedness Invariant

`KernelFrame` currently has **no** `inv()`. Callers of `base()`
(`into_page_address`, and the page/stack address arithmetic behind it) rely on
the owned address being **page-aligned**. This is a caller-visible constraint
on the abstract state, so it belongs in `inv()`:

```rust
impl KernelFrame {
    // Placeholder for implementation-consistency facts (e.g. "the stored
    // FrameAddress is well-formed", `self.base.inv()`). Cannot be written until
    // impl bodies are visible; the spec phase fills it in. Closed.
    pub closed spec fn internal_inv(&self) -> bool { true }

    // Caller-visible well-formedness: the owned frame address is page-aligned.
    // This is exactly `FrameAddress::inv` lifted onto KernelFrame's int view,
    // and it is what kpage / KernelStack address conversions depend on.
    pub open spec fn inv(&self) -> bool {
        &&& self.internal_inv()
        &&& self@ % spec_page_size() == 0
    }
}
```

- `inv()` is **`pub open`** so callers can establish and consume it.
- `internal_inv()` is **`pub closed`**, left `true` now; in the spec phase it
  will become `self.base.inv()` (the stored `FrameAddress` is page-aligned),
  from which the open clause `self@ % spec_page_size() == 0` follows because
  `self@ == self.base@`.
- `spec_page_size()` is the same `pub uninterp spec fn` already used by
  `FrameAddress::inv` (`self@ % spec_page_size() == 0`); reusing it keeps the
  alignment notion identical across the two types.

No other `pub spec fn` is added to `impl KernelFrame` beyond `view`, `inv`,
`internal_inv`.

---

## Spec Transition Functions

**There are none — and that is a deliberate, substitution-tested result.**

A `KernelFrame`'s abstract view is fixed at construction and **never changes**
for the lifetime of the handle. The in-scope API confirms this:

- `new(base)` is a **constructor**: it produces a fresh handle whose view *is*
  the input address. The contract is a constructor postcondition, not a
  transition on an existing view:

  ```rust
  // new (already external_body per tcb-allowed.md; keep its existing spec,
  // optionally surface the invariant it already guarantees):
  ensures match result {
      Ok(kf) => kf@ == base@ && kf.inv(),   // inv follows from required base.inv()
      Err(_) => true,                       // no ownership taken on failure
  }
  ```

- `base(&self)` is a **pure query** — no view change. Needed new spec:

  ```rust
  fn base(&self) -> (result: FrameAddress)
      requires self.inv(),
      ensures  result@ == self@ && result.inv();
  ```

  `result@ == self@` gives `kpage` the exact address; `result.inv()` (carried
  from `self.inv()`'s alignment clause) lets `into_page_address` and the stack
  arithmetic proceed.

- `drop(&mut self)` is a **destructor**. Its effect is *not* a transition on
  `KernelFrame`'s own view (the value is consumed). Dropping returns the frame
  `old(self)@` to the **global frame allocator**, so its postcondition is
  written against the allocator's View (`FrameAllocView` / `phys_view().frames`,
  the same View the manager already reasons over), e.g. "after drop,
  `old(self)@` is no longer allocated / is back in the free partition, and no
  other frame's status changes." Because that state lives in the allocator's
  abstraction, **no field is needed on `KernelFrameView` to express it**.

Consequently `impl KernelFrame`'s View needs no `spec_*` mutator helpers: the
view is immutable per handle, the only state change in the module
(allocation/free) is owned by the allocator's View.

---

## Design Rationale

### Field: the view value `self@ : int` — the physical frame address
- **Why needed.** It is the one thing every caller reasons about: the manager
  proves the wrapped address equals the allocated address and builds
  `kernel_addr_set` / contiguity facts from it; `kpage` converts it to page and
  virtual addresses; `drop` frees *that* address. There is no caller use of
  `KernelFrame` that does not reduce to its address.
- **Substitution test.** *If `KernelFrame` were rewritten to store a raw
  `usize`, a frame number, or to compute the address on the fly instead of
  holding a `FrameAddress` — would "the physical frame address" still make
  sense?* **Yes.** Any owning-frame-handle implementation must denote *some*
  physical frame; its address is the implementation-independent identity of
  what is owned. ✅
- **Caller-only.** A caller understands "the physical address of the frame this
  handle owns" with zero knowledge of the internal field, the identity map, or
  page tables.

### `inv()` clause: `self@ % spec_page_size() == 0` (page-aligned)
- **Why needed.** `base()` returns a `FrameAddress` that downstream code feeds
  to `into_page_address` and to `KernelStack` index arithmetic, both of which
  assume page alignment. Collecting this into `inv()` (instead of re-deriving it
  at every call site) follows the "missing wf()" anti-pattern fix.
- **Substitution test.** Alignment is a property of *what a physical frame is*
  (page-sized, page-aligned), not of how the handle stores it. Any
  implementation that owns a real frame yields a page-aligned address. ✅
- **Caller-abstract.** Stated purely on `self@` using the shared
  `spec_page_size()`; no implementation detail appears.

### `internal_inv()` left as `true`
- Per methodology, implementation-consistency facts (here: "the stored
  `FrameAddress` satisfies its own `inv`") require seeing impl fields, so they
  are deferred to the spec phase. The caller-visible consequence (alignment) is
  already exposed through `inv()`.

### Why no spec transition functions
- The handle's view is immutable; the only mutation in the module's lifecycle
  (a frame leaving the free partition on `new`'s upstream `alloc`, and returning
  on `drop`) is state of the **allocator**, captured by `FrameAllocView`, not by
  this per-object View. Modeling it here would duplicate (and risk
  desynchronizing) the allocator's abstraction.

---

## Rejected Alternatives

1. **Wrapper struct `KernelFrameView { addr: int }`.**
   Rejected for minimality. A one-field struct adds a layer of `.addr`
   projections to every caller spec (`kf@.addr` vs `kf@`) and a `view()` that
   constructs the struct, with zero added information. `type V = int` is the
   simpler, already-adopted, caller-agreed abstraction. The `inv()` predicate
   lives perfectly well on `impl KernelFrame`.

2. **Add an `allocated: bool` / ownership flag, or an allocator reference, to
   the View.**
   Rejected as an abstraction leak / wrong owner. Whether a frame is allocated
   is global allocator state, already modeled by `FrameAllocView`
   (`phys_view().frames`). The manager error path and `KernelStack::drop` reason
   about "freed exactly once" through the allocator's view, not through a flag
   on the handle. Duplicating it here would create two sources of truth that
   must be kept in sync. `drop`'s contract is therefore written against the
   allocator's View, leaving `KernelFrame`'s View minimal.

3. **Add a `mapped: bool` (identity-mapping) field.**
   Rejected. The caller analysis is explicit: callers *do not care* about the
   identity-mapping side effect of `new`. It is an internal effect of
   `mm::virt::identity_map_page` (outside `mm::phys` scope) and observable to no
   in-scope caller. Including it would leak HOW the handle is set up.

4. **Add the frame's byte contents (e.g. `Seq<u8>` / a `PointsTo`) to the
   View.**
   Rejected for this scope. Byte contents are only relevant to `Deref`,
   `DerefMut`, and `clear` — all explicitly **out of scope** and not to be
   touched. The in-scope functions (`new`, `base`, `drop`) concern only frame
   *ownership/identity*, i.e. the address. If/when the access functions are
   verified, the natural model for their bytes is a `PointsTo` over the
   identity-mapped region (keyed by `self@`), not a field baked into this
   ownership View — so adding it now would be premature and would destabilize
   the View the current callers already depend on.

5. **Change `view()` to `usize` instead of `int`.**
   Rejected for consistency. The entire surrounding address algebra
   (`FrameAddress`, `Inner`, `FrameAllocView`, the manager's
   `kernel_addr_set`/contiguity lemmas) is expressed with `View::V = int`.
   Switching this one View to `usize` would force casts at every interface with
   those specs for no benefit; the inherited `int` view is correct as-is.

6. **Strengthen `new`'s `Err(_) => true` into an explicit "frame not consumed"
   postcondition.**
   Considered (the caller does rely on `new` not freeing `base` on failure), but
   **out of scope for View design** and partly outside this module: the
   no-consume guarantee on the error path is currently provided by `new` being a
   trusted `external_body` dependency contract (it never touches the global
   allocator on `Err`) plus the caller's explicit `frame::free(base)` cleanup.
   The View needs no field to express it. Noted here for the spec phase; the
   View design is unaffected.

---

## Quality Review

| Criterion | Check |
|---|---|
| **Substitution** | The address (and its page-alignment) survive any rewrite of how the handle stores/derives the frame. ✅ |
| **Caller-only** | Every caller reads `kf@` as "the owned frame's physical address" with no impl knowledge. ✅ |
| **Complete** | All in-scope caller-observable concepts — owned-frame identity (`new`/`base`/`drop`), set/contiguity facts, page-aligned conversions — are expressible from `self@` + `inv()`. Allocator/ownership effects of `drop` are covered by the existing allocator View. ✅ |
| **Minimal** | Single `int` view; the one `inv()` clause is used by every `base()` caller. No unused fields. ✅ |
| **No code-as-spec** | The View states *what* (a physical address, page-aligned), never *how* (identity map, page tables, field layout). ✅ |
