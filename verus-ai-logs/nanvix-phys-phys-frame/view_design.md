# View Design: `mm::phys::frame`

> Status: the View for this module **already exists** and is on the
> do-not-modify list (`FrameAllocView` + `View for Inner` in `mod.spec.rs` /
> `frame.proof.rs`). This document **validates and records** that design against
> the caller analysis and the body-removed public signatures, applies the
> substitution test to every field, and captures the alternatives that were
> rejected. It proposes no changes to the locked definitions.

## Abstract Resource

To every (intra-crate) caller, `mm::phys::frame` is the **global pool of physical
page frames**: a fixed set of *covered* frames partitioned into **allocated** and
**free**, where each allocated frame carries a **reference count** (1..=255)
modelling shared ownership for copy-on-write. Callers obtain frames
(`alloc`/`alloc_contiguous`), reserve them out-of-band at boot
(`book`/`alloc_range`), share/release ownership (`share`/`free`), and query state
(`free_count`/`refcount`/`is_covered`).

Nothing a caller does is phrased in terms of "a bit in a bitmap" or "a byte in a
refcount table" — only in terms of *which frames are allocated/free* and *how
many owners each allocated frame has*. That is exactly the vocabulary the View
must provide.

## View Struct (existing — do not modify)

```rust
// mod.spec.rs
pub struct FrameAllocView {
    /// Physical addresses of frames currently reserved/handed out.
    pub allocated_frames: Set<int>,
    /// Physical addresses of frames currently available to hand out.
    pub free_frames: Set<int>,
    /// Maps each *allocated* frame address to its reference count.
    /// A frame is present in the map iff it is currently allocated.
    pub refcounts: Map<int, int>,
}
```

Three fields, all mathematical (`Set<int>`, `Map<int, int>`). Frame identity is
the **physical address** (`int`), matching `FrameAddress::view()` /
`PageAligned::view()` — the same token every caller already uses
(`frame@`, `phys_addr@`). No machine type (`usize`, `Bitmap`, `u8` slice,
`MaybeUninit`, `AtomicBool`) appears.

### How `Inner` realizes it (existing `View for Inner`, do-not-modify)

The mapping is `closed`, so this realization does **not** leak to callers; it is
shown only to confirm the View is faithfully implementable:

- `allocated_frames` = `{ frame_addr_of(i) | bit i set in bitmap }`
- `free_frames`     = `{ frame_addr_of(i) | 0 ≤ i < num_bits, bit i clear }`
- `refcounts`       = `{ frame_addr_of(i) ↦ refcount[i] | bit i set }`
- `frame_addr_of(i) = i * spec_page_size()`

The internal-vs-abstract boundary is clean: a complete rewrite (e.g. a free-list
or buddy allocator instead of a bitmap) would produce the *same three sets/map*
and require no change to any caller-facing spec.

## Well-formedness Invariant (existing — do not modify)

```rust
// FrameAllocView::wf  (mod.spec.rs)
pub open spec fn wf(&self) -> bool {
    // Page-alignment of every tracked frame address
    &&& forall|a| allocated_frames.contains(a) ==> a % spec_page_size() == 0
    &&& forall|a| free_frames.contains(a)      ==> a % spec_page_size() == 0
    // Allocated and free are disjoint partitions
    &&& allocated_frames.disjoint(free_frames)
    // Allocated ⇔ positive refcount
    &&& forall|a| #[trigger] allocated_frames.contains(a) <==>
            refcounts.contains_key(a) && refcounts[a] > 0
    // Free frames carry no refcount entry
    &&& forall|a| #[trigger] free_frames.contains(a) ==> !refcounts.contains_key(a)
    // Refcounts are within u8 ownership range
    &&& forall|a| refcounts.contains_key(a) ==> 0 < refcounts[a] <= 255
}
```

`Inner::inv() = self@.wf() && self.internal_inv()` ties the abstract `wf` to the
bitmap-level `internal_inv` (do-not-modify). Every `Inner` method requires
`old(self).inv()` and ensures `final(self).inv()`, so `wf` is preserved by all
operations — which is precisely the "all operations preserve `FrameAllocView::wf`"
invariant the caller analysis lists.

These six clauses are exactly the caller-observable consistency facts named in
the analysis's *Key Invariants* section: alignment, disjoint partition,
allocated⇔refcount>0, free⇒no-refcount, refcount≤255. None of them mentions an
implementation artifact.

### Subsystem wrapper (`PhysModView`, existing — do not modify)

The singleton has no spec-readable receiver, so the free-function layer reasons
through `phys_view(): PhysModView`, which embeds `frames: FrameAllocView` plus
two liveness booleans (`initialized`, `manager_ready`). This is where the
caller-analysis "*liveness depends on `init`*" fact lives (`initialized`), kept
out of `FrameAllocView` itself so the frame partition stays a pure data
abstraction. `PhysModView::inv` says `initialized ==> frames.wf()`.

## Spec Transition Functions

The state transitions are expressed directly as `FrameAllocView { ... }` literals
in the `Inner` method `ensures` (do-not-modify) and as named helpers on
`FrameAllocView` consumed by callers. The helpers below are the caller-facing
vocabulary layered on the three fields (all existing, in `mod.spec.rs` /
`manager.spec.rs`):

```rust
// Queries
covers(self, addr)        := allocated_frames.contains(addr)
                              || free_frames.contains(addr)        // is_covered
reserved(self, addr)      := allocated_frames.contains(addr)       // booked ⇒ alloc skips it
free_count(self)          := free_frames.len()                     // free_count()
all_reserved(self, set)   := forall a in set: reserved(a)
all_free(self, set)       := forall a in set: free_frames.contains(a)
user_alloc_ok(self, n)    := free_count() >= n + kernel_watermark  // manager watermark gate

// Transitions (pure functions returning a new FrameAllocView)
alloc_one(self, addr)     := move addr free→allocated, refcounts[addr]=1
book_all(self, set)       := move set  free→allocated, each refcount=1
book_covered(self, set)   := book_all(set.filter(covers))          // MMIO booking rule
```

Per-operation transition, as it appears in the locked `Inner` specs, restated in
View terms:

| Op | Ok-transition on `self@: FrameAllocView` | Err |
|---|---|---|
| `alloc` | `frame@ ∈ free` → `alloc_one(frame@)` | `free.is_empty()`, state unchanged |
| `alloc_contiguous` | `frames ⊆ free` → `book_all(frames)` (frames = `{base@+i·PS}`) | state unchanged |
| `alloc_range` | `frames ⊆ free` → `book_all(frames)` (frames = region's addrs) | `¬(frames ⊆ free)`, unchanged |
| `book` | `addr ∈ free` → `alloc_one(addr)` | `addr ∉ free`, unchanged |
| `free` (last ref) | `refcounts[f]==1` → remove from allocated, insert into free, drop refcount entry | `f ∉ allocated`, unchanged |
| `free` (shared) | `refcounts[f]>1` → `refcounts[f] -= 1` | — |
| `share` | `f ∈ allocated` → `refcounts[f] += 1` | `f ∉ allocated ∨ refcounts[f] ≥ 255`, unchanged |
| `refcount` | `f ∈ allocated`, return `refcounts[f]` | `f ∉ allocated` |
| `is_covered` | return `covers(addr)`; no state change | — |
| `free_count` | return `free_count()`; no state change | — |

Every right-hand side is written purely in the three View fields. The `..self`
frame condition is implicit in the helper definitions (`alloc_one`/`book_all`
rebuild all three fields, the unchanged ones from `self`).

## Design Rationale (substitution test per field)

For each field: *"If the implementation were rewritten with a different algorithm,
would this field still make sense?"*

### `allocated_frames: Set<int>` ✅

- **Why callers need it.** `book`/`alloc_range` promise the frame becomes
  reserved so `alloc` never hands it out; `share`/`refcount`/`free` require the
  frame to be *allocated*; `manager`/`upool` re-derive their own partitions from
  `phys_view().frames.allocated_frames.contains(frame@)`. `is_covered` is
  `allocated ∨ free`.
- **Substitution.** "The set of frames currently reserved" is an abstract fact
  any frame allocator maintains. A buddy/free-list/tree allocator would still
  have a well-defined allocated set. **Passes.**

### `free_frames: Set<int>` ✅

- **Why callers need it.** `alloc`/`alloc_contiguous`/`book`/`alloc_range` all
  draw only from free frames (`frame@ ∈ free`, `frames ⊆ free`); `free_count()`
  is `free_frames.len()` and drives the kernel watermark; `is_covered` is
  `allocated ∨ free`.
- **Substitution.** "The set of frames available to hand out" is
  implementation-independent. **Passes.**
- **Redundancy note.** Could be derived as `covered \ allocated` *if* the View
  also carried a `covered` set. It does not (see Rejected Alternatives): keeping
  `free_frames` explicit is what lets `is_covered`, `free_count`, and the booking
  preconditions be stated without a separate universe set, and lets the
  partition (`allocated ⊎ free`) define coverage directly. The two sets are the
  minimal pair that expresses "covered, and on which side".

### `refcounts: Map<int, int>` ✅

- **Why callers need it.** `share` (upool copy-on-write) increments and fails at
  255; `refcount` returns the exact value; `free` decrements and only returns the
  frame to `free_frames` when it hits 0; `alloc`/`book`/`alloc_range` set it to 1.
  `wf` ties `allocated ⇔ refcount>0`.
- **Substitution.** Reference-counted shared ownership is a *semantic* property of
  the resource (it is why `share` exists), not a storage choice. Any
  implementation supporting copy-on-write sharing must track per-frame owner
  counts. The fact that the real backing is a BSS `u8` array (`REFCOUNT_STORAGE`)
  is invisible: the View uses unbounded `int` with the `0 < r ≤ 255` bound stated
  in `wf`. **Passes.**
- **`int` not `u8`.** Spec world avoids machine widths; the 255 cap is a
  `wf`/`share`-error fact, not a type. Saturation is modelled as an *error*
  (`Err` when `refcounts[f] ≥ 255`), exactly matching the caller's `share`
  expectation.

### Frame identity = physical address (`int`) ✅

Callers already hold `FrameAddress`/`PageAligned<PhysicalAddress>` and speak
`frame@`/`phys_addr@`. Using the physical address as the set/map key means caller
specs need **no translation** between a "frame number" and the address they hold
— the substitution-stable, caller-native identity. `frame_addr_of(i)=i·PS`
converts the internal bitmap index, but that lives behind the `closed` `view()`.

## Quality Review

| Criterion | Verdict |
|---|---|
| **Substitution** | All three fields survive a complete reimplementation (buddy/free-list/tree). The bitmap, `u8` slice, `MaybeUninit`, `AtomicBool` are all hidden behind a `closed view()`. ✅ |
| **Caller-only** | Every field is referenced by caller specs without reading impl: `allocated_frames`/`free_frames` (manager, mod boot path), `refcounts` (upool `share`/`refcount`). ✅ |
| **Complete** | Every caller-observable concept maps to a field/helper: allocate, book, free, share, query count, query refcount, query coverage — all expressible. Liveness (`init`) is captured one level up in `PhysModView.initialized`. ✅ |
| **Minimal** | Each field is used by ≥1 caller spec; dropping any one breaks a caller (`refcounts`→share/refcount, `free_frames`→free_count/alloc precondition, `allocated_frames`→reserved/share). ✅ |
| **No code-as-spec** | Transitions are set/map operations (insert/remove/union/difference), never bitmap scanning or byte arithmetic. ✅ |

## Rejected Alternatives

1. **Mirror the bitmap: `bitmap: Seq<bool>` (+ `refcount: Seq<int>`).** Rejected —
   fails the substitution test outright: it bakes in "frames are bits in a dense
   array indexed by frame number". A non-bitmap allocator could not present this.
   It also forces callers to translate address↔index in every spec. The chosen
   `Set`/`Map` over addresses is algorithm-neutral and caller-native.

2. **Single `frames: Map<int, Option<int>>`** (address ↦ `Some(refcount)` if
   allocated, `None` if free). Rejected — conflates "covered" and "side of the
   partition", making the two most common caller predicates (`frame ∈ free`,
   `frame ∈ allocated`) require a pattern-match and an `is_Some`/`is_None` unwrap
   instead of a direct `Set::contains`. The disjoint-set form yields the
   simplest caller-side ensures (per spec-design "direct usability"). The
   `allocated ⇔ refcount>0` `wf` clause already provides the cross-link the merged
   map would have encoded structurally.

3. **Add an explicit `covered: Set<int>` field.** Rejected as non-minimal:
   coverage is fully derivable as `allocated_frames ∪ free_frames`
   (`FrameAllocView::covers`), and the only caller of `is_covered`
   (`book_mmio_regions`) is served by that helper. A third field would add a
   `covered == allocated ∪ free` consistency obligation to `wf` for zero new
   caller information.

4. **Track `total`/`capacity` (`number_of_bits`) as a field.** Rejected —
   `free_count()` is `free_frames.len()`, and no caller needs total capacity;
   the watermark check (`manager`) compares `free_count()` to a constant, not to
   capacity. Capacity is an internal sizing detail of the bitmap.

5. **Put `initialized` inside `FrameAllocView`.** Rejected — `FrameAllocView`
   should be a *pure data* abstraction of the partition; liveness/initialization
   is a subsystem-lifecycle fact. It is correctly placed one layer up in
   `PhysModView.initialized` (with `initialized ==> frames.wf()`), keeping the
   frame partition reusable and uncluttered.

6. **Refcount as `u8` / bound it via the type.** Rejected for spec hygiene
   (spec-design: prefer `int`, avoid machine widths). The `0 < r ≤ 255` bound is
   stated declaratively in `wf`, and the 255 saturation surfaces as a `share`
   error — both caller-visible facts rather than a representation constraint.

## Notes for downstream phases

- The View, `wf`, `View for Inner`, `internal_inv`, `frame_addr_of`,
  `byte_at_address`, and the existing `Inner::*` top-level specs are all
  **do-not-modify**. The free-function (`pub(super)`) wrappers are the surface
  still to be strengthened/verified; they currently reason through
  `phys_view().frames` (the `PhysModView` embedding) because the singleton has no
  spec receiver.
- `share`/`refcount`/`free_count`/`alloc`/`alloc_contiguous` wrapper specs are
  pinned to `phys_view().frames` rather than a local `self@` for that reason;
  `is_covered`/`book`/`alloc_range` wrappers presently carry no `verus_spec` and
  are the natural next targets, using the `covers`/`book_all`/`book_covered`
  helpers already defined on `FrameAllocView`.
