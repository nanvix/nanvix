# View Design: `mm::phys::frame`

> Status: the View for this module **already exists** in the codebase and is on
> the do-not-modify list (`FrameAllocView` + `FrameAllocView::wf` in
> `mod.spec.rs`; `View for Inner` in `frame.proof.rs`; the `PhysMemView`
> singleton wrapper + `phys_view()` in `mod.spec.rs`). This document reconstructs
> the design **from the caller analysis and public signatures only**, applies the
> substitution test to every field, and confirms the existing design is the right
> one (no change required). Rejected alternatives are recorded so future
> reviewers can see why each field is shaped the way it is.

---

## Abstract Resource

To callers, `frame` is **the system's single global pool of physical page
frames**: a fixed universe of page-aligned physical addresses partitioned into
*allocated* vs *free*, with a per-frame *reference count* layered on top so a
frame can be shared (copy-on-write) and is returned to the free pool only when
its last reference is released. Callers reach it through `pub(super)` free
functions over a singleton; they never see the backing bitmap or the BSS
refcount array.

Two distinct abstraction levels appear in the caller analysis, so the design has
two View layers:

1. **`FrameAllocView`** — the abstract reservation state of the allocator
   itself (what `Inner::*` methods transform).
2. **`PhysMemView`** — the module-level singleton wrapper that adds the boot
   `initialized` lifecycle gate and is the handle every free-function shim
   (`alloc`, `free`, `share`, …) states its contract over (`phys_view()`).

---

## View Struct

### Allocator view (mirrors `Inner@`)

```rust
pub struct FrameAllocView {
    /// Page-aligned physical addresses currently reserved (handed out or booked).
    pub allocated_frames: Set<int>,
    /// Page-aligned physical addresses currently available for allocation.
    pub free_frames: Set<int>,
    /// Per-frame reference count. A frame is a key iff it is allocated; the
    /// value (1..=255) is how many owners share it. Models copy-on-write.
    pub refcounts: Map<int, int>,
}
```

### Singleton view (the handle shims are stated over)

```rust
pub struct PhysMemView {
    /// Boot lifecycle gate: `false` before `init`, `true` after a successful
    /// `init`. Models "initialized-before-use"; `instance()` panics if false.
    pub initialized: bool,
    /// The global allocator's reservation state. Well-formed once initialized.
    pub frames: FrameAllocView,
}
```

All state is expressed in **mathematical types** (`Set<int>`, `Map<int,int>`,
`bool`) over **physical addresses**, never machine types, bit indices, or
pointers. A caller who has never read the implementation can name every field:
"which frames are allocated", "which are free", "how many owners each allocated
frame has", and "has the allocator been brought up yet".

---

## Well-formedness Invariant

```rust
// FrameAllocView::wf  (do-not-modify; reproduced for reference)
pub open spec fn wf(&self) -> bool {
    // Page-alignment of every tracked address.
    &&& forall|a: int| self.allocated_frames.contains(a) ==> a % spec_page_size() == 0
    &&& forall|a: int| self.free_frames.contains(a)      ==> a % spec_page_size() == 0
    // Allocated and free are disjoint.
    &&& self.allocated_frames.disjoint(self.free_frames)
    // Allocated <=> positive refcount.
    &&& forall|a: int| #[trigger] self.allocated_frames.contains(a) <==>
            self.refcounts.contains_key(a) && self.refcounts[a] > 0
    // Free frames carry no refcount entry.
    &&& forall|a: int| #[trigger] self.free_frames.contains(a) ==>
            !self.refcounts.contains_key(a)
    // Refcounts fit in a u8 (kernel caps sharers at MAX_PROCESSES <= 255).
    &&& forall|a: int| self.refcounts.contains_key(a) ==> 0 < self.refcounts[a] <= 255
}

// PhysMemView::inv  (do-not-modify)
pub open spec fn inv(self) -> bool {
    self.initialized ==> self.frames.wf()
}
```

These invariants encode exactly the caller-perspective facts from the analysis:
the **partition** (disjoint, page-aligned), the **refcount coupling**
(`allocated ⟺ refcount > 0`, free frames have no entry), the **u8 bound**
(`0 < c ≤ 255`), and **initialized-before-use** (well-formedness is only claimed
once the singleton is up). No bitmap- or array-specific clause leaks into the
caller-facing `wf`; the implementation-coupling invariant (bitmap bit ⟺ refcount
slot, tail-zero, slice length) lives separately in the do-not-modify
`Inner::internal_inv`.

---

## Spec Transition Functions

The mutating operations are expressed as transitions on the View. Two
complementary forms appear, both reusing only View vocabulary:

**Reusable helpers on `PhysMemView`** (do-not-modify; used by `mod`/`manager`):

```rust
impl PhysMemView {
    // Derived predicates (NOT fields — see Rejected Alternatives).
    pub open spec fn covered(self) -> Set<int> {
        self.frames.allocated_frames.union(self.frames.free_frames)
    }
    pub open spec fn region_frames(start: int, size: int) -> Set<int> {
        let first = start / spec_page_size();
        let last  = (start + size) / spec_page_size();
        set_int_range(first, last).map(|i: int| i * spec_page_size())
    }

    // State transitions.
    pub open spec fn spec_initialize(self, initial: FrameAllocView) -> PhysMemView {
        PhysMemView { initialized: true, frames: initial }
    }
    pub open spec fn spec_book_frame(self, addr: int) -> PhysMemView { /* free→alloc, rc=1 */ }
    pub open spec fn spec_book_frames(self, frames: Set<int>) -> PhysMemView { /* set form */ }
}
```

**Inline `FrameAllocView` literals** in the `Inner::*` contracts capture the
per-operation post-state declaratively (each is a pure set/map rewrite):

| Operation | Abstract transition on `self@` |
|-----------|--------------------------------|
| `alloc` (Ok) | pick `f ∈ free_frames`; `allocated += f`, `free −= f`, `refcounts[f] = 1` |
| `alloc_contiguous` (Ok) | `frames = {base + i·PAGE : 0≤i<count} ⊆ free`; `allocated ∪= frames`, `free ∖= frames`, each `refcounts = 1` |
| `book` (Ok) | `f ∈ free`; `allocated += f`, `free −= f`, `refcounts[f] = 1` |
| `alloc_range` (Ok) | `region_frames ⊆ free`; bulk move as above |
| `share` (Ok) | `f ∈ allocated`; `refcounts[f] += 1` (sets unchanged) |
| `free` (Ok) | if `refcounts[f] == 1`: `allocated −= f`, `free += f`, drop key; else `refcounts[f] −= 1` |
| `refcount` / `is_covered` / `free_count` | pure queries; no transition |

Each transition rewrites only the three abstract fields; the unchanged field(s)
are preserved explicitly (frame condition), matching the existing contracts.

---

## Design Rationale (per field, with substitution test)

> Substitution test: *"If the implementation were completely rewritten with a
> different algorithm (free-list, buddy allocator, B-tree of extents…), would
> this field still make sense?"*

- **`allocated_frames: Set<int>`** — every caller of `alloc`, `alloc_contiguous`,
  `book`, `alloc_range`, `share`, `refcount` asserts "the frame is now / is still
  in `allocated_frames`". *Substitution:* any allocator must know which frames it
  has handed out, regardless of how it stores that fact. ✅ Survives. Addresses
  (not bit indices) because callers hold `FrameAddress`/`PageAligned<…>` and
  compare via `frame@`.

- **`free_frames: Set<int>`** — `alloc`/`book`/`alloc_range` require the target be
  *in* `free_frames`; `free_count`'s watermark gate reasons over `free_frames`
  (`spec_watermark_ok`, `.finite()`/`.len()`); the partition invariant needs it.
  *Substitution:* "which frames are available" is intrinsic to any allocator. ✅
  Survives.

- **`refcounts: Map<int, int>`** — `share` (+1), `free` (−1, release on reaching
  0), and `refcount` (exact value) are *defined* by this map; fresh allocations
  pin `refcounts[f] == 1`. Copy-on-write is the whole reason the abstraction
  carries counts. *Substitution:* reference-counted shared ownership is a
  behavioral contract, not an implementation choice — a free-list or buddy
  rewrite still needs per-frame counts. ✅ Survives. Keyed by **address** and
  present **iff allocated**, so it doubles as the allocated-set witness via `wf`.

- **`PhysMemView.initialized: bool`** — every shim except `init` requires
  `phys_view().initialized`; `instance()` panics otherwise; `free` from `Drop`
  must preserve `inv()` which is gated on this flag. *Substitution:* a one-shot
  boot gate is inherent to a process-wide singleton, independent of its internals.
  ✅ Survives.

- **`PhysMemView.frames: FrameAllocView`** — composes the allocator state into the
  singleton handle the shims are specified over. *Substitution:* a singleton is
  exactly "lifecycle + the resource it guards". ✅ Survives.

**Derived, not stored:** `covered()` (= `allocated ∪ free`, the abstraction of
`is_covered`), `region_frames()` (the frame set of a region, used by `book` /
`alloc_range`), and the free count (`free_frames.len()` under `.finite()`) are
**methods/derivations**, never fields — they are computable from the three core
fields, so storing them would violate minimality.

---

## Quality Review

| Criterion | Verdict |
|-----------|---------|
| **Substitution** | Every field survives a complete allocator rewrite (sets of allocated/free addresses + refcount map + boot flag are algorithm-independent). ✅ |
| **Caller-only** | Each field names a concept from the caller analysis (allocated/free/shared/initialized). No bitmap, slice, pointer, or bit index appears. ✅ |
| **Complete** | All observed caller needs are expressible: alloc/book post-state, contiguity (`{base+i·PAGE}`), free/share refcount transitions, `refcount` value, `is_covered` (`covered()`), `free_count` watermark (`free_frames`), init gate. ✅ |
| **Minimal** | Each field is used by ≥1 contract; redundant candidates (free-count, covered-universe, bit-vector) are derived or rejected. ✅ |
| **No code-as-spec** | Transitions are set/map rewrites (`insert`/`remove`/`union`/`difference`), never bitmap scans or loop logic. WHAT, not HOW. ✅ |

---

## Rejected Alternatives

- **Bitmap as `Seq<bool>` / bit-vector indexed by frame number.** Mirrors the
  concrete `Bitmap`. Fails substitution — a free-list or buddy allocator has no
  bits. Callers never reason in bits; they hold addresses. ❌

- **Keying by frame *number* (bit index) instead of physical *address*.** The
  bitmap's natural key, but callers compare `frame@` (a physical address). Using
  indices would leak the `addr = i · PAGE_SIZE` indexing into every caller proof.
  Addresses keep the View in the caller's vocabulary. ❌ (The index↔address map
  `frame_addr_of` is correctly confined to `internal_inv` / the `view()` mapping,
  not the View surface.)

- **A `free_count: usize`/`nat` field.** Redundant: derivable as
  `free_frames.len()` (with `free_frames.finite()`). The watermark caller reasons
  over the set directly via `spec_watermark_ok`. Storing it would add an
  invariant-coupling burden for zero new information. ❌ (Kept as a derivation.)

- **A `covered: Set<int>` field.** Equal to `allocated_frames ∪ free_frames`.
  Provided as the `covered()` **method** instead, so `is_covered`'s contract reads
  naturally without a field that must be kept in sync. ❌ as a field, ✅ as a
  helper.

- **Refcounts as a parallel `Set<(int,int)>` or a single aggregate total.** A
  `Map<int,int>` is the precise abstraction: per-frame lookup for `refcount`,
  pointwise `+1`/`−1` for `share`/`free`. A set of pairs loses functional-map
  ergonomics; a total can't express per-frame queries. ❌

- **Refcount as `u8`/`Map<int,u8>` (machine type).** Specs should live in `int`
  to avoid overflow reasoning; the `≤ 255` bound is stated in `wf` instead. ❌

- **Dropping `allocated_frames` and deriving it from `refcounts.dom()`.**
  Technically `allocated = { a : refcounts[a] > 0 }`, so the field is *derivable*.
  **Kept explicit anyway** because (a) `wf` makes the equivalence a first-class,
  trigger-friendly fact callers cite directly (`allocated_frames.contains(f)`),
  and (b) the bulk transitions (`alloc_contiguous`, `alloc_range`) read far more
  cleanly as symmetric `union`/`difference` on `allocated`/`free` than as map
  filtering. This is a deliberate, minimal redundancy pinned by the invariant —
  not implementation mirroring. ✅ keep.

- **Folding `initialized` into `FrameAllocView`.** The boot gate is a property of
  the *singleton*, not of allocator reservation state; the `Inner::*` methods
  operate on an already-live allocator. Separating it into `PhysMemView` keeps
  `FrameAllocView` reusable and lets `inv()` express "well-formed once up". ❌
  (merging), ✅ (two-layer split).

---

## Notes on Boundaries (consistency with do-not-modify specs)

- `view()` for `Inner` is `closed spec fn` — public so callers can write `self@`,
  closed so the bitmap↔address mapping stays hidden. ✅ matches skill guidance.
- `FrameAllocView::wf` / `PhysMemView::inv` are `open spec fn` — caller-visible
  abstraction-level properties. ✅
- Reusable helpers (`covered`, `region_frames`, `spec_book_frame`,
  `spec_book_frames`, `spec_initialize`) live on the **View type**, not as extra
  `pub spec fn`s on `Inner`. ✅ matches "no extra pub spec fns on the impl".
- `phys_view()` is `uninterp` because the singleton lives in `static`s a `spec
  fn` cannot read; the exec shims pin its value through their `ensures`. This is
  the established design and the View this document validates — no change.
