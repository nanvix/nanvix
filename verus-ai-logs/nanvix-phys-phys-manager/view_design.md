# View Design: `mm::phys::manager` (`PhysMemoryManager`)

> Phase output. Designs the abstract `View` that all later specs
> (`requires`/`ensures`) for the six in-scope functions will reference:
> `init`, `alloc_user_frame`, `check_user_watermark`,
> `alloc_many_user_frames`, `alloc_many_kernel_frames`, `alloc_kernel_frame`.
>
> Built **only** from `caller_analysis.md` and the body-removed public API
> (`body_removed_source.rs`), plus the **existing, do-not-modify** spec
> definitions (`byte_at_address`, `FrameAllocView`, `FrameAllocView::wf`,
> `Inner::inv`, `frame_addr_of`, `View for Inner`, `Inner::internal_inv`) and
> the already-committed subsystem View `PhysModView` / `phys_view()`.

---

## 1. Abstract Resource (from caller analysis)

To its callers (`mm/virt`, all through `unsafe { get_mut() }`), the
`PhysMemoryManager` is the **singleton broker over the global pool of physical
page frames**. It hands out *typed, owning frame handles*:

- **`KernelFrame`s** — drawn straight from the frame allocator, **contiguous on
  request**, **never watermark-gated** (kernel allocation must succeed whenever
  *any* physical frame is free).
- **`UserFrame`s** — drawn for user mappings and **watermark-gated**: a user
  request is rejected (`OutOfMemory`) if fulfilling it would drop the number of
  free frames below `KERNEL_WATERMARK`, so the kernel always keeps a reserve.

The only abstract state these six functions observe or mutate is therefore the
**frame partition**: which physical frames are allocated vs. free, with
per-frame refcounts, and — derived from it — *how many frames are free* (the
quantity the watermark gate reads). That partition is **already** modeled by the
do-not-modify `FrameAllocView` (== the verified `View for Inner` of the global
`frame::INSTANCE`, and == `PhysModView.frames`).

Callers explicitly **do not** observe: the `upool` internals, the
`MaybeUninit`/`AtomicBool` singleton storage, the watermark *threshold value*,
the chosen physical addresses, or how contiguity is obtained. They observe only:
*which frames became reserved*, *that user paths enforce the watermark*, *that
kernel bulk frames are contiguous*, and the *all-or-nothing / no-leak* error
behavior.

---

## 2. The View — reuse `FrameAllocView`, add no new struct

Because the six functions are `&mut self` methods, Verus already gives us the
`old(self)@ -> self@` transition pair for free — no global-token deferral is
needed for the *receiver* (unlike phys-mod's free functions). The receiver's
abstract state is exactly the frame partition it brokers, which is precisely
`FrameAllocView`. Wrapping it in a new one-field struct would add indirection
and zero information, so:

```rust
impl View for PhysMemoryManager {
    type V = FrameAllocView;

    // Closed: the manager brokers the *global* frame partition (frame::INSTANCE).
    // The concrete mapping to that singleton is pinned in the proof phase by the
    // same ghost-token attachment PhysModView uses (see §8); here it is read like
    // any `self@`.
    closed spec fn view(&self) -> FrameAllocView;
}
```

So `self@ : FrameAllocView`, and every spec for these functions speaks the
**existing frame-partition vocabulary** (`reserved`, `all_free`, `covers`,
`book_all`, the `allocated_frames`/`free_frames`/`refcounts` fields) plus the
small allocation/watermark extensions in §2.1.

### 2.1 Derived spec helpers (new `impl FrameAllocView` block — existing `struct`/`wf` untouched)

```rust
impl FrameAllocView {
    /// Number of frames currently free, i.e. available to hand out. This is the
    /// single quantity the kernel watermark reads. Models `frame::free_count()`.
    pub open spec fn free_count(self) -> nat {
        self.free_frames.len()
    }

    /// A user allocation of `count` frames is admissible: fulfilling it would
    /// still leave at least `KERNEL_WATERMARK` frames free for the kernel.
    /// This is the predicate behind the user-vs-kernel asymmetry — the gate
    /// `check_user_watermark` enforces. Reads the build-time constant directly
    /// so callers need not thread the threshold through.
    pub open spec fn user_alloc_ok(self, count: nat) -> bool {
        self.free_count() >= count + (config::kernel::KERNEL_WATERMARK as nat)
    }

    /// Allocate a single currently-free frame `addr`: move it free -> allocated
    /// with refcount 1. (== `book_all(set![addr])`; spelled out for readability.)
    pub open spec fn alloc_one(self, addr: int) -> FrameAllocView {
        FrameAllocView {
            allocated_frames: self.allocated_frames.insert(addr),
            free_frames: self.free_frames.remove(addr),
            refcounts: self.refcounts.insert(addr, 1),
        }
    }
}
```

The set-at-once transition already exists as the do-not-modify
`FrameAllocView::book_all(set)` — reused verbatim for the bulk paths.
Contiguity reuses the do-not-modify `region_frame_addrs(base, size)` (the set of
page-aligned frame addresses of `[base, base+size)`).

---

## 3. Well-formedness Invariant `inv()`

```rust
impl PhysMemoryManager {
    pub open spec fn inv(&self) -> bool {
        // The brokered frame partition is well formed: free/allocated disjoint,
        // page-aligned, refcount<->allocated consistent, refcounts in 1..=255.
        self@.wf()
    }
}
```

`inv()` is just `self@.wf()` and nothing more, on purpose:

- **Liveness is structural.** Possessing `&mut self` (only obtainable from
  `get_mut()` after `init` succeeded) already witnesses `manager_ready`; there is
  no separate "is-up" bit for these methods to carry. The cross-layer liveness
  fact lives in `PhysModView` (`manager_ready ==> initialized`) and is the
  business of `phys::init`, not of the per-allocation contracts.
- **The watermark is NOT an invariant.** "≥ `KERNEL_WATERMARK` frames free" holds
  only *after a user allocation*; kernel allocations are designed to dip below
  it. So it is a per-user-alloc **gate** (`user_alloc_ok`), never a type
  invariant. Putting it in `inv()` would be a false spec that kernel allocation
  legitimately violates.

---

## 4. Spec transitions of the target functions

State is `v = old(self)@`, `v' = self@` (both `FrameAllocView`). `ps` =
`spec_page_size()`, `W` = `KERNEL_WATERMARK`. Every clause is stated only over
`FrameAllocView`; nothing names `upool`, the singleton storage, or the threshold
value beyond `user_alloc_ok`.

> Frame-handle addresses: a `UserFrame`/`KernelFrame` exposes its physical frame
> address. Below this is written `f.addr()` — a thin spec accessor
> (`address()@` as `int`) to be added in the spec phase; the View commits only
> to *which address* each returned handle owns, not to the handle's storage.

### 4.1 `alloc_kernel_frame(&mut self) -> Result<KernelFrame, Error>`

Kernel single frame; **no watermark**.

```
requires  self.inv()
ensures   self.inv()
          match result {
            Ok(kf) => {
              // some previously-free frame is now reserved and owned by kf
              &&& v.free_frames.contains(kf.addr())
              &&& v' == v.alloc_one(kf.addr())
              // ⇒ v'.reserved(kf.addr())  and  v'.free_count() == v.free_count() - 1
            }
            Err(_) => {
              // nothing allocated, nothing leaked: partition unchanged
              &&& v' == v
              // failure ⇔ no frame was available at all (kernel never self-limits)
              &&& v.free_count() == 0
            }
          }
```

`v.free_count() > 0 ==> result is Ok` is the contrapositive liveness fact a
caller relies on (kernel allocation succeeds whenever any frame is free).

### 4.2 `alloc_user_frame(&mut self) -> Result<UserFrame, Error>`

User single frame; **watermark-gated**, identical gate to the bulk path at
`count == 1`. Fast path, no intermediate `Vec`.

```
requires  self.inv()
ensures   self.inv()
          match result {
            Ok(uf) => {
              &&& v.free_frames.contains(uf.addr())
              &&& v' == v.alloc_one(uf.addr())
              &&& v.user_alloc_ok(1)              // the gate held before allocating
            }
            Err(_) => {
              &&& v' == v                          // unchanged, no leak
              &&& !v.user_alloc_ok(1)              // rejected ⇔ watermark would break
            }
          }
```

Bidirectional gate: `result is Ok <==> v.user_alloc_ok(1)` (modulo pool
availability, which `user_alloc_ok` already implies via `free_count`). This is
exactly the CoW caller's mental model: `OutOfMemory` iff the watermark blocks it.

### 4.3 `check_user_watermark(count: usize) -> Result<(), Error>` (private gate)

No external callers, no state change; it *is* the predicate `user_alloc_ok`.

```
ensures   v' == v                                  // pure check, no mutation
          result is Ok  <==>  v.user_alloc_ok(count as nat)
```

This single clause is what `alloc_user_frame` / `alloc_many_user_frames` reuse to
discharge their gate, so the two user paths provably enforce the watermark
*identically*.

### 4.4 `alloc_many_user_frames(&mut self, count, frames: &mut Vec<UserFrame>) -> Result<(), Error>`

Bulk user; **watermark-gated**, **not** required contiguous. Caller supplies an
empty vector with capacity ≥ `count`.

```
requires  self.inv()
          old(frames)@.len() == 0                  // caller-supplied storage contract
                                                   // (capacity >= count is an exec
                                                   //  precondition; runtime check on
                                                   //  emptiness is a removable safeguard)
ensures   self.inv()
          match result {
            Ok(()) => {
              &&& v.user_alloc_ok(count as nat)     // gate held
              // exactly `count` distinct frames, all free in v, now reserved
              &&& frames@.len() == count
              &&& let S = Set::new(|a| exists|i| 0 <= i < count && frames@[i].addr() == a);
              &&& S.len() == count
              &&& v.all_free(S)
              &&& v' == v.book_all(S)               // ⇒ v'.all_reserved(S)
            }
            Err(_) => {
              &&& v' == v                            // all-or-nothing: unchanged
              &&& frames@.len() == 0                 // vec emptied, no leak
              &&& !v.user_alloc_ok(count as nat)     // rejected ⇔ watermark would break
            }
          }
```

The returned frames are modeled as a **set** `S` (order not caller-observable for
user frames). `v' == v.book_all(S)` is the whole effect; `all_reserved(S)`
follows.

### 4.5 `alloc_many_kernel_frames(&mut self, count, frames: &mut Vec<KernelFrame>) -> Result<(), Error>`

Bulk kernel; **physically contiguous**, **no watermark**. Empty vector supplied.

```
requires  self.inv()
          old(frames)@.len() == 0
ensures   self.inv()
          match result {
            Ok(()) => {
              &&& frames@.len() == count
              // contiguous, in order: handle i owns base + i*ps
              &&& exists|base: int| {
                    &&& base % ps == 0
                    &&& forall|i: int| 0 <= i < count ==> frames@[i].addr() == base + i * ps
                    &&& let S = region_frame_addrs(base, count * ps);
                    &&& v.all_free(S)
                    &&& v' == v.book_all(S)          // ⇒ v'.all_reserved(S)
                  }
            }
            Err(_) => {
              &&& v' == v                            // all-or-nothing: unchanged
              &&& frames@.len() == 0                 // vec emptied, no leak
            }
          }
```

Contiguity is surfaced as `frames@[i].addr() == base + i*ps` (the identity-map
requirement the caller depends on); the set effect reuses `region_frame_addrs` +
`book_all`. The `Err` arm makes no watermark claim — kernel bulk fails only when
no contiguous run of `count` free frames exists, which is an internal
availability fact the caller maps straight to `?`.

### 4.6 `init(upool: Upool) -> Result<(), Error>`  (`pub(super)`, pre-existing `external_body`)

Static one-shot bring-up of the manager singleton; takes no `self` and returns no
handle. Its only abstract effect is *the manager layer becomes live over the
already-seeded frame partition* — i.e. it flips `PhysModView.manager_ready`,
which is precisely the fact `phys::init` ensures via `lemma_manager_ready()`. The
manager-View `inv()` (`self@.wf()`) then holds for every `&mut self` later
obtained from `get_mut()`.

```
// Effect expressed at the subsystem level (PhysModView), where init's caller reads it:
ensures   match result {
            Ok(())  => phys_view().manager_ready
                       && phys_view().frames == old(phys_view()).frames,  // frames untouched
            Err(e)  => e == Error::InvalidArgument                        // double-init only
                       && phys_view() == old(phys_view()),
          }
```

`init` reserves no frames; it only marks the layer ready, so the frame partition
is unchanged on both arms. (It remains `external_body` as inherited — not added
by this work — so this is the contract its callers rely on, discharged by the
`phys::init` lemma rather than by a verified body.)

---

## 5. Substitution test (per field / per helper)

> *"If the manager / frame allocator were rewritten with a different algorithm
> (buddy allocator, free-list, different singleton mechanism, different pool),
> would this still make sense?"*

| Element | Survives? | Reasoning |
|---------|-----------|-----------|
| `self@ : FrameAllocView` (the whole View) | ✅ | The allocated/free/refcount partition is what *any* physical-frame broker maintains; it is the existing `View for Inner`. A buddy/free-list rewrite still exposes the same `Set<int>` partition. Names no `upool`, bitmap, or refcount slice. |
| `free_count` | ✅ | "How many frames are free" is observable regardless of representation; the watermark is defined in terms of it, not of any counter field. |
| `user_alloc_ok` | ✅ | The user-vs-kernel asymmetry ("keep `KERNEL_WATERMARK` free for the kernel") is a policy any implementation of this manager must honor; phrased purely over `free_count`. |
| `alloc_one` / reuse of `book_all` | ✅ | "Move a free frame to allocated with refcount 1" is the algorithm-independent meaning of an allocation; identical to the frame-level booking vocabulary already shared with `frame`. |
| contiguity via `region_frame_addrs` + `base + i*ps` | ✅ | The *caller-required* property is "these frames are physically contiguous", independent of how the run is found (`alloc_contiguous`, buddy, etc.). |

No element commits to a mechanism. The implementation's `Upool`, `MaybeUninit`,
`AtomicBool`, `Bitmap`, and `[u8; N]` refcount slice are *one* realization; the
View commits to none of them.

---

## 6. Design rationale

- **Reuse the frame partition; introduce no new struct.** The six functions'
  sole caller-observable effect is on the global frame partition, which
  `FrameAllocView` already abstracts (and which `frame`'s verified `View for
  Inner` and `PhysModView.frames` already use). Setting `type V = FrameAllocView`
  keeps the manager specs interoperable with the `frame`-level contracts they
  call and honors the do-not-modify constraint. A wrapper struct would duplicate
  state and add indirection for no caller-visible gain.
- **`&mut self` gives the transition directly.** Unlike phys-mod's free
  functions, these are methods, so `old(self)@ -> self@` *is* the v->v' pair; no
  per-call global-token ceremony is needed in the contracts (only the closed
  `view()` mapping is pinned in proof — §8).
- **Watermark = a gate predicate, not an invariant.** `user_alloc_ok`
  (`free_count >= count + W`) is the one predicate distinguishing user from
  kernel allocation. It appears as a precondition-equivalent in the user paths
  and is deliberately absent from `inv()` because kernel allocation is *meant*
  to breach it. `check_user_watermark`'s contract is exactly this predicate, so
  both user paths provably share one gate.
- **Allocation expressed as set-booking.** `book_all(S)` (existing) /
  `alloc_one(addr)` describe the entire effect declaratively; `all_reserved`,
  `free_count` decrements, and "booked ⇒ never re-handed-out" all follow, so each
  ensures is a clause the caller drops straight into a proof.
- **Contiguity surfaced only where required.** Kernel bulk states
  `frames@[i].addr() == base + i*ps`; user bulk states only a set. This mirrors
  the analysis: contiguity is a kernel-stack requirement, explicitly *not* a user
  property — modeling user frames as ordered would over-specify.
- **Honest, symmetric error specs.** Every fallible path states the `Err` frame
  condition (`v' == v`, vec emptied) — the "no-leak / all-or-nothing" guarantee —
  and, for the watermark paths, the bidirectional gate (`Err ⇔ !user_alloc_ok`).
  Kernel paths fail only on genuine unavailability (`free_count == 0` / no
  contiguous run), stated as an interface predicate rather than a list of code
  checks.

---

## 7. Rejected alternatives

1. **A new wrapper `PhysManagerView { frames: FrameAllocView }`.** Rejected. It
   carries exactly one field already named by `FrameAllocView`; the wrapper adds
   indirection and a second name for the same partition with no caller-visible
   information. `type V = FrameAllocView` is the minimal honest model. (Contrast
   phys-mod, where the wrapper *earned its keep* by adding two liveness bits.)

2. **Mirror the implementation: fields `upool: UpoolView`, `init_flag: bool`.**
   Rejected. `upool` is internal bookkeeping over the *same* frames the partition
   already models; exposing it leaks the pool mechanism, fails the substitution
   test (a free-list rewrite has no `Upool`), and duplicates state. The init flag
   is structurally implied by holding `&mut self`.

3. **Put the watermark (`free_count >= W`) into `inv()`.** Rejected as a false
   spec: kernel allocations deliberately drop free frames below `W`, so it is not
   an invariant. It belongs as the per-user-alloc gate `user_alloc_ok`.

4. **Add a separate `free_count: nat` field to the View.** Rejected.
   `free_frames.len()` already determines it; a separate field would create a
   consistency obligation (`free_count == free_frames.len()`) for zero gain. Keep
   it derived.

5. **Model returned user frames as an ordered `Seq` (like kernel frames).**
   Rejected. The analysis states user frames are mapped individually and their
   order is not caller-observable; a `Set` is the right abstraction and avoids
   over-specifying. Only kernel bulk needs order (contiguity).

6. **Spec the manager over `phys_view()` (the free accessor) instead of
   `self@`.** Rejected for these functions. They are `&mut self`, so `self@` /
   `old(self)@` is the natural and Verus-native transition handle; routing
   through the global accessor would forfeit that and force a token argument into
   every contract. The relationship `self@ == phys_view().frames` is recorded as
   a proof-phase attachment invariant (§8), not pushed into the contracts.

7. **Promise full state preservation on *every* error including a hypothetical
   mid-bulk failure.** Accepted here (`v' == v`) *because* the bulk paths are
   genuinely all-or-nothing (watermark checked up front; on any later per-frame
   failure the vec is cleared, freeing everything). This is sound — unlike
   phys-mod's booking, which is non-transactional and could only promise `inv()`.

---

## 8. Global-state modeling note (deferred to proof phase)

`view()` is `closed` because the manager brokers the **global** frame allocator
(`frame::INSTANCE`), not state physically inside the `PhysMemoryManager` struct
(whose only field, `upool`, is bookkeeping over those same frames). The proof
phase pins the closed mapping via the same ghost-token attachment `PhysModView`
uses, with the cross-View invariant:

- `self@`            ↔ `frame::INSTANCE@` (the verified `View for Inner`), and
- `self@ == phys_view().frames` whenever `phys_view().manager_ready`.

`free_count`, `user_alloc_ok`, `alloc_one`, and the contiguity helper are open
and computed purely from that `FrameAllocView`, so all six contracts in §4 are
the fixed target the token machinery must satisfy and do not change. (Same
pattern the bump-allocator and phys-mod Views used to defer their global-token
attachments.)
