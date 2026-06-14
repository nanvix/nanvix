# View Design: `mm::phys::manager` (`PhysMemoryManager`)

> **Implementation note (spec phase outcome).** A bespoke `self@` View on
> `PhysMemoryManager` was found to be *unrealizable* and is **not** used by the
> implemented specs. `PhysMemoryManager { upool: Upool }` and `Upool` carry no
> fields — the manager is a **stateless facade** whose entire observable state is
> the *global* frame-reservation state behind `static mut PHYS_MEMORY_MANAGER` and
> the `frame::*` statics. A `view(&self)` over a field-less value is a constant, so
> it cannot witness global mutation (`old(self)@ == self@` always), and there is no
> `old(phys_view())` handle either. The implemented contracts therefore follow the
> established `frame.rs` free-function-shim pattern: every target method is
> `#[verus_verify(external_body)]` with a `#[verus_spec]` stated over the
> do-not-modify `phys_view()` / `FrameAllocView` plus the returned frame values —
> **monotone post-state facts** (e.g. "the returned frame's address is now in
> `allocated_frames`", `spec_watermark_ok`, `kernel_frames_contiguous`). The
> `FrameAllocView`-based vocabulary below is retained because the *contract content*
> (allocated/free sets, watermark, contiguity) is exactly what those `phys_view()`
> facts assert; only the *carrier* changed from `self@` to the global `phys_view()`.
> See `manager.spec.rs` for the realized helpers and `tcb-allowed.md` for the shim
> rationale.

## Abstract Resource

`PhysMemoryManager` is the kernel's **single global owner of physical RAM
frames**. Through a `&mut PhysMemoryManager` (obtained from the `get_mut`
singleton accessor) a caller hands out *exclusive ownership* of individual
physical frames in two qualities of service:

- **user frames** — refcounted (CoW-shareable), gated by the kernel watermark,
  not guaranteed contiguous;
- **kernel frames** — watermark-exempt, single or a physically-contiguous run.

The only state a caller reasons about across these operations is: **which
physical frame addresses are currently allocated vs. free, and the per-frame
reference count.** Callers never observe *how* frames are tracked (`Upool`,
bitmap, `MaybeUninit`/`AtomicBool` storage) — only the reservation state and
whether their request was honored with the right ownership/contiguity/watermark
policy.

---

## View Struct

The instance-level abstract state is exactly the global frame-reservation
state, which the codebase **already** models with the do-not-modify
`FrameAllocView`. The manager therefore reuses it as its `View::V` rather than
introducing a parallel type:

```rust
// (existing, do-not-modify — reproduced for reference only)
pub struct FrameAllocView {
    pub allocated_frames: Set<int>,   // frame addresses currently handed out
    pub free_frames:      Set<int>,   // frame addresses still available
    pub refcounts:        Map<int,int>, // allocated addr -> refcount (1..=255)
}

impl View for PhysMemoryManager {
    type V = FrameAllocView;

    // closed: the Upool/global-allocator -> abstract mapping is fixed during
    // proving and must not leak to callers.
    closed spec fn view(&self) -> FrameAllocView;
}
```

`self@` denotes the global frame-reservation state observable through this
manager: `allocated_frames` are the frames live owners hold, `free_frames` are
those an `alloc_*` call may still draw from, and `refcounts[addr]` is the share
count of an allocated user/kernel frame.

### Why not a bespoke `PhysManagerView`?

Every field a manager-specific view would carry (allocated set, free set,
refcounts) is already named, well-formed, and proven against by `FrameAllocView`
+ `FrameAllocView::wf`, and the upstream notes designate it as the anchor.
Reusing it keeps the manager contracts in the same vocabulary as `frame::*` and
the subsystem `PhysMemView`, avoids a redundant abstraction, and satisfies
**Minimal** (no new fields) and **No code-as-spec**.

---

## Lifecycle (`initialized`) is *not* an instance field

`init(upool)` is a **static** constructor for the singleton; it does not take
`&mut self`. Its caller-relevant effect — "after `Ok`, the subsystem is
initialized; every later `get_mut()`/`alloc_*` is valid" — is a property of the
**subsystem**, already modeled by the do-not-modify `PhysMemView { initialized,
frames }` and `phys_view()` (`spec_initialize` flips `initialized` on).

A caller that *holds* a `&mut PhysMemoryManager` is, by construction, past
`init`; per-instance the flag is a constant `true` and carries no information.
By the **substitution + minimality** tests an `initialized` field on the manager
View would be dead, so it is excluded. `init` is `external_body` (TCB-allowed via
`get_mut`/lifecycle reasoning) and states its post-condition over `phys_view()`,
not over `self@`.

---

## Well-formedness Invariant

```rust
impl PhysMemoryManager {
    pub open spec fn inv(&self) -> bool {
        // Frame-allocator well-formedness: page-alignment, allocated/free
        // disjointness, allocated <-> positive-refcount consistency, free
        // frames have no refcount, refcounts in 1..=255.
        &&& self@.wf()
        // Cardinalities must be finite so the watermark predicate can speak of
        // free_frames.len(). (Set::len is only meaningful on finite sets.)
        &&& self@.allocated_frames.finite()
        &&& self@.free_frames.finite()
    }
}
```

`wf()` is reused verbatim from `FrameAllocView::wf` (do-not-modify). The two
`finite()` conjuncts are the only abstraction-level additions, and they exist
solely because the watermark policy is stated over `free_frames.len()` (below);
without finiteness `.len()` is undefined.

`view()` is `closed`, `inv()` is `open` — matching the project rule that the
abstraction mapping stays hidden while the invariant is visible to callers.

---

## Spec Transition Functions & Policy Predicates

`FrameAllocView` is frozen, so manager-scoped helpers are declared as free
`spec fn`s in `manager.spec.rs` (they *consume/produce* `FrameAllocView`; they do
not extend its definition). These are the vocabulary the six target functions'
`requires`/`ensures` will be written in.

### Allocation = move free → allocated, refcount 1

The caller-observable effect of *every* successful single-frame allocation
(user or kernel) is identical: one frame leaves `free_frames`, enters
`allocated_frames`, gains refcount `1`. (Watermark is a *gate*, not part of the
post-state; contiguity is a property of the returned addresses, below.)

```rust
/// Post-state of allocating one previously-free frame `addr`.
pub open spec fn spec_alloc_frame(v: FrameAllocView, addr: int) -> FrameAllocView {
    FrameAllocView {
        allocated_frames: v.allocated_frames.insert(addr),
        free_frames:      v.free_frames.remove(addr),
        refcounts:        v.refcounts.insert(addr, 1int),
    }
}

/// Post-state of allocating a whole set of previously-free frames at once,
/// each with refcount 1. (Bulk user/kernel allocation.)
pub open spec fn spec_alloc_frames(v: FrameAllocView, fs: Set<int>) -> FrameAllocView {
    FrameAllocView {
        allocated_frames: v.allocated_frames.union(fs),
        free_frames:      v.free_frames.difference(fs),
        refcounts:        v.refcounts.union_prefer_right(
                              Map::new(|a: int| fs.contains(a), |a: int| 1int)),
    }
}
```

These deliberately mirror the existing `PhysMemView::spec_book_frame` /
`spec_book_frames` shapes (allocation and booking have the same abstract effect),
keeping the whole `mm::phys` module in one transition vocabulary. The
"all-or-nothing" bulk contract is captured by stating the success post-state as
`spec_alloc_frames(old, fs)` and the error post-state as `self@ == old(self)@`
(frame condition) — no partial-fill state is expressible, which is exactly the
caller guarantee.

### Watermark policy (user path only)

```rust
/// User allocations are admissible iff servicing `count` frames still leaves at
/// least KERNEL_WATERMARK frames free. Stated on the abstract free-set size, not
/// on any allocator counter.
pub open spec fn spec_watermark_ok(v: FrameAllocView, count: int) -> bool {
    v.free_frames.len() >= (config::kernel::KERNEL_WATERMARK as int) + count
}
```

- `alloc_user_frame` / `alloc_many_user_frames` succeed only when
  `spec_watermark_ok(self@, count)`; on `!spec_watermark_ok` they fail with
  `OutOfMemory` and leave `self@` unchanged.
- `check_user_watermark(count)` is exactly the decision procedure for this
  predicate (plus an `InvalidArgument` arm when `KERNEL_WATERMARK + count`
  overflows `usize`); it is the shared private gate, no state effect.
- `alloc_kernel_frame` / `alloc_many_kernel_frames` are specified **without**
  this predicate — kernel allocations bypass the watermark.

### Contiguity (kernel bulk path only)

Contiguity is a property of the **returned addresses**, not a View field. Using
the observable address accessors (`KernelFrame::base()`, `UserFrame::address()`),
the kernel bulk contract is stated over the returned sequence:

```rust
/// `addrs` is an ascending, page-stride contiguous run of length `n`.
pub open spec fn is_contiguous_run(addrs: Seq<int>, base: int) -> bool {
    forall|i: int| 0 <= i < addrs.len()
        ==> addrs[i] == base + i * spec_page_size()
}
```

- `alloc_many_kernel_frames` ensures the returned frames' base addresses satisfy
  `is_contiguous_run(.., base)` for some `base`, and that this run is precisely
  the set moved free→allocated in `spec_alloc_frames`.
- `alloc_many_user_frames` makes **no** contiguity claim (its `count` frames are
  just *some* set moved free→allocated) — matching "non-contiguity is explicitly
  fine."

---

## Mapping to Verification-Order Targets

| Function | Abstract effect over `self@` |
|---|---|
| `init` (static, TCB) | establishes subsystem; stated over `phys_view().spec_initialize(..)`, **not** `self@`. |
| `alloc_user_frame` | `requires`-able success when `spec_watermark_ok(self@,1)`; `Ok` ⇒ `self@ == spec_alloc_frame(old, a)`, returned `UserFrame.address()@ == a`, `a ∈ old.free_frames`. `Err` ⇒ `self@` unchanged. |
| `alloc_many_user_frames` | preconds: `frames` empty, `capacity ≥ count`. `Ok` ⇒ `self@ == spec_alloc_frames(old, fs)`, `|fs| == count`, `frames` holds exactly those addrs, watermark held. `Err` ⇒ `frames` empty & `self@` unchanged. |
| `check_user_watermark` | pure: `Ok` ⇔ `spec_watermark_ok(self@, count)` (no overflow); no effect. |
| `alloc_kernel_frame` | `Ok` ⇒ `self@ == spec_alloc_frame(old, a)`, returned `KernelFrame.base()@ == a`; **no** watermark gate. `Err` ⇒ unchanged. |
| `alloc_many_kernel_frames` | `Ok` ⇒ `self@ == spec_alloc_frames(old, fs)`, the addrs form `is_contiguous_run(.., base)`, `|fs| == count`; **no** watermark. `Err` ⇒ `frames` empty & `self@` unchanged. |

---

## Design Rationale (per field — substitution test)

| Element | Why needed | Substitution test ("rewrite with a different algorithm?") |
|---|---|---|
| `allocated_frames: Set<int>` | Callers must know a successful alloc yields a frame *not already owned* (no double-allocation); bulk/contiguity contracts name the moved set. | ✅ Any frame allocator — bitmap, free-list, buddy — has a notion of "currently handed-out frames." Survives. |
| `free_frames: Set<int>` | Source of new allocations; the watermark policy is stated over its cardinality. | ✅ Every allocator distinguishes available capacity; representation-independent. Survives. |
| `refcounts: Map<int,int>` | User frames are refcounted for CoW sharing; callers rely on "freshly allocated ⇒ refcount 1" and on the allocated⇔positive-refcount law. | ✅ Refcounting is a *caller-facing* QoS property of user frames, not a storage choice; any impl that supports CoW exposes it. Survives. |
| `inv := wf() ∧ finite` | Reuses proven frame-allocator law; finiteness makes `free_frames.len()` (watermark) well-defined. | ✅ Pure properties of the abstract sets/map; independent of allocator internals. |

The transition helpers (`spec_alloc_frame`, `spec_alloc_frames`),
`spec_watermark_ok`, and `is_contiguous_run` are all phrased over abstract
`Set`/`Seq`/`int` and the public `spec_page_size`/`KERNEL_WATERMARK` constants —
each survives a complete reimplementation because each restates *what* the
operation guarantees, never *how* it is computed.

---

## Quality Review

| Criterion | Result |
|---|---|
| **Substitution** | ✅ Each field describes state any frame allocator maintains; none mentions `Upool`/bitmap/atomic storage. |
| **Caller-only** | ✅ Allocated/free/refcount are precisely what `vmem`/`virt::manager` reason about; no internal bookkeeping exposed. |
| **Complete** | ✅ Covers ownership/no-double-alloc (`allocated_frames`), watermark (`free_frames.len()`), CoW (`refcounts`), all-or-nothing (`spec_alloc_frames` + frame condition), contiguity (`is_contiguous_run`), lifecycle (delegated to `phys_view`). |
| **Minimal** | ✅ Three fields, each used by ≥1 target's spec; `initialized` deliberately excluded as per-instance-constant. |
| **No code-as-spec** | ✅ Transitions are set/map algebra; the loop-and-push and two-phase cleanup mechanics are never modeled. |

---

## Rejected Alternatives

1. **`initialized: bool` on the manager View.** Rejected: `init` is static and
   the lifecycle is already modeled by the do-not-modify `PhysMemView` /
   `phys_view()`. Holding `&mut PhysMemoryManager` implies initialized, so the
   field is a per-instance constant — fails minimality/substitution.

2. **A bespoke `PhysManagerView` newtype duplicating allocated/free/refcounts.**
   Rejected: it would re-derive exactly `FrameAllocView` + `wf`, fragmenting the
   `mm::phys` vocabulary and adding proof overhead, with no caller-visible gain.

3. **Separate `user_frames` vs. `kernel_frames` allocated sets.** Rejected: the
   user/kernel distinction is a *policy on the allocation path* (watermark gate,
   contiguity guarantee), not a property of a frame once owned. The watermark is
   captured by `spec_watermark_ok`; contiguity by `is_contiguous_run`. Splitting
   the set would leak an implementation-irrelevant taxonomy and complicate the
   "a frame is allocated to exactly one owner" invariant. Fails substitution.

4. **`free_count: nat` / `watermark` as View fields.** Rejected: `free_count` is
   derivable as `free_frames.len()` (redundant); the watermark is a compile-time
   constant (`config::kernel::KERNEL_WATERMARK`), not state. Modeling either as a
   field violates minimality.

5. **`Upool` / bitmap / `MaybeUninit`+`AtomicBool` storage in the View.**
   Rejected outright: no caller observes these; they are exactly the "HOW" the
   abstraction must hide. Fails substitution and caller-only.

6. **A `contiguous: bool` flag in the View.** Rejected: contiguity is a property
   of a *specific returned run* at a call site, not a standing property of the
   manager's state. It belongs in `alloc_many_kernel_frames`'s `ensures` via
   `is_contiguous_run`, not in the View.
