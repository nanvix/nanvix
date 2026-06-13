# View Design: `mm::phys` (`src/kernel/src/mm/phys/mod.rs`)

> Phase output. Designs the abstract `View` that all later specs
> (`requires`/`ensures`) for `init`, `book_physical_memory_regions`, and
> `book_mmio_regions` will reference.
>
> Built **only** from `caller_analysis.md` and the body-removed public API
> (`body_removed_source.rs`), plus the **existing, do-not-modify** spec
> definitions (`byte_at_address`, `FrameAllocView`, `FrameAllocView::wf`,
> `frame_addr_of`, `View for Inner`, `Inner::inv`, `Inner::internal_inv`).
>
> Target functions in scope: `init`, `book_physical_memory_regions`,
> `book_mmio_regions`. (`test` is out of scope.)

---

## 1. Abstract Resource (from caller analysis)

`mm::phys` is, to its one caller (`kernel_vas::init`), the **boot-time
constructor of the machine's global physical-memory subsystem**. After
`init(..) -> Ok(())` the caller depends on a *world* in which:

1. the **global frame allocator singleton is live** (initialized exactly once
   from the firmware bitmap) and internally consistent;
2. every frame of every supplied *non-usable* physical region has been
   **reserved** (booked) — moved out of the free pool so a later `alloc()`
   can never hand it out;
3. every **tracked** MMIO frame has been reserved, while MMIO frames the
   allocator does *not* track are **silently skipped** (coverage-gated);
4. the **`PhysMemoryManager` singleton + a fresh user page pool are live**
   (the layer the rest of boot builds on);
5. the whole thing satisfies the frame-allocator well-formedness
   `FrameAllocView::wf` (free/allocated disjoint, page-aligned,
   refcount-consistent).

`init` is the *only* externally observed function; `book_*` are private
helpers reachable only through `init`, so their contracts exist purely to
support `init`'s post-state. The caller observes **only `Ok(())` vs `Err`** —
never a partial state, never the iteration order, never the `Error` variant,
never whether MMIO is walked per-page.

These three functions are **free functions over global singletons**
(`frame::INSTANCE` + `frame::INSTANCE_INIT`, and the `PhysMemoryManager` /
`Upool` singletons), not `&mut self` methods. The View therefore models the
**abstract state of that global subsystem**, and the `v -> v'` transitions are
realized in the proof phase by a ghost token over the singletons (see §8) —
exactly as the bump-allocator View deferred its atomic-ghost token.

---

## 2. The View

The only abstract state these functions establish or mutate is: *is the
subsystem up*, *which physical frames are reserved vs. free*, and *is the
manager layer up*. The frame partition is **already** captured by the
do-not-modify `FrameAllocView` (the `View for Inner`). The phys-mod View wraps
it and adds the two liveness facts the caller depends on.

```rust
/// Abstract view of the global physical-memory subsystem managed by
/// `mm::phys`. Pure ghost description — names no `MaybeUninit`, `AtomicBool`,
/// bitmap, refcount slice, or any other storage mechanism.
pub ghost struct PhysModView {
    /// The frame allocator singleton has been initialized (`frame::init` ran
    /// successfully). All `frames`-related guarantees are meaningful only
    /// when this holds.
    pub initialized: bool,
    /// Abstract frame-allocator state: which physical frames are allocated
    /// (reserved) vs. free, with per-frame refcounts. This is the existing,
    /// do-not-modify `FrameAllocView` (== the `View for Inner` of the global
    /// `frame::INSTANCE`).
    pub frames: FrameAllocView,
    /// The `PhysMemoryManager` singleton has been initialized with a fresh
    /// user page pool (`Upool`). A single liveness bit — the caller observes
    /// only that this layer is up, never its contents.
    pub manager_ready: bool,
}
```

Attachment (no struct in `mod.rs` to `impl View` for; the state is global):

```rust
/// Current abstract state of the global physical-memory subsystem.
/// Uninterpreted accessor; the proof phase pins it to the frame /
/// manager singletons via a ghost token (§8).
pub uninterp spec fn phys_view() -> PhysModView;
```

`phys_view()` is read by specs exactly like `self@` would be: `init`'s ensures
talk about `phys_view()` before vs. after the call.

### 2.1 Derived spec helpers

Reusable frame-set vocabulary lives **on `FrameAllocView`** (a *new* `impl`
block — the existing `struct`/`wf` are untouched), so every spec speaks the
same language:

```rust
impl FrameAllocView {
    /// The allocator tracks (covers) the frame at `addr` — it is one of the
    /// frames this allocator knows about, allocated or free. Models
    /// `frame::is_covered`.
    pub open spec fn covers(self, addr: int) -> bool {
        self.allocated_frames.contains(addr) || self.free_frames.contains(addr)
    }

    /// The frame at `addr` is reserved: present in the allocated set, hence
    /// `alloc()` can never return it. This is the core caller-visible fact a
    /// "booked" frame satisfies.
    pub open spec fn reserved(self, addr: int) -> bool {
        self.allocated_frames.contains(addr)
    }

    /// Every frame address in `set` is reserved.
    pub open spec fn all_reserved(self, set: Set<int>) -> bool {
        forall|a: int| set.contains(a) ==> self.reserved(a)
    }

    /// Every frame address in `set` is currently free (booking precondition:
    /// a range can be booked only if it is entirely free).
    pub open spec fn all_free(self, set: Set<int>) -> bool {
        forall|a: int| set.contains(a) ==> self.free_frames.contains(a)
    }
}
```

Region → frame-set bridge (defined over the public region accessors
`start()`/`size()`; exact closed form finalized in the spec phase):

```rust
/// The set of page-aligned physical frame addresses covered by a physical
/// memory region `[start, start + size)`. Uses the existing `frame_addr_of`.
pub open spec fn region_frames(start_frame: int, num_frames: int) -> Set<int> {
    Set::new(|a: int| exists|i: int|
        0 <= i < num_frames && a == frame_addr_of(start_frame + i))
}
```

PhysModView convenience predicate used in `init`'s post-state:

```rust
impl PhysModView {
    /// The subsystem is fully brought up and self-consistent.
    pub open spec fn live(self) -> bool {
        self.initialized && self.manager_ready && self.frames.wf()
    }
}
```

---

## 3. Well-formedness Invariant `inv()`

```rust
impl PhysModView {
    pub open spec fn inv(self) -> bool {
        // (a) Once the allocator is up, the frame partition is well formed.
        //     `FrameAllocView::wf` = free/allocated disjoint, page-aligned,
        //     refcount/allocated consistency, refcounts in 1..=255.
        &&& self.initialized ==> self.frames.wf()
        // (b) The manager layer is built on top of the frame allocator, so it
        //     can only be up if the allocator is up.
        &&& self.manager_ready ==> self.initialized
    }
}
```

| Clause | Guarantees caller invariant |
|--------|-----------------------------|
| (a) `initialized ==> frames.wf()` | the downstream-relied-upon consistency (disjoint, aligned, refcount-consistent) holds whenever the allocator is live |
| (b) `manager_ready ==> initialized` | the manager singleton is never "up" over a dead allocator — rejects an init that builds the manager without seeding frames |

`inv()` is deliberately thin: the heavy lifting (`wf`) is reused from the
existing frame View, and the two implications encode the only ordering
constraint between the two liveness bits.

---

## 4. Spec transitions of the target functions

State is `v = phys_view()` (pre), `v' = phys_view()` (post). All clauses are
stated only over `PhysModView` / `FrameAllocView`; nothing names the
singleton storage.

### 4.1 Frame-set transitions (on `FrameAllocView`)

```rust
impl FrameAllocView {
    /// Reserve a single currently-free frame: move it from `free_frames` to
    /// `allocated_frames` with refcount 1. (Models `frame::book` /
    /// per-frame effect of `alloc_range` on a covered, free frame.)
    pub open spec fn book_frame(self, addr: int) -> FrameAllocView {
        FrameAllocView {
            allocated_frames: self.allocated_frames.insert(addr),
            free_frames: self.free_frames.remove(addr),
            refcounts: self.refcounts.insert(addr, 1),
        }
    }

    /// Reserve every frame in `set` (each must be free in `self`).
    /// Used to describe `alloc_range` over a region's frames.
    pub open spec fn book_all(self, set: Set<int>) -> FrameAllocView {
        FrameAllocView {
            allocated_frames: self.allocated_frames.union(set),
            free_frames: self.free_frames.difference(set),
            refcounts: Map::new(
                |a: int| self.refcounts.contains_key(a) || set.contains(a),
                |a: int| if set.contains(a) { 1 } else { self.refcounts[a] }),
        }
    }

    /// Reserve only the *covered* frames of `set`; skip the rest (coverage-
    /// gated). Models the MMIO booking rule.
    pub open spec fn book_covered(self, set: Set<int>) -> FrameAllocView {
        self.book_all(set.filter(|a: int| self.covers(a)))
    }
}
```

### 4.2 `book_physical_memory_regions(regions) -> Result<(), Error>`

Private; called once by `init`. Consumes the list by value. Let `R` be the
union of `region_frames(..)` over all regions in the list.

```
requires  v.inv() && v.initialized          // allocator must be live first

ensures   v'.inv() && v'.initialized && v'.manager_ready == v.manager_ready

          match result {
            Ok(()) => {
              // every region was entirely free, and is now entirely reserved
              &&& v.frames.all_free(R)
              &&& v'.frames == v.frames.book_all(R)     // hence v'.frames.all_reserved(R)
            }
            Err(_) => {
              // fail-fast on a booking conflict: some region was NOT free.
              // Booking stops at the first failing region; earlier regions
              // stay booked, so the partition is still well formed but only a
              // prefix is reserved. No in-scope caller observes this state.
              &&& !v.frames.all_free(R)
              &&& v'.frames.wf()
            }
          }
```

Error condition is stated as one **abstract interface predicate**
(`!all_free(R)` ⇔ a conflict with an already-reserved frame), not as a list of
code checks — this is the "fail-fast on conflict" property the caller relies on.

### 4.3 `book_mmio_regions(&regions) -> Result<(), Error>`

Private; called once by `init`. Borrows the list. Let `M` be the union of
`region_frames(..)` over the MMIO regions (after GVA→GPA translation; the
translation is total on the success path).

```
requires  v.inv() && v.initialized

ensures   v'.inv() && v'.initialized && v'.manager_ready == v.manager_ready

          match result {
            Ok(()) => {
              // tracked MMIO frames booked; untracked ones skipped, untouched.
              &&& v'.frames == v.frames.book_covered(M)
              // ⇒ forall f in M: v.frames.covers(f) ==> v'.frames.reserved(f)
              // ⇒ forall f in M: !v.frames.covers(f) ==> frame state unchanged
            }
            Err(_) => {
              // a covered frame was already reserved (book conflict) or an
              // address conversion failed; partition stays well formed.
              &&& v'.frames.wf()
            }
          }
```

The `Ok` arm makes the coverage gate explicit and surfaces both halves the
caller depends on: covered ⇒ reserved, **and** uncovered ⇒ *not* assumed
reserved (state unchanged for those frames).

### 4.4 `init(physical_memory_regions, &mmio_regions, physical_memory_layout) -> Result<(), Error>`

The only public function. Let `P` = union of `region_frames` over
`physical_memory_regions`, `M` = union over `mmio_regions`. `seed(bitmap)` is
the `FrameAllocView` produced by `frame::init` from the bitmap (frames it
tracks, partitioned per the bitmap's set bits).

```
requires  v.inv()
          !v.initialized            // one-shot: frame allocator not yet up
                                     // (frame::init rejects a second call)

ensures   v'.inv()

          match result {
            Ok(()) => {
              // (1) subsystem fully live & consistent
              &&& v'.live()                          // initialized && manager_ready && frames.wf()
              // (2) seeded from the bitmap, then phys regions booked, then
              //     covered MMIO frames booked
              &&& v'.frames == seed(physical_memory_layout@)
                                   .book_all(P)
                                   .book_covered(M)
              // (3) booked ⇒ never allocated (the core safety property)
              &&& v'.frames.all_reserved(P)
              &&& forall|f: int| M.contains(f) && v'.frames.covers(f)
                      ==> v'.frames.reserved(f)
            }
            Err(_) => {
              // boot aborts; caller uses `?` and never touches the subsystem
              // again. No partial-state guarantee is promised beyond inv().
              true
            }
          }
```

Notes:
- (1) `v'.live()` is the single fact downstream `virt::init` / page-table
  setup needs (both singletons up, `wf` holds). It rejects a buggy `init`
  that returns `Ok` while leaving a singleton uninitialized.
- (3) is the headline caller property ("booked ⇒ never allocated"); it is
  *derivable* from (2) (`book_all`/`book_covered` definitions) but surfaced
  directly because it is exactly what the caller writes into its proof.
- The `Err` arm is intentionally `true` (only `inv()` is promised): the caller
  aborts on `?` and the analysis states no in-scope caller observes any
  particular partial state. Strengthening it would over-specify an
  unobserved, non-transactional path.

---

## 5. Substitution test (per field)

> *"If `mm::phys` / the frame allocator were rewritten with a different
> algorithm (e.g. a buddy allocator, a free-list, a different singleton
> mechanism), would this field still make sense?"*

| Field | Survives? | Reasoning |
|-------|-----------|-----------|
| `initialized` | ✅ | "Is the physical-memory subsystem up?" is observable regardless of how the singleton is stored (`AtomicBool`, `OnceCell`, …). The caller depends on exactly this one-shot liveness. |
| `frames: FrameAllocView` | ✅ | The free/allocated/refcount partition is the abstract resource *any* frame allocator maintains; it is the existing `View for Inner`. A buddy/free-list rewrite still exposes the same `Set<int>` partition. Names no bitmap/refcount-slice. |
| `manager_ready` | ✅ | "Is the `PhysMemoryManager` + user page pool layer up?" is a caller-observable liveness bit independent of how the manager is built. A single bool — the caller observes liveness, never contents. |

Every field describes the **subsystem's observable state**, not a mechanism.
The implementation's `MaybeUninit<Inner>`, `AtomicBool`, `Bitmap`, and
`[u8; NFRAMES]` refcount storage are *one* realization; the View commits to
none of them.

Helper predicates pass too: `covers`/`reserved`/`book_*` are phrased over the
abstract frame sets (`is_covered` = "tracked", `book` = "move free→allocated"),
all algorithm-independent.

---

## 6. Design rationale

- **Reuse, don't re-derive, the frame partition.** `FrameAllocView` already is
  the correct abstract state for the allocator and is shared with the
  `frame` submodule's verified `View for Inner`. Embedding it (rather than
  re-modeling frames) keeps `init`/`book_*` specs interoperable with the
  frame-level contracts they call (`frame::book`, `frame::alloc_range`,
  `frame::is_covered`) and honors the do-not-modify constraint.
- **Two liveness bits, nothing more.** The caller's *additional* observations
  beyond the frame partition are exactly "allocator up" and "manager up". Each
  is a single bool, each is used (`initialized` gates `book_*` preconditions
  and `frames.wf()`; `manager_ready` appears in `init`'s `live()`
  post-state). Minimal and complete.
- **Coverage-gating modeled as a `filter`.** `book_covered = book_all ∘
  filter(covers)` makes "tracked ⇒ booked, untracked ⇒ skipped" one
  declarative line, capturing the LAPIC-above-RAM skip the caller is warned
  about — without walking GVA→GPA per page.
- **Reserve-set vocabulary on the View.** `reserved`/`all_reserved`/`all_free`
  + `region_frames` turn "booked ⇒ never allocated" and the booking
  precondition into reusable predicates, so each ensures is a clause a caller
  drops straight into a proof.
- **Error paths fold to `inv()` (+ a conflict predicate).** Booking is
  non-transactional (partial on failure) and no in-scope caller observes the
  partial state, so the honest contract is: `Err` preserves `wf` and (for
  `book_physical_memory_regions`) reveals the abstract conflict condition
  `!all_free(R)`. We do **not** fabricate state-preservation that the code
  does not provide.
- **`init` post-state as a composed transition.** `seed(..).book_all(P)
  .book_covered(M)` is a single declarative description of the whole boot
  sequence; the headline safety fact (3) is surfaced separately because that
  is the form the caller actually uses.

---

## 7. Rejected alternatives

1. **No phys-mod View; specs reference `FrameAllocView` (and the raw global)
   directly.** Rejected. The frame partition alone cannot express "the
   allocator is initialized" (one-shot `init` precondition) or "the manager
   layer is up" — both are caller-observed. A thin wrapper carrying the two
   liveness bits is the minimal honest model.

2. **Mirror the implementation: fields `instance_init: bool`,
   `bitmap: Seq<bool>`, `refcount: Seq<u8>`.** Rejected. Leaks the
   `AtomicBool` + `Bitmap` + refcount-slice mechanism, fails the substitution
   test (a buddy/free-list rewrite has no bitmap), and duplicates state the
   existing `FrameAllocView` already abstracts.

3. **Model the manager/Upool with a full sub-View
   (`manager: PhysMemoryManagerView`).** Rejected for *this* module. The
   in-scope functions only need "manager up" (one bit); the caller observes
   only liveness here. A structured manager View belongs to the `manager`
   module's own verification, not phys-mod, and would require reading its
   internals (forbidden, and unnecessary). Promote to a sub-View later only if
   a caller is shown to depend on manager contents.

4. **Track per-region booking results / fault flags in the View.** Rejected.
   Errors are transient return values the caller maps to abort-on-`?`; no error
   is part of the persistent abstract resource. Conflict conditions live in
   per-call `ensures` (`!all_free(R)`), not in `PhysModView`.

5. **Make the `Err` arm of `init`/`book_*` promise full state preservation
   (`v' == v`).** Rejected as unsound: booking mutates the global incrementally
   and stops at the first conflict, so earlier regions stay booked. The code
   does not roll back; a `v' == v` ensures would be a false spec. We promise
   only `inv()` (and the conflict predicate), which is all any caller observes.

6. **Represent reserved frames as a `Seq<int>` (ordered) instead of reusing
   `Set<int>` from `FrameAllocView`.** Rejected. Order of booking is explicitly
   *not* caller-observable (the analysis lists "ordering in which regions are
   iterated" as a non-breaking internal). Sets are the right abstraction and
   are already the established frame-View vocabulary.

---

## 8. Global-state modeling note (deferred to proof phase)

`init`/`book_*` take **no `self`**: they read and mutate the global singletons
`frame::INSTANCE` (+ `INSTANCE_INIT`) and the `PhysMemoryManager`/`Upool`
singletons. The `v -> v'` transitions in §4 are therefore not a literal
`old(self)`/`self` pair. They will be realized in the proof phase by a ghost
token whose invariant is exactly `PhysModView::inv()` and whose value is
`PhysModView`, with:

- `initialized`  ↔ `frame::INSTANCE_INIT`,
- `frames`       ↔ `frame::INSTANCE@` (the verified `View for Inner`, valid when
  `initialized`),
- `manager_ready`↔ the `PhysMemoryManager` singleton's init flag.

This is an *attachment* detail; the abstract View, `inv()`, and the transitions
above are the target the token machinery must satisfy and do not change. (Same
pattern the bump-allocator View used to defer its atomic-ghost token.)
