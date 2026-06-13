# View Design: `mm::phys::upool` (`UserFrame`, `Upool`)

> Phase output. Designs the abstract `View`s that all later specs
> (`requires`/`ensures`) for the eight in-scope functions will reference:
> `UserFrame::new`, `UserFrame::address`, `UserFrame::leak`,
> `UserFrame::share`, `UserFrame::refcount`, `UserFrame::drop`,
> `Upool::new`, `Upool::alloc`.
>
> Built **only** from `caller_analysis.md` and the body-removed public API
> (`body_removed_source.rs`), plus the **existing, do-not-modify** spec
> definitions (`byte_at_address`, `FrameAllocView`, `FrameAllocView::wf`,
> `Inner::inv`, `frame_addr_of`, `View for Inner`, `Inner::internal_inv`) and
> the already-committed subsystem View `PhysModView` / `phys_view()` and the
> frame-set / allocation vocabulary on `FrameAllocView`
> (`covers`, `reserved`, `book_all`, `free_count`, `alloc_one`, …).

---

## 1. Abstract Resource (from caller analysis)

The module exposes **user-space physical frames** drawn from the global frame
allocator under two types:

- **`UserFrame`** — a caller's RAII proof of ownership of **one reference to one
  physical frame**. Every caller (CoW fault resolution, fork sharing, ELF
  segment loading, page-table mapping) reasons about exactly one thing: *which
  physical frame this handle owns*. They feed `address()@` to `page_table.map`
  and `memcpy`, decrement a refcount by dropping it, suppress the decrement with
  `leak`/`ManuallyDrop`, add a reference with `share`, and read the count with
  `refcount`. They never observe the handle's storage.

- **`Upool`** — a thin, watermark-agnostic, single-frame **allocation facade**
  over the pool's free partition. It has no real state of its own
  (`_private: ()`); its backing store *is* the global frame allocator. Its only
  caller is boot-time `mm::phys` init (`Upool::new`, once) and the manager's
  user-allocation paths (`Upool::alloc`).

The only abstract state these functions observe or mutate is therefore the
**frame partition**: which physical frames are allocated vs. free, with per-frame
reference counts. That partition is **already** modeled by the do-not-modify
`FrameAllocView` (== the verified `View for Inner` of the global
`frame::INSTANCE`, == `PhysModView.frames`). The refcount-affecting operations
(`share`, `drop`) act on that *global* partition — not on bytes inside the
`UserFrame` struct, which holds only an address.

Callers explicitly **do not** observe: how the address is stored in a
`UserFrame`, the pool's `_private` field, the `frame::share`/`frame::free`
mechanism, or the watermark threshold. They observe only: *which frame each
handle owns*, *the per-frame reference-count discipline*, and the *conservation
/ no-leak* behavior of allocation.

---

## 2. The Views — keep the two existing ones; add no new struct

Both Views already exist in `body_removed_source.rs` and are the minimal honest
models. This design **adopts and justifies them unchanged**; it adds only
derived spec helpers (§2.1).

### 2.1 `UserFrame` — abstract value is the owned frame address (`int`)

```rust
impl View for UserFrame {
    type V = int;

    // Closed: a handle abstractly *is* the physical address of the frame it owns.
    closed spec fn view(&self) -> int {
        self.addr@
    }
}
```

`UserFrame@ : int` is exactly what every caller reasons about (*which frame*).
Crucially it is **not** a pair `(addr, refcount)` and **not** a struct: the
reference count is a property of the *shared frame in the global partition*,
readable through `phys_view().frames.refcounts[self@]`, not of the handle. `new`
fabricates handles **without** bumping any count (the probe / take-to-free /
re-wrap idioms), so a per-handle "owned refcount" would be unsound and would
double-count (see §7.1, §7.2).

### 2.2 `Upool` — abstract value is the frame partition it draws from (uninterp)

```rust
impl View for Upool {
    type V = FrameAllocView;

    // Uninterp: the pool has no real state; its store is the global allocator.
    uninterp spec fn view(&self) -> FrameAllocView;
}
```

`Upool@ : FrameAllocView`, uninterpreted because the concrete pool struct
(`_private: ()`) carries no state — the trust that `self@` tracks the global
partition is discharged by `Upool` being `external_body` (per
`verus-ai-logs/tcb-allowed.md`) and pinned in proof exactly as the manager's
`self@ == phys_view().frames` attachment is.

### 2.3 Derived spec helpers (new `impl FrameAllocView` block; struct/`wf` untouched)

The refcount duality of `share` / `drop` is named once, mirroring the
do-not-modify `Inner::share` / `Inner::free` transitions verbatim so the upool
contracts and the frame-level contracts speak the same vocabulary:

```rust
impl FrameAllocView {
    /// Add one reference to the already-allocated frame `addr`: bump its refcount
    /// by one, leaving the allocated/free partition unchanged. Models the effect
    /// of `frame::share` (== `Inner::share`).
    pub open spec fn add_ref(self, addr: int) -> FrameAllocView {
        FrameAllocView {
            allocated_frames: self.allocated_frames,
            free_frames: self.free_frames,
            refcounts: self.refcounts.insert(addr, self.refcounts[addr] + 1),
        }
    }

    /// Release one reference to the allocated frame `addr`. If it was the last
    /// reference (refcount == 1) the frame moves free; otherwise the refcount is
    /// decremented. Models the effect of `frame::free` (== `Inner::free`).
    pub open spec fn release(self, addr: int) -> FrameAllocView {
        if self.refcounts[addr] == 1 {
            FrameAllocView {
                allocated_frames: self.allocated_frames.remove(addr),
                free_frames: self.free_frames.insert(addr),
                refcounts: self.refcounts.remove(addr),
            }
        } else {
            FrameAllocView {
                allocated_frames: self.allocated_frames,
                free_frames: self.free_frames,
                refcounts: self.refcounts.insert(addr, self.refcounts[addr] - 1),
            }
        }
    }
}
```

The single-frame **allocation** transition (`alloc_one`) and `free_count`
already exist (added with the manager View) and are reused verbatim by
`Upool::alloc`.

---

## 3. Well-formedness Invariants `inv()`

### 3.1 `UserFrame::inv()` — page-aligned address only

```rust
impl UserFrame {
    /// The handle's address is a valid, page-aligned physical frame address.
    /// This is exactly `self.addr.inv()`; surfaced so the refcount-affecting
    /// methods can discharge the `frame.inv()` precondition of the frame layer.
    pub open spec fn inv(&self) -> bool {
        self.addr.inv()        // ==> self@ % spec_page_size() == 0
    }
}
```

`inv()` carries **only** structural validity of the address, and deliberately
**no ownership claim** (no `phys_view().frames.allocated_frames.contains(self@)`,
no `refcounts[self@] >= 1`). Ownership is *not* a handle invariant because:

- **`new` does not establish it.** The probe idiom wraps an arbitrary
  PTE-recovered address in `ManuallyDrop` purely to call `refcount`/`share`;
  forcing `new` to prove the frame is allocated would reject that legitimate
  pattern and is impossible from the address alone.
- **Aliasing would double-count.** If each live `UserFrame` invariantly "owned"
  a `>= 1` refcount, two handles to the same frame (the normal `share` result)
  plus the partition's single counter would be mutually inconsistent. Ownership
  is therefore a per-operation *transition* fact (§4), not a type invariant.

### 3.2 `Upool::inv()` — partition well-formed

```rust
impl Upool {
    pub open spec fn inv(&self) -> bool {
        self@.wf()             // free/allocated disjoint, page-aligned, refcounts 1..=255
    }
}
```

`inv()` is just `self@.wf()`: liveness is structural (a `Upool` exists only after
boot constructed it) and the watermark is the manager's per-allocation gate, not
a pool invariant. This matches the existing `Upool::alloc` contract, which is
stated over `self@.wf()`.

---

## 4. Spec transitions of the target functions

Notation: for the **global** frame partition, `F = old(phys_view()).frames` and
`F' = phys_view().frames`; `ps = spec_page_size()`. Refcount-affecting
`UserFrame` methods speak over `phys_view()` (the global accessor) because the
handle holds no partition — the same free-function modeling phys-mod uses.
Pure-address methods are stated over `self@` directly.

### 4.1 `UserFrame::new(addr: FrameAddress) -> Self`

Thin owning wrapper: no allocation, no refcount change, infallible.

```
requires  addr.inv()
ensures   result@ == addr@                    // owns exactly the given frame
          result.inv()
          phys_view() == old(phys_view())     // no global effect (no alloc, no bump)
```

### 4.2 `UserFrame::address(&self) -> FrameAddress`

Pure, repeatable read of the owned address.

```
requires  self.inv()
ensures   result@ == self@
          result.inv()
          phys_view() == old(phys_view())     // no side effect
```

### 4.3 `UserFrame::leak(self) -> FrameAddress`

Consumes the handle, returning its address **without** releasing the frame.

```
requires  self.inv()
ensures   result@ == self@
          result.inv()
          phys_view() == old(phys_view())     // Drop suppressed: refcount NOT decremented
```

The whole point of `leak` is the *absence* of a `release`: the partition is
unchanged, so the frame survives for whoever now records the address (the page
table). A regression that released here would double-free at teardown.

### 4.4 `UserFrame::share(&self) -> Result<UserFrame, Error>`

Adds one reference to the owned frame and returns a fresh aliasing handle.

```
requires  self.inv()
ensures   match result {
            Ok(uf) => {
              &&& uf@ == self@                          // same physical frame
              &&& uf.inv()
              &&& F.allocated_frames.contains(self@)    // frame was owned/allocated
              &&& F.refcounts[self@] < 255              // had headroom to bump
              &&& F'  == F.add_ref(self@)               // refcount += 1, partition else equal
              // only the frame partition changes; liveness bits untouched:
              &&& phys_view().initialized  == old(phys_view()).initialized
              &&& phys_view().manager_ready == old(phys_view()).manager_ready
            }
            Err(_) => {
              &&& phys_view() == old(phys_view())        // self + frame unchanged
              &&& (!F.allocated_frames.contains(self@)   // failure cause:
                   || F.refcounts[self@] >= 255)         //   unallocated or saturated
            }
          }
```

`uf@ == self@` (alias the *same* frame) and the `add_ref` bump are exactly the
fork/CoW caller's contract; the `Err`-arm frame condition (`self` untouched) is
what lets the parent — held in `ManuallyDrop` — survive a `?` early return
without a double-decrement.

### 4.5 `UserFrame::refcount(&self) -> Result<u8, Error>`

Pure read of the owned frame's current reference count.

```
requires  self.inv()
ensures   phys_view() == old(phys_view())            // no mutation, no free
          match result {
            Ok(c)  => {
              &&& F.allocated_frames.contains(self@)
              &&& c as int == F.refcounts[self@]
            }
            Err(_) => !F.allocated_frames.contains(self@)
          }
```

This is the `== 1` last-reference probe the CoW resolver depends on; `&self` +
the `phys_view()` frame condition make "does not consume, does not free"
explicit.

### 4.6 `UserFrame::drop(&mut self)` (`impl Drop`)

Releases exactly one reference to the owned frame; reclaims the frame on the last
reference. Errors are logged, not propagated (drop cannot fail).

```
ensures   F' == F.release(self@)                       // target: one reference released
          // best-effort caveat (see §8): if the underlying free fails, it is
          // logged and the partition is left consistent (the frame is leaked,
          // not corrupted). The committed *design* semantics is `release`.
```

`release(self@)` is the central RAII guarantee callers rely on for automatic
cleanup on error paths — and is precisely what `leak`/`ManuallyDrop` suppress.
The `share`/`drop` pair is `add_ref` / `release`: a frame survives until the
reference added by `share` is matched by a `drop`.

### 4.7 `Upool::new() -> Self`  *(pub(super))*

Boot-time construction of the pool facade, once, infallibly.

```
ensures   result@.wf()             // pool ready; subsequent alloc precondition met
```

The pool introduces no frames of its own — `wf()` is the only fact its single
caller (`mm::phys` init) needs before handing the pool to `PhysMemoryManager`.

### 4.8 `Upool::alloc(&mut self) -> Result<UserFrame, Error>`  *(pre-existing, `external_body`)*

Single user-frame allocation over the pool's free partition. Contract already
committed (in `body_removed_source.rs`); reproduced as the fixed target:

```
requires  old(self)@.wf()
ensures   final(self)@.wf()
          match result {
            Ok(uf) => {
              &&& old(self)@.free_frames.contains(uf@)   // came from the free partition
              &&& final(self)@ == old(self)@.alloc_one(uf@)   // free -> allocated, refcount 1
            }
            Err(_) => {
              &&& final(self)@ == old(self)@             // pool unchanged on failure
              &&& old(self)@.free_count() == 0           // failed only when exhausted
            }
          }
```

Conserves frames (`alloc_one`), and the `Err` arm's `free_count() == 0` +
unchanged-pool facts are what the manager's bulk-failure restore logic relies on.

---

## 5. Substitution test (per element)

> *"If `UserFrame` / `Upool` / the frame allocator were rewritten with a
> different algorithm (buddy allocator, free-list, fat pointer, different pool),
> would this still make sense?"*

| Element | Survives? | Reasoning |
|---------|-----------|-----------|
| `UserFrame@ : int` (owned address) | ✅ | "Which physical frame does this RAII handle own" is observable for *any* representation. Names no field, no refcount slot. A fat-pointer or index-based handle still owns one address. |
| `UserFrame::inv() == addr.inv()` (page-aligned) | ✅ | Every physical frame address is page-aligned regardless of allocator; purely structural. |
| `Upool@ : FrameAllocView` (uninterp) | ✅ | The pool is a facade over the frame partition; a free-list/buddy rewrite still hands out frames from the same allocated/free partition. Names no `_private`, no bitmap. |
| `add_ref` / `release` | ✅ | "Bump a shared frame's refcount" and "drop a reference, freeing on the last" are the algorithm-independent meaning of CoW sharing and RAII release — identical to the frame-level `Inner::share`/`Inner::free` shapes. |
| `new` = pure wrap (no partition effect) | ✅ | Constructing a handle from a known address bumps nothing under any scheme. |
| `leak` = suppress `release` | ✅ | "Transfer ownership without releasing" is representation-independent; the *absence* of a partition change is the contract. |
| `alloc_one` / `free_count` reuse (alloc) | ✅ | "Move one free frame to allocated with refcount 1" and "how many frames are free" survive any allocator rewrite. |

No element commits to a mechanism. The implementation's `addr: FrameAddress`
field, `_private: ()`, `frame::share`/`frame::free`, bitmap, and refcount slice
are *one* realization; the Views commit to none of them.

---

## 6. Design rationale

- **Two minimal Views, no new struct.** `UserFrame@ = int` and
  `Upool@ = FrameAllocView` (both already committed) are the smallest honest
  models: a handle's caller-visible value is the frame it owns; a stateless pool
  facade's value is the partition it draws from. Wrapping either in a one-field
  struct would add indirection and zero information.
- **Refcount lives in the partition, not the handle.** `UserFrame` holds only an
  address; `share`/`drop`/`refcount` therefore state their effect over the
  *global* partition `phys_view().frames` (via `add_ref`/`release`/read), exactly
  the free-function global-state pattern phys-mod uses. This is why the View is
  `int`, not `(addr, count)`.
- **`share`/`drop` are `add_ref`/`release` — one named duality.** Mirroring the
  do-not-modify `Inner::share`/`Inner::free` transitions keeps the upool contracts
  legible and interoperable with the frame layer they delegate to, and makes the
  "survives until both handles drop" invariant a direct consequence.
- **`leak` is the *absence* of `release`.** Its contract is `phys_view()`
  unchanged — the no-double-free guarantee — distinct from `drop`'s `release`.
- **Ownership is a transition fact, not an `inv()`.** `new`'s probe/take/re-wrap
  idioms forbid a per-handle ownership invariant (§3.1, §7.2); the allocation
  state appears only in the `Ok`/`Err` arms of `share`/`refcount`/`drop`.
- **Honest, symmetric error specs.** `share`'s `Err` arm pins `self`+frame
  unchanged and the failure cause (unallocated or saturated), so a `?` on a
  `ManuallyDrop` parent cannot double-decrement; `Upool::alloc`'s `Err` arm pins
  pool-unchanged + exhaustion.

---

## 7. Rejected alternatives

1. **Encode the refcount in `UserFrame@` (e.g. `type V = (int, nat)` or a
   struct `{addr, refcount}`).** Rejected. The reference count is a property of
   the *shared frame in the global partition*, not of the handle: `new` makes
   handles without bumping it, and `share` produces two handles to *one* counter.
   A per-handle count would double-count and contradict the partition. `int`
   (address) is the caller's actual mental model; the count is read through
   `phys_view().frames.refcounts[self@]`.

2. **Make `UserFrame::inv()` assert ownership
   (`allocated_frames.contains(self@) && refcounts[self@] >= 1`).** Rejected.
   The probe idiom (`ManuallyDrop::new(UserFrame::new(pte_addr))`) and the
   take-to-free idiom construct handles whose frame state the *caller* knows, not
   `new`; forcing the invariant would reject these patterns and is unprovable
   from an address alone. Ownership belongs in the per-operation transitions.

3. **Give `Upool` an interpreted view of its `_private: ()` state.** Rejected.
   The pool genuinely has no state — its store is the global allocator. `uninterp`
   + `external_body` (per `tcb-allowed.md`) is the correct, honest model; an
   interpreted view would invent fictional state that fails the substitution test.

4. **A new wrapper struct (`UserFrameView { addr: int }` / `UpoolView { frames }`).**
   Rejected. Each carries exactly one field already named by `int` /
   `FrameAllocView`; the wrapper adds indirection and a second name for the same
   value with no caller-visible gain.

5. **State `drop` with no partition effect (just consume `self`).** Rejected.
   `drop`'s defining service is releasing one reference (auto-cleanup on error
   paths); dropping that from the contract would make the `share`/`drop` refcount
   discipline — the whole reason CoW/fork is safe — unverifiable. (Conversely,
   `leak`'s no-effect contract is *correct*, because suppressing release is its
   purpose.)

6. **Inline `FrameAllocView { … }` transitions in every contract instead of
   `add_ref`/`release` helpers.** Rejected as a readability/maintenance choice:
   the `share` and `drop` shapes are the `Inner::share`/`Inner::free` transitions
   verbatim; naming them once (like the existing `alloc_one`) makes the duality
   explicit and the contracts terse, matching the established `FrameAllocView`
   vocabulary.

---

## 8. Global-state & dependency modeling note (deferred to proof phase)

`UserFrame`'s view is `closed` and the refcount-affecting methods name
`phys_view().frames`, not bytes inside the struct, because the reference counts
they mutate live in the **global** frame allocator (`frame::INSTANCE@`, the
verified `View for Inner`, == `phys_view().frames`). The proof phase pins these
exactly as phys-mod/manager already do:

- `phys_view()`'s cross-call transition (`old(phys_view()) -> phys_view()`) is
  realized by the ghost token over the `frame::INSTANCE`/`PhysMemoryManager`
  singletons; `add_ref`/`release` are `open` and computed purely from
  `FrameAllocView`, so the §4 contracts are the fixed target that machinery must
  satisfy.
- `Upool@`'s attachment to that partition (`self@ == phys_view().frames`,
  uninterp + `external_body`) mirrors the manager's `self@ == phys_view().frames`.

Two dependency facts the proof phase must lift (not part of this View, but noted
so §4 is discharge-able):

- **`UserFrame::share`/`refcount`** delegate to `frame::share`/`frame::refcount`
  (free-function layer). `Inner::share`/`Inner::refcount` already carry the exact
  `add_ref` / read transitions; the free-function wrappers must expose them
  (currently thin, unspecified) to discharge §4.4/§4.5.
- **`UserFrame::drop`** delegates to `frame::free`, whose free-function wrapper is
  presently best-effort (`ensures true`). The `release` transition in §4.6 is the
  committed *design* semantics; proving it requires lifting `Inner::free`'s
  transition through `frame::free`. Until then `drop`'s provable postcondition is
  the best-effort disjunction (release-or-logged-leak), with `release` as the
  target. This is the only place a frame-layer spec gap, not a View choice,
  limits strength.
