# View Design: `mm::phys::frame`

> **Re-run after rollback (specification → view-design, attempt 1/1).**
> The previous design modeled the global frame-allocator singleton with an
> **argument-free, uninterpreted constant** `phys_view() -> PhysMemView`. A
> constant has the same value at every program point, so it cannot distinguish the
> pre- and post-state of a *mutating* operation. The specification phase proved
> (reproducers `02`/`03`) that, under the only sound single-state `instance()`
> bridge, asserting a **post-state** allocation fact over that constant is not
> merely unproven but **provably false** (it contradicts `FrameAllocView::wf`
> disjointness). The mutating shims were therefore forced to weaken to *pre-state*
> facts, contradicting the documented caller expectations.
>
> **This revision fixes the View's *state-threading mechanism*, not its abstract
> content.** The abstract content (`FrameAllocView` = allocated/free sets +
> refcount map) is already correct and is on the do-not-modify list; it is
> reproduced here for reference and **kept verbatim**. What changes is *how the
> current value of the singleton is exposed to a verified shim*: the 0-ary constant
> `phys_view()` is replaced by a **tracked authority token** whose `&mut` carries
> the `old → post` transition. This is the "standard Verus pattern for global
> mutable state" named in the rollback report (option 2).

---

## Abstract Resource

To callers, `frame` is **the system's single global pool of physical page
frames**: a fixed universe of page-aligned physical addresses partitioned into
*allocated* vs *free*, with a per-frame *reference count* layered on top so a
frame can be shared (copy-on-write) and is returned to the free pool only when
its last reference is released. Callers reach it through `pub(super)` free
functions over a singleton; they never see the backing bitmap or the BSS
refcount array.

The abstraction has two layers and (this revision) one mechanism layer:

1. **`FrameAllocView`** — the abstract reservation state of the allocator
   (allocated set, free set, refcount map). *Do-not-modify; kept verbatim.*
2. **`PhysMemView`** — the singleton's abstract value: a boot `initialized`
   lifecycle gate plus the `FrameAllocView`. *Kept; transition helpers extended.*
3. **`PhysAuth` (NEW, tracked)** — the *carrier* of the current `PhysMemView`
   value. Replaces the 0-ary `phys_view()` constant so that a `&mut PhysAuth`
   exposes `old(auth)@` (pre) and `auth@` (post) of a mutation. This is the
   diff-able mechanism; it carries no new caller-observable *content*.

---

## View Struct

### Allocator reservation state — `FrameAllocView` (do-not-modify; verbatim)

```rust
pub struct FrameAllocView {
    /// Page-aligned physical addresses currently reserved (handed out or booked).
    pub allocated_frames: Set<int>,
    /// Page-aligned physical addresses currently available for allocation.
    pub free_frames: Set<int>,
    /// Maps each allocated frame address to its reference count.
    /// A frame is present in the map iff it is currently allocated. Value in 1..=255.
    pub refcounts: Map<int, int>,
}
```

### Singleton abstract value — `PhysMemView` (kept)

```rust
pub struct PhysMemView {
    /// Boot lifecycle gate: `false` before `init`, `true` after a successful
    /// `init`. Models "initialized-before-use"; `instance()` panics if false.
    pub initialized: bool,
    /// The global allocator's reservation state. Well-formed once initialized.
    pub frames: FrameAllocView,
}
```

### Diff-able carrier — `PhysAuth` (NEW, tracked)

```rust
/// Tracked ghost authority over the global frame-allocator singleton.
///
/// Its abstract value is the current `PhysMemView`. Exactly one `PhysAuth`
/// exists (minted by `init`, thereafter held by `PhysMemoryManager`); holding
/// `&mut PhysAuth` is the right to mutate the singleton and, crucially, exposes
/// BOTH the pre-state `old(auth)@` and the post-state `auth@` of that mutation.
///
/// `PhysAuth` adds NO new caller-visible content: `auth@` ranges over the SAME
/// `PhysMemView` values the previous design used. It only restores the ability
/// to *name two program points*, which a 0-ary spec constant cannot.
pub tracked struct PhysAuth { /* opaque ghost state */ }

impl PhysAuth {
    /// The singleton's abstract value at the current program point.
    pub spec fn view(self) -> PhysMemView;   //  auth@
}
```

All caller-visible state is expressed in **mathematical types** (`Set<int>`,
`Map<int,int>`, `bool`) over **physical addresses** — never machine types, bit
indices, or pointers. `PhysAuth` is `tracked` (ghost): it is erased at runtime
and exists only to thread the abstract value across a mutation.

---

## Well-formedness Invariant

```rust
// FrameAllocView::wf  (do-not-modify; reproduced verbatim for reference)
pub open spec fn wf(&self) -> bool {
    &&& forall|addr: int| self.allocated_frames.contains(addr) ==> addr % spec_page_size() == 0
    &&& forall|addr: int| self.free_frames.contains(addr) ==> addr % spec_page_size() == 0
    &&& self.allocated_frames.disjoint(self.free_frames)
    &&& forall|addr: int| #[trigger] self.allocated_frames.contains(addr) <==>
            self.refcounts.contains_key(addr) && self.refcounts[addr] > 0
    &&& forall|addr: int| #[trigger] self.free_frames.contains(addr) ==>
            !self.refcounts.contains_key(addr)
    &&& forall|addr: int| self.refcounts.contains_key(addr) ==>
            0 < self.refcounts[addr] <= 255
}

// PhysMemView::inv  (kept)
pub open spec fn inv(self) -> bool {
    self.initialized ==> self.frames.wf()
}
```

These encode exactly the caller-perspective facts from the analysis: the
**partition** (disjoint, page-aligned), the **refcount coupling**
(`allocated ⟺ refcount > 0`; free frames have no entry), the **u8 bound**
(`0 < c ≤ 255`), and **initialized-before-use**. No bitmap/array clause leaks
into `wf`; the implementation-coupling invariant lives in the do-not-modify
`Inner::internal_inv`.

The `PhysAuth` carrier needs no new well-formedness of its own beyond
`auth@.inv()`: it is a faithful holder of a `PhysMemView`, and its `inv()` is
just `self.view().inv()`.

---

## Spec Transition Functions

The mutating operations are expressed as **transitions on `PhysMemView`** (a pure
function `old_view → new_view`). The previous design already defined
`spec_initialize`, `spec_book_frame`, and `spec_book_frames`; this revision adds
the per-operation transitions the mutating shims need so they can write
`auth@ == old(auth)@.spec_<op>(...)` against the *post* state. All live on the
`PhysMemView` type (a View artifact), never as extra `pub spec fn`s on `Inner`.

```rust
impl PhysMemView {
    // ----- Derived predicates (methods, NOT fields — see Rejected Alternatives) -----
    pub open spec fn covered(self) -> Set<int> {
        self.frames.allocated_frames.union(self.frames.free_frames)
    }
    pub open spec fn region_frames(start: int, size: int) -> Set<int> {
        let first = start / spec_page_size();
        let last  = (start + size) / spec_page_size();
        set_int_range(first, last).map(|i: int| i * spec_page_size())
    }

    // ----- Lifecycle -----
    pub open spec fn spec_initialize(self, initial: FrameAllocView) -> PhysMemView {
        PhysMemView { initialized: true, frames: initial }
    }

    // ----- Reservation: free -> allocated, refcount := 1 -----
    /// `alloc` / `book`: a single free frame becomes allocated with one reference.
    pub open spec fn spec_alloc_one(self, addr: int) -> PhysMemView {
        PhysMemView {
            frames: FrameAllocView {
                allocated_frames: self.frames.allocated_frames.insert(addr),
                free_frames:      self.frames.free_frames.remove(addr),
                refcounts:        self.frames.refcounts.insert(addr, 1int),
            },
            ..self
        }
    }
    // `spec_book_frame` is retained as the existing name/alias of `spec_alloc_one`.

    /// `alloc_contiguous` / `alloc_range`: a whole set moves free -> allocated,
    /// each with refcount 1. (Generalizes a contiguous run to an arbitrary set.)
    pub open spec fn spec_alloc_set(self, frames: Set<int>) -> PhysMemView {
        PhysMemView {
            frames: FrameAllocView {
                allocated_frames: self.frames.allocated_frames.union(frames),
                free_frames:      self.frames.free_frames.difference(frames),
                refcounts:        self.frames.refcounts.union_prefer_right(
                    Map::new(|a: int| frames.contains(a), |a: int| 1int)),
            },
            ..self
        }
    }
    // `spec_book_frames` is retained as the existing name/alias of `spec_alloc_set`.

    // ----- Reference counting -----
    /// `share`: +1 reference on an already-allocated frame (sets unchanged).
    pub open spec fn spec_share(self, addr: int) -> PhysMemView {
        PhysMemView {
            frames: FrameAllocView {
                refcounts: self.frames.refcounts.insert(addr, self.frames.refcounts[addr] + 1),
                ..self.frames
            },
            ..self
        }
    }

    /// `free`: −1 reference; the last reference returns the frame to the pool.
    /// (Defined for completeness / for the manager-level reasoning that *can*
    ///  thread the token; the `frame::free` shim itself stays weak — see below.)
    pub open spec fn spec_free(self, addr: int) -> PhysMemView {
        if self.frames.refcounts[addr] == 1 {
            PhysMemView {
                frames: FrameAllocView {
                    allocated_frames: self.frames.allocated_frames.remove(addr),
                    free_frames:      self.frames.free_frames.insert(addr),
                    refcounts:        self.frames.refcounts.remove(addr),
                },
                ..self
            }
        } else {
            PhysMemView {
                frames: FrameAllocView {
                    refcounts: self.frames.refcounts.insert(addr, self.frames.refcounts[addr] - 1),
                    ..self.frames
                },
                ..self
            }
        }
    }
}
```

Each transition rewrites only the abstract fields; unchanged fields are preserved
via `..self` (frame condition). These mirror, value-for-value, the do-not-modify
`Inner::*` `old(self)@ → final(self)@` contracts — which is what makes the shim
proof a one-line `..self` equality after the `Inner::*` call.

---

## The Fix: a Diff-able Mechanism (core of this revision)

### Why the 0-ary constant is unfixable in specification

```text
phys_view() : PhysMemView          // 0-ary, uninterp  ==> a single constant value
instance() ensures (*r).inv() && (*r)@ == phys_view().frames    // pins r@ to PRE
```

In `fn alloc() { let r = instance(); r.alloc() }`:

- `instance()` gives `old(r)@ == phys_view().frames`.
- `r.alloc()` gives `final(r)@ == old(r)@.spec_alloc_one(frame)` (frame now allocated).
- The shim wants `ensures phys_view().frames.allocated_frames.contains(frame@)`.
- But `phys_view()` is the *same constant* at entry and exit, `== old(r)@`, in which
  `frame@` is **free**. By `wf` disjointness, `allocated.contains(frame@)` is **false**.

So the post-state fact is *provably false*, not just unproven (reproducer `02`).
Strengthening `instance()` to also pin the post-state forces `pre == post`, deriving
`false` for any mutation (reproducer `03`). A spec constant cannot name two program
points — this is intrinsic and cannot be repaired in the specification phase.

### The replacement: a tracked authority token

Replace the 0-ary constant with a **tracked carrier** whose `&mut` exposes both
program points:

```rust
// REMOVED:  pub uninterp spec fn phys_view() -> PhysMemView;
// ADDED:    pub tracked struct PhysAuth { .. }   with  spec fn view(self) -> PhysMemView;
```

`instance()` is redesigned to hand out the live `Inner` *together with* a tracked
borrow of the authority, with the bridge tying the live state to the token:

```rust
#[verus_spec((r, Tracked(auth)) =>
    requires old(auth)@.initialized,
    ensures
        (*r).inv(),
        (*r)@ == auth@.frames,        // live Inner agrees with the token (PRE)
        auth@ == old(auth)@,          // instance() itself does not mutate state
)]
fn instance<'a>(Tracked(auth): Tracked<&'a mut PhysAuth>) -> (r: &'static mut Inner)
```

A mutating shim then threads the token and re-synchronizes it to the post-state of
the `Inner::*` call:

```rust
#[verus_spec(result =>
    requires old(auth)@.initialized, old(auth)@.inv(),
    ensures
        auth@.initialized, auth@.inv(),
        match result {
            Ok(frame) => {
                &&& frame.inv()
                &&& auth@ == old(auth)@.spec_alloc_one(frame@)   // POST-STATE, true!
                &&& auth@.frames.allocated_frames.contains(frame@)
                &&& auth@.frames.refcounts[frame@] == 1
            }
            Err(_) => auth@ == old(auth)@,
        },
)]
pub(super) fn alloc(Tracked(auth): Tracked<&mut PhysAuth>) -> Result<FrameAddress, Error> {
    let r = instance(Tracked(auth.borrow_mut()));     // old(r)@ == old(auth)@.frames
    let res = r.alloc();                              // final(r)@ == old(r)@.spec_alloc_one(..)
    proof { auth.borrow_mut().sync_to(r) }            // auth@.frames := final(r)@  (ghost)
    res
}
```

Now `auth@` is the **post** value and `old(auth)@` is the **pre** value — two
distinct, nameable program points. The caller-expected fact
`auth@.frames.allocated_frames.contains(frame@) && refcounts[frame@] == 1` is
**true and provable**, discharging the rollback's central requirement. The
existing `Inner::*` transition contracts are reused unchanged; only the shim and
`instance()` gain the token.

`PhysAuth` carries **no new content** — `auth@ : PhysMemView` ranges over exactly
the same values the constant did. It only restores the two-program-point naming
that mutation reasoning fundamentally requires.

### Threading plan (who holds the token)

- **`init`** *mints* the unique `PhysAuth` from the supplied bitmap and yields it
  (`ensures auth@ == old(auth)@.spec_initialize(initial)`, `auth@.initialized`).
- **`PhysMemoryManager`** — the subsystem's stateful front door (`&mut self`
  methods) — *holds* `Tracked<PhysAuth>` and threads `&mut self.auth` into the
  reservation/share shims. `&mut self` naturally carries the pre/post of a manager
  operation, so the strong guarantees that previously lived in the
  `external_body` axioms `manager::alloc_user_frame` /
  `alloc_kernel_frame` / `alloc_many_user_frames` are now **derived** from the
  threaded shim transitions. Those axioms are deleted (they are *not* in
  `tcb-allowed.md`).
- **Boot reservation** (`mod::book_*` over the std `LinkedList`) threads the token
  through the `external_body` boundary already permitted in `tcb-allowed.md`;
  the booking effect is stated as `auth@ == old(auth)@.spec_alloc_set(frames)`.
- **Query shims** (`refcount`, `is_covered`, `free_count`) take `Tracked<&PhysAuth>`
  (shared, immutable) and read `auth@`; they remain pure and need no transition.

### `free` and the `Drop` path (the one operation that stays weak)

`frame::free` is called from `UserFrame::drop` / `KernelFrame::drop`, which are
`opens_invariants none` + `no_unwind` with the trait-fixed `drop(&mut self)`
signature. It therefore **cannot** receive a `Tracked<&mut PhysAuth>` ghost
parameter and **cannot** open a global invariant to reach the authority. Its
contract stays the weak, always-true:

```rust
#[verus_spec(result =>
    ensures auth_inv_preserved()   // subsystem invariant preserved on every path
    opens_invariants none
    no_unwind
)]
pub(super) fn free(frame: FrameAddress) -> Result<(), Error>
```

This is **sound and sufficient**: the caller analysis explicitly records that
`free`'s callers "don't care about the precise refcount value — the shim
deliberately cannot express the transition", and the only hard requirements are
*invariant preservation*, *no unwind*, and *no precondition* (so `Drop` is sound).
`spec_free` is still defined above for the manager-level reasoning that *can* hold
the token, but the `free` shim itself does not assert it. Keeping `free` weak is a
deliberate, caller-justified exception — **not** the forbidden weakening, which
concerned the *reservation* ops (`alloc`/`book`/`alloc_range`/`alloc_contiguous`/
`share`) that this design now strengthens.

> Design boundary recorded for the proving phase: the precise per-reference `free`
> transition would require a per-handle `Tracked<FrameRef>` permission owned by
> `UserFrame`/`KernelFrame` and consumed in `Drop` (the Rc-style refcount pattern).
> That is a larger, optional refinement; it is **not** needed to satisfy any
> documented caller, so it is left out to keep the change minimal.

---

## Design Rationale (per field, with substitution test)

> Substitution test: *"If the implementation were completely rewritten with a
> different algorithm (free-list, buddy allocator, B-tree of extents…), would
> this field still make sense?"*

- **`FrameAllocView.allocated_frames: Set<int>`** — every caller of `alloc`,
  `alloc_contiguous`, `book`, `alloc_range`, `share`, `refcount` reasons about
  "the frame is now / still in `allocated_frames`". *Substitution:* any allocator
  must know which frames it has handed out. ✅ Addresses (not bit indices) because
  callers hold `FrameAddress` and compare via `frame@`.

- **`FrameAllocView.free_frames: Set<int>`** — `alloc`/`book`/`alloc_range`
  require the target be *in* `free_frames`; `free_count`'s watermark gate reasons
  over it; the partition invariant needs it. *Substitution:* "which frames are
  available" is intrinsic to any allocator. ✅

- **`FrameAllocView.refcounts: Map<int,int>`** — `share` (+1), `free` (−1, release
  at 0), and `refcount` (exact value) are *defined* by this map; fresh allocations
  pin `refcounts[f] == 1`. *Substitution:* reference-counted shared ownership is a
  behavioral contract, not an implementation choice. ✅ Keyed by address, present
  **iff** allocated (doubles as the allocated-set witness via `wf`).

- **`PhysMemView.initialized: bool`** — every shim except `init` requires it;
  `instance()` panics otherwise; `free`-from-`Drop` must preserve `inv()` gated on
  it. *Substitution:* a one-shot boot gate is inherent to a process-wide singleton.
  ✅

- **`PhysMemView.frames: FrameAllocView`** — composes the allocator state into the
  singleton value. *Substitution:* a singleton is "lifecycle + the resource it
  guards". ✅

- **`PhysAuth` (carrier, tracked) — passes the substitution test as a *mechanism*,
  not content.** It stores no algorithm-specific data; its `view()` is a
  `PhysMemView`. *Substitution:* **any** implementation of a mutable global
  singleton, under **any** allocation algorithm, needs a way for a verified mutator
  to name the post-state — a threaded ghost token is the algorithm-independent way
  to do that. ✅ It is the *correct* generalization of the rejected 0-ary constant,
  which failed precisely because it pretended a mutable singleton could be named
  without threading.

**Derived, not stored:** `covered()` (= `allocated ∪ free`), `region_frames()`,
and the free count (`free_frames.len()` under `.finite()`) are
methods/derivations, never fields.

---

## Quality Review

| Criterion | Verdict |
|-----------|---------|
| **Substitution** | Every content field survives a complete allocator rewrite; `PhysAuth` is an algorithm-independent state-threading mechanism. ✅ |
| **Caller-only** | Each content field names a concept from the caller analysis (allocated/free/shared/initialized). No bitmap, slice, pointer, or bit index appears. `PhysAuth@` is just a `PhysMemView`. ✅ |
| **Complete** | All caller needs are now expressible **including post-state**: `alloc`/`book`/`alloc_range`/`alloc_contiguous` Ok ⇒ frame(s) in `allocated_frames` with refcount 1; `share` Ok ⇒ `refcounts[f]` incremented; `refcount`/`is_covered`/`free_count` queries; `free` invariant-preservation + Drop-safety. ✅ (This is the line-205 claim from attempt 1 that was previously **false** and is now **true**.) |
| **Minimal** | Each content field is used by ≥1 contract; `PhysAuth` is the single minimal addition that makes mutation expressible; derived predicates stay methods. ✅ |
| **No code-as-spec** | Transitions are set/map rewrites (`insert`/`remove`/`union`/`difference`), never bitmap scans. WHAT, not HOW. ✅ |
| **Sound** | No spec constant is asked to hold two values; `instance()` carries no contradictory `ensures`; `free` keeps a *true* (weak) contract. No false axiom. ✅ |

---

## Rejected Alternatives

- **0-ary `uninterp spec fn phys_view() -> PhysMemView` (the attempt-1 design).**
  A constant cannot name pre- and post-state of a mutation; under the only sound
  `instance()` bridge a post-state membership `ensures` is **provably false**
  (reproducer `02`), and strengthening the bridge derives `false` (reproducer
  `03`). This is the defect being fixed. ❌

- **A *pair* of 0-ary constants `phys_view_pre()` / `phys_view_post()`.** Lets a
  *single* shim state a transition, but cannot **sequence**: the `post` of one
  operation must equal the `pre` of the next, which two constants cannot express
  (they would have to be globally equal, collapsing them). Unsound/unusable for
  `book(a); book(b)` chains at the manager level. ❌ (Threading a token is the only
  form of "pre/post pair" that sequences — hence `PhysAuth`.)

- **Strengthening `instance()`'s `external_body` `ensures` to reflect post-state
  into the constant.** Forces `pre == post` for any mutation, i.e. an inconsistent
  (false) axiom on a trusted accessor — strictly worse than `admit`. ❌

- **Per-handle `Tracked<FrameRef>` permission for *every* operation (full Rc-style
  refcount state machine).** The most precise model (would let even `free` express
  its exact transition by consuming the handle's reference token in `Drop`). But it
  requires adding a `tracked` field to the `UserFrame`/`KernelFrame` exec structs
  and changing `free`'s signature — large, invasive, and **unnecessary**: no
  documented caller needs `free`'s precise transition. Recorded as a future
  refinement, not adopted. ❌ (for now)

- **Bitmap as `Seq<bool>` / bit-vector keyed by frame *number*.** Mirrors the
  concrete `Bitmap`; fails substitution (a free-list/buddy allocator has no bits);
  callers hold addresses, not indices. ❌

- **A `free_count: usize`/`nat` field.** Derivable as `free_frames.len()` (with
  `.finite()`); the watermark caller reasons over the set directly. ❌ (kept as a
  derivation).

- **A `covered: Set<int>` field.** Equal to `allocated ∪ free`; provided as the
  `covered()` method so `is_covered`'s contract reads naturally without a synced
  field. ❌ as a field, ✅ as a helper.

- **Refcount as `u8` / `Map<int,u8>`.** Specs live in `int` to avoid overflow
  reasoning; the `≤ 255` bound is in `wf`. ❌

- **Dropping `allocated_frames`, deriving it from `refcounts.dom()`.** Technically
  derivable, but kept explicit so `wf` makes the equivalence a trigger-friendly
  fact and the bulk transitions read as symmetric `union`/`difference`. A
  deliberate, invariant-pinned minimal redundancy. ✅ keep.

- **Folding `initialized` into `FrameAllocView`.** The boot gate belongs to the
  *singleton*, not to reservation state; separating it keeps `FrameAllocView`
  reusable. ✅ two-layer split.

---

## Downstream Consequences (to be realized by specification/proving)

1. **`mod.spec.rs`**: remove `pub uninterp spec fn phys_view()`; add
   `pub tracked struct PhysAuth` + `PhysAuth::view`/`inv`, and the new
   `PhysMemView::spec_alloc_one`/`spec_alloc_set`/`spec_share`/`spec_free`
   transitions (`spec_book_frame`/`spec_book_frames` retained as aliases).
   `FrameAllocView`, `wf`, `PhysMemView` fields, `covered`, `region_frames`,
   `spec_initialize` unchanged.
2. **`frame.rs`**: `instance()` gains the `Tracked<&mut PhysAuth>` bridge
   (`(*r)@ == auth@.frames`); the reservation/share shims thread the token and
   state `auth@ == old(auth)@.spec_<op>(...)`; query shims take `Tracked<&PhysAuth>`;
   `free` keeps its weak `opens_invariants none`/`no_unwind` contract. `Inner::*`
   contracts and `Inner::inv`/`internal_inv`/`View for Inner` untouched.
3. **`manager.rs` / `upool.rs` / `mod.rs`**: thread the token from the
   `PhysMemoryManager`-held `Tracked<PhysAuth>` down through the reservation/share
   call chain; restate the per-op effects over `auth@`. **Delete** the
   `external_body` strong-guarantee axioms `manager::alloc_user_frame`,
   `manager::alloc_kernel_frame`, `manager::alloc_many_user_frames` — their
   guarantees are now *derived* from the verified shim transitions, removing the
   `tcb-allowed.md` violation flagged in the rollback.

---

## Notes on Boundaries (consistency with do-not-modify specs)

- `View for Inner` / `Inner::inv` / `Inner::internal_inv` / `FrameAllocView` /
  `FrameAllocView::wf` / `frame_addr_of` / `byte_at_address` — **untouched.**
- `view()` for `Inner` stays `closed spec fn` (bitmap↔address mapping hidden);
  `wf` / `PhysMemView::inv` / `PhysAuth::inv` stay `open spec fn`.
- Reusable helpers (`covered`, `region_frames`, `spec_*`) live on the **View
  type** (`PhysMemView`), not as extra `pub spec fn`s on `Inner`. ✅
- `PhysAuth` is `tracked` (ghost), erased at runtime; it threads the abstract
  value with zero runtime cost and replaces the unsound 0-ary constant.
