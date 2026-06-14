# View Design: `mm::phys` (`src/kernel/src/mm/phys/mod.rs`)

Scope: the boot-orchestration layer — `init`, `book_physical_memory_regions`,
`book_mmio_regions`. These are free functions that operate on the **global**
frame-allocator singleton (`frame::instance()` / `INSTANCE_INIT`); none take a
`self`. The View therefore models the abstract state of the global physical
memory subsystem that these functions read and mutate.

Out of scope to modify (reused as-is): `FrameAllocView`, `FrameAllocView::wf`,
`Inner::inv`, `Inner::internal_inv`, `View for Inner`, `frame_addr_of`,
`byte_at_address`.

---

## Abstract Resource

To callers, `mm::phys` is the **global physical-memory reservation subsystem at
boot**. It has a *lifecycle* (uninitialized → initialized) and, once
initialized, a *reservation state* (which physical frames are reserved vs. free).
The one entry point callers use is `init`, whose abstract effect is "bring the
allocator up and pre-reserve all boot-known physical-RAM and tracked-MMIO
frames so the general allocator never hands them out."

The existing `FrameAllocView` already captures the reservation state of the
frame allocator (allocated/free sets + refcounts). What `FrameAllocView` cannot
express — and what is central to `init`'s contract — is the **lifecycle
transition** from "allocator does not yet exist" to "allocator established".
The phys-module View adds exactly that one dimension on top of the reused
`FrameAllocView`.

---

## View Struct

```rust
/// Abstract state of the global physical-memory subsystem managed by `mm::phys`.
///
/// Models the boot lifecycle of the frame-allocator singleton plus its
/// frame-reservation state. The `frames` component mirrors `frame::instance()@`;
/// the `initialized` flag mirrors the `INSTANCE_INIT` lifecycle gate.
pub struct PhysMemView {
    /// Whether the frame-allocator singleton has been established
    /// (i.e. `init` ran once and returned `Ok`). Models one-shot/monotonic
    /// boot: `false` before `init`, `true` after a successful `init`.
    pub initialized: bool,

    /// Abstract reservation state of the global frame allocator: which physical
    /// frames are reserved (allocated) vs. free, with per-frame refcounts.
    /// Meaningful (well-formed) only once `initialized` is `true`.
    pub frames: FrameAllocView,
}
```

`view()` for the exec carrier is `pub closed spec fn` (so the mapping to
`instance()@` / `INSTANCE_INIT` stays private); `inv()` below is
`pub open spec fn`.

---

## Well-formedness Invariant

```rust
impl PhysMemView {
    pub open spec fn inv(self) -> bool {
        // Once the allocator is established, the frame-allocator invariant holds.
        // Before initialization there is no allocator, so no constraint applies.
        self.initialized ==> self.frames.wf()
    }
}
```

`inv` deliberately constrains `frames` *only* in the initialized state. This
matches the caller contract: `init` is what **establishes** `FrameAllocView::wf`
(every later `frame::*` / `PhysMemoryManager::*` call relies on it), and an
uninitialized allocator has no meaningful allocated/free sets.

---

## Spec Helpers (on the View type)

```rust
impl PhysMemView {
    /// Frames the allocator actually tracks (covered by the bitmap):
    /// the union of reserved and free frames. A frame address is "covered"
    /// iff it is in this set — the abstract form of `is_covered`.
    pub open spec fn covered(self) -> Set<int> {
        self.frames.allocated_frames.union(self.frames.free_frames)
    }

    /// The set of frame addresses occupied by a region `[start, start+size)`.
    /// Region frames are the page-aligned addresses of the bitmap indices in range.
    pub open spec fn region_frames(start: int, size: int) -> Set<int> {
        let first = start / spec_page_size();
        let last = (start + size) / spec_page_size();
        vstd::set_lib::set_int_range(first, last).map(|i: int| i * spec_page_size())
    }
}
```

`covered()` and `region_frames()` are the abstract vocabulary the three target
functions need; placing them on the View keeps `impl Inner` free of extra public
spec fns (only `inv` / `view` there) and lets every spec reference one canonical
definition.

---

## Spec Transition Functions

Each mutating operation has a deterministic spec transition on the View. They
are written to compose exactly with the already-verified `Inner::book` /
`Inner::alloc_range` post-states, so `init`'s contract follows by composition.

```rust
impl PhysMemView {
    /// `init` established the allocator. Frame state starts from the bitmap the
    /// caller supplies (its initial free/allocated split); lifecycle flips on.
    pub open spec fn spec_initialize(self, initial: FrameAllocView) -> PhysMemView {
        PhysMemView { initialized: true, frames: initial }
    }

    /// Book one covered frame: move `addr` from free to allocated, refcount = 1.
    /// Mirrors `Inner::book`'s `Ok` post-state.
    pub open spec fn spec_book_frame(self, addr: int) -> PhysMemView {
        PhysMemView {
            frames: FrameAllocView {
                allocated_frames: self.frames.allocated_frames.insert(addr),
                free_frames: self.frames.free_frames.remove(addr),
                refcounts: self.frames.refcounts.insert(addr, 1int),
            },
            ..self
        }
    }

    /// Book a set of frames at once: move them all from free to allocated,
    /// each with refcount = 1. Mirrors `Inner::alloc_range`'s `Ok` post-state and
    /// generalizes it from a contiguous region to an arbitrary frame set.
    pub open spec fn spec_book_frames(self, frames: Set<int>) -> PhysMemView {
        PhysMemView {
            frames: FrameAllocView {
                allocated_frames: self.frames.allocated_frames.union(frames),
                free_frames: self.frames.free_frames.difference(frames),
                refcounts: self.frames.refcounts.union_prefer_right(
                    Map::new(|a: int| frames.contains(a), |a: int| 1int)),
            },
            ..self
        }
    }
}
```

### How the target functions map to transitions

- **`book_physical_memory_regions(regions)`** — on `Ok`, every frame of every
  region is booked. Let `R = union over regions of region_frames(start, size)`.
  Precondition: `R.subset_of(old.frames.free_frames)` (no region frame already
  reserved). Post-state: `new == old.spec_book_frames(R)`. On `Err`: state
  unchanged (`new == old`).

- **`book_mmio_regions(regions)`** — on `Ok`, the **tracked** MMIO frames are
  booked and untracked ones are skipped (not an error). Let `M` be the set of
  page-aligned MMIO frame addresses; the booked subset is `M.intersect(old.covered())`.
  Post-state: `new == old.spec_book_frames(M.intersect(old.covered()))`. This
  expresses "skip-if-not-covered" declaratively: untracked frames are simply
  outside `covered()` and never enter `allocated_frames`. On `Err`: `new == old`.

- **`init(physical_memory_regions, mmio_regions, layout)`** — composition. On
  `Ok`: `old.initialized == false` and
  `new == old.spec_initialize(layout_frames)
            .spec_book_frames(physical_R)
            .spec_book_frames(mmio_M.intersect(covered_after_phys))`,
  hence `new.initialized && new.frames.wf()` with all reserved-RAM and tracked
  MMIO frames in `new.frames.allocated_frames`. On `Err`: boot is terminal; no
  guarantee is offered about partially-booked intermediate state (the caller
  abandons the path), so the error branch only asserts that init failed
  (`!new.initialized` is acceptable / unconstrained — matching the caller, which
  consumes nothing on failure).

---

## Design Rationale (per field)

### `initialized: bool`
- **What it represents (caller terms):** whether the physical-memory subsystem
  is up. Before `init` no `frame::*` call is legal; after a successful `init`
  every later call may assume `FrameAllocView::wf`.
- **Substitution test:** ✅ Any implementation of boot setup — bitmap, buddy
  allocator, free-list, anything — has a notion of "has setup completed once."
  The flag describes the lifecycle, not the mechanism (`INSTANCE_INIT` is one
  way to implement it).
- **Used in specs:** `init` post (`Ok ⇒ initialized`), `book_*` and all later
  `frame::*` preconditions ("allocator established"), and `inv` (gates `wf`).
- **Why not folded into `FrameAllocView`:** `FrameAllocView` has no
  uninitialized state; an empty allocator (`{}`/`{}`) is indistinguishable from
  "never initialized," yet the two differ for callers. The flag is the minimal
  addition that makes the lifecycle expressible.

### `frames: FrameAllocView`
- **What it represents (caller terms):** which physical frames are reserved vs.
  free, with refcounts — exactly the state callers care about ("reserved frames
  are never handed out by `alloc`").
- **Substitution test:** ✅ Reserved/free frame sets + refcounts are the abstract
  outcome any physical allocator maintains, independent of bitmap vs. tree vs.
  list representation.
- **Used in specs:** every booking post-state (`spec_book_frame`,
  `spec_book_frames`), `covered()`, and `init`'s "all reserved/MMIO frames
  booked" guarantee.
- **Why reuse, not flatten:** composing the existing, already-verified
  `FrameAllocView` (and its `wf`) lets `book`/`alloc_range` post-states plug in
  directly, avoiding a second, divergent statement of the same truth.

---

## Quality Review

| Criterion | Result |
|-----------|--------|
| **Substitution** | ✅ Both fields survive a complete reimplementation (lifecycle + abstract reservation sets; no bitmap/refcount-slice/iteration detail leaks in). |
| **Caller-only** | ✅ Every field is explained in caller-visible terms ("is the subsystem up", "which frames are reserved"); no internal bookkeeping. |
| **Complete** | ✅ Covers all caller-observable concepts from the analysis: establishes `wf` (`initialized` + `inv`), reserved frames excluded from allocation (`frames`/`spec_book_frames`), tracked-vs-untracked MMIO (`covered`/intersection), one-shot init (`initialized`, `Ok ⇒ old.initialized == false`). |
| **Minimal** | ✅ Two fields; each is referenced by at least one transition or contract. Dropping either makes a target function's contract inexpressible. |
| **No code-as-spec** | ✅ Transitions describe *what* state results (set/map operations), never *how* (no frame-by-frame walk, GVA→GPA, bitmap `set`, or iteration order). |

---

## Rejected Alternatives

- **Reuse `FrameAllocView` directly as the module View (no wrapper).** The
  caller-analysis assessment notes `init` needs "no new abstract state beyond
  `FrameAllocView`," but that overlooks the **lifecycle**. `FrameAllocView`
  cannot distinguish "allocator not yet initialized" from "initialized and
  empty," so it cannot express `init`'s core promise (establishes `wf`,
  one-shot). Rejected: incomplete for the boot contract.

- **`mmio_regions` / `physical_regions` as View fields.** These are *inputs* to
  `init`, not persistent subsystem state. Their only abstract effect is "which
  frames end up reserved," already captured by `frames`. Adding them would mirror
  the call arguments rather than abstract state. Rejected: redundant, fails
  minimality.

- **A `covered: Set<int>` field (frames tracked by the bitmap).** Coverage is a
  *derived* quantity (`allocated ∪ free`), exposed as the `covered()` helper.
  Storing it separately risks inconsistency with `frames` and duplicates state.
  Rejected: derivable, not primitive.

- **Booking/iteration progress fields (e.g. `next_region`, `booked_so_far`).**
  These describe the per-region loop mechanism. Callers treat `init` as
  one-shot and `Err` as terminal — they never observe intermediate progress.
  Rejected: HOW, not WHAT; fails the substitution test.

- **Machine-typed fields (`usize` addresses, `&[u8]` refcount slice, `Bitmap`).**
  The View lives in spec world. The reused `FrameAllocView` already uses
  `Set<int>`/`Map<int,int>`. Rejected: not abstract.

- **An `error: Option<Error>` / failure-cause field.** The caller treats failure
  as terminal and consumes no partial state, so the cause is not
  caller-observable subsystem state; failure conditions belong in each
  function's `Err` branch, not the View. Rejected: not persistent state.

---

## Notes for Later Phases

- The exec carrier of `PhysMemView` is the **global** singleton: `frames`
  corresponds to `frame::instance()@`, `initialized` to `INSTANCE_INIT`. Because
  the target functions take no `self`, later phases will thread this View as the
  abstract pre/post-state of that global (e.g. via thin pass-through specs on the
  `frame::*` free-function wrappers, which currently lack specs). No exec struct
  or signature is changed by this design.
- `inv()` may be tightened as requires/ensures are written (per spec-design,
  Part 3), but the field set above is intended to be stable.
