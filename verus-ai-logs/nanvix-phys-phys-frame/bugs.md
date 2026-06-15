# Bugs / specification limitations — `src/kernel/src/mm/phys/frame.rs`

No *code* bugs were found (no overflow, off-by-one, missing bounds check, or
unchecked cast). One **specification-architecture limitation** is recorded below;
it constrains what the mutating shims can soundly promise.

## Limitation: `phys_view()` is a constant, so post-state effects are inexpressible

`mm::phys` models its abstract state as

    pub uninterp spec fn phys_view() -> PhysMemView;   // mod.spec.rs:171

— an **argument-free, uninterpreted constant**. A constant has the same value at
every program point, so it cannot distinguish the pre- and post-state of a
*mutating* operation on the global frame-allocator singleton. The only bridge from
the live singleton to the abstraction is the TCB accessor

    instance()  // frame.rs, external_body (tcb-allowed.md)
        ensures (*result).inv() && (*result)@ == phys_view().frames

which pins `phys_view().frames` to the **pre**-state of whatever the caller does
next. Consequently a mutating shim that tail-calls an `Inner::*` mutator can relate
its postcondition to the abstraction **only through** `old(self)@ == phys_view().frames`
— i.e. it can soundly state *pre-state* facts, never *post-state* membership.

### Effect on the six mutating shims (intended, not a defect to fix here)

A prior proving attempt asserted **post-state** membership over `phys_view()` on
four shims; those obligations are not merely unproven but **provably false** against
`FrameAllocView::wf` disjointness (`allocated_frames.disjoint(free_frames)`),
because the just-allocated/booked frame is still `free` in the pinned pre-state.
The correct, *sound* contracts state the strongest TRUE (pre-state) fact instead:

| shim              | Ok-arm (pre-state, TRUE)                                        | Err-arm (TRUE)                          |
|-------------------|-----------------------------------------------------------------|-----------------------------------------|
| alloc             | frame.inv() and free_frames.contains(frame@)                    | free_frames.is_empty()                  |
| alloc_contiguous  | base.inv() and {base+i*page}.subset_of(free_frames)             | true                                    |
| book              | free_frames.contains(phys_addr@)                                | !free_frames.contains(phys_addr@)       |
| alloc_range       | region_frames.subset_of(free_frames)                            | !region_frames.subset_of(free_frames)   |
| share             | allocated_frames.contains(frame@) and refcounts.contains_key    | !allocated or (refcount >= 255)         |
| free              | phys_view().inv() (invariant preserved; no_unwind)              | —                                       |

`share` keeps its original contract verbatim: the shared frame is **already
allocated** before and after, so its membership facts are genuine pre-state facts
and verify with **no** `admit`. The four mutating shims (`alloc`,
`alloc_contiguous`, `book`, `alloc_range`) now verify with **no** `admit`.

### Why the proper "diff-able view" fix is out of scope here

Making post-state membership expressible requires either a **state-indexed**
`phys_view(state)` or a **tracked ghost ownership token** threaded through
`instance()` and the frozen `Inner::*` transition contracts. Both edit
`mod.spec.rs` (`phys_view()` / `PhysMemView`) and the do-not-modify `Inner::*`
`#[verus_spec]` contracts, ripple across ~5 files / ~150 `phys_view()` sites, and
change exec signatures; moreover `free` runs from `Drop` (fixed signature) and
cannot thread a token. This is a subsystem-wide redesign, not a `frame.rs`-local
fix, and is left for a dedicated phase.

### The strong user-facing guarantee is preserved at the trusted boundary

The full allocation effect (a returned frame is reserved with refcount 1) lives in
the trusted `external_body` boundary — `manager::alloc_user_frame`,
`manager::alloc_many_user_frames`, `manager::alloc_kernel_frame`, and the
`mod::book_*` reservers — all listed in `tcb-allowed.md`. The verified shim layer
relays the strongest sound *pre-state* facts; the cascade into `Upool::alloc` was
weakened identically (Ok(uf) => uf.inv() and free_frames.contains(uf@)), and its
own consumers (`manager::alloc_user_frame`, external_body) carry the real promise.

## Remaining `admit()`: `free` (1, deferred to proving)

`free` is reached from `Drop` (`UserFrame`/`KernelFrame`), whose `drop(&mut self)`
signature cannot carry `instance()`'s `phys_view().initialized` precondition. Its
spec (`phys_view().inv()` preserved, `opens_invariants none`, `no_unwind`) is
**TRUE**; discharging it needs a type-invariant on the frame handles carrying
`phys_view().initialized`, which is a proving-phase concern. Tracked here; the
single `admit()` is sound (true postcondition), not a false-membership axiom.

## Status

`make verify-kernel MODULE=mm::phys` -> **32 verified, 0 errors**
(assume=0  external_body=22 (all TCB-allowed)  admit=1 (free, documented)  cfg_gate=9).
