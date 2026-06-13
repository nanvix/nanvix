# Caller Analysis: `mm::phys::upool`

## Script Output
See: `verus-ai-logs/nanvix-phys-phys-upool/find_callers_lsp.out`
(rust-analyzer LSP, intra-crate only; crate `kernel` has no external dependents).

Summary: 8 exec functions, all `pub`/`pub(super)`/trait-pub, 0 private, 2 types
(`UserFrame`, `Upool`). One implicit caller: `impl Drop for UserFrame`.

All callers live in the same crate, concentrated in:
- `src/kernel/src/mm/virt/vmem.rs` (mapping + copy-on-write fault resolution)
- `src/kernel/src/mm/virt/manager.rs` (fork / frame sharing)
- `src/kernel/src/mm/phys/manager.rs` (bulk + single user-frame allocation)
- `src/kernel/src/mm/phys/mod.rs` (boot-time pool construction)
- `src/kernel/src/mm/elf.rs` (segment loading — holds `Vec<UserFrame>`)

## Trait Obligations
- Trait: `Drop` for `UserFrame` — `drop(&mut self)` must release the handle's
  reference to the underlying physical frame (`frame::free`), reclaiming the frame
  only when the last reference is dropped. Callers depend on this for automatic
  cleanup on error paths, and deliberately **suppress** it (via `ManuallyDrop` or
  `leak`) whenever ownership is transferred elsewhere (page table) or the handle is
  only a transient *probe*. Errors inside `drop` are logged, not propagated (drop
  cannot fail).
- Trait: `View for UserFrame` — abstract value is `int`, the physical frame address
  (`self.addr@`). Callers reason about *which* frame a handle owns, never about the
  handle's internal layout.

## Caller Expectations

### `UserFrame::new(addr) -> Self`
- Callers assume: produces an owning handle whose `view == addr@`; no allocation,
  no refcount change, infallible.
- Used three distinct ways:
  - **Probe** wrapped in `ManuallyDrop` to call `refcount()`/`share()` without ever
    freeing (`vmem.rs:869`, `virt/manager.rs:349`).
  - **Take-ownership-to-free**: `drop(UserFrame::new(old_frame))` to decrement the
    refcount of a frame whose address was recovered from a PTE (`vmem.rs:895`).
  - **Re-wrap** an unmapped frame address back into an owning handle returned to the
    caller (`vmem.rs:1522`).
- Callers don't care about: how the address is stored; that it's just a thin wrapper.
- Would break callers: if `new` allocated, bumped a refcount, or returned a handle
  whose `view != addr@`.

### `UserFrame::address(&self) -> FrameAddress`
- Callers assume: pure read of the owned frame's physical address, no consume, no
  side effect, repeatable. Result feeds `page_table.map(...)` and `memcpy`
  (`vmem.rs:361,882,887`).
- Callers don't care about: internal representation; only that the returned address
  equals the handle's `view`.

### `UserFrame::leak(self) -> FrameAddress`
- Callers assume: consumes the handle and returns its address **without** running
  `Drop`/`frame::free`, transferring ownership of the frame to whoever now records
  the address (the page table). Used right after a successful `map` (`vmem.rs:368`,
  `vmem.rs:891`).
- Critical invariant: after `leak`, no decrement of the frame's refcount occurs from
  this handle. A regression that freed on leak would double-free once the page table
  is later torn down.
- Callers don't care about: the returned address in the `_ = leak()` case — they only
  need the suppress-drop effect.

### `UserFrame::share(&self) -> Result<UserFrame, Error>`
- Callers assume (success): the underlying frame's refcount is incremented and a
  **new** owning handle aliasing the same physical frame (`result@ == self@`) is
  returned. The two handles independently own one reference each; the frame survives
  until both are dropped.
- Callers assume (error): no new reference acquired, `self` and its frame unchanged —
  so the parent's existing mapping/refcount is untouched (`virt/manager.rs:350`, fork
  CoW path; the parent is held in `ManuallyDrop` so a `?` early-return cannot
  double-decrement).
- Callers don't care about: the mechanism of the refcount bump (`frame::share`).
- Would break callers: returning a handle to a *different* frame, or succeeding
  without actually incrementing the refcount (would cause premature free).

### `UserFrame::refcount(&self) -> Result<u8, Error>`
- Callers assume: returns the current reference count of the owned frame; does **not**
  consume or mutate the handle (`&self`), and (the probe is in `ManuallyDrop`) does not
  free. Used to detect the `== 1` last-reference fast path in CoW resolution
  (`vmem.rs:870`).
- Callers don't care about: anything but the numeric count for the owned frame.

### `Upool::new() -> Self`  *(pub(super))*
- Caller: boot-time `mm::phys` init (`mod.rs:185`), exactly once, before handing the
  pool to `PhysMemoryManager::init`.
- Callers assume: produces a valid pool facade with `@.wf()` holding so subsequent
  `alloc` preconditions are met. No parameters, infallible.
- Callers don't care about: the pool has no real state (`_private: ()`); the actual
  backing store is the global frame allocator.

### `Upool::alloc(&mut self) -> Result<UserFrame, Error>`
- Caller: `PhysMemoryManager::alloc_user_frame` (single) and the loop in
  `alloc_many_user_frames` (`manager.rs:238,290`), after the kernel-watermark check.
- Callers assume (success): one frame is removed from the pool's free partition and
  returned as an owning `UserFrame`; per the existing spec
  `old@.free_frames.contains(uf@)` and `final@ == old@.alloc_one(uf@)`,
  with `@.wf()` preserved.
- Callers assume (error): pool unchanged (`final@ == old@`) and the pool was empty
  (`old@.free_count() == 0`). The manager relies on this to restore state on bulk
  failure (`frames.clear()` + `lemma_user_bulk_err_restored`).
- Callers don't care about: that it delegates to `frame::alloc`.

## Abstract Resource
The module manages **user-space physical frames** drawn from the global frame
allocator, exposing them as **reference-counted owning handles** (`UserFrame`) plus a
thin per-pool allocation entry point (`Upool`). A `UserFrame` is the caller's RAII
proof of ownership of one reference to one physical frame; `Upool` is the watermark-
agnostic single-frame allocation facade over the pool's free partition.

## Key Invariants (caller perspective)
- A live `UserFrame` owns exactly one reference to the frame at its `view` address;
  the frame is not reclaimed while any handle still references it.
- `Drop` releases exactly one reference; `leak` transfers ownership without releasing;
  `ManuallyDrop` wrapping is the idiom for read-only probing (`refcount`/`share`)
  without affecting the count.
- `share` is the only operation that adds a reference, and it preserves the address
  (`result@ == self@`); `new`, `address`, `refcount`, and `leak` never change a
  frame's refcount.
- `Upool::alloc` conserves frames: success moves one frame from free to allocated
  (`alloc_one`), failure leaves the pool exactly as it was and signals exhaustion.
- A frame address round-trips faithfully through `new`/`address`/`leak`
  (`UserFrame::new(a).address()@ == a@`, `leak()@ == a@`), so callers can hand a frame
  to a page table and later re-wrap it for freeing without aliasing the wrong frame.

## Pre-existing Specs (from upstream verification)
- Source: added during top-down verification of `mm::phys` (manager/init paths).
- Functions WITH specs: `Upool::alloc` (full `requires`/`ensures` on `@.wf()`,
  `alloc_one`, empty-pool `Err` arm). `Upool`, `Upool::new`, and `Upool::alloc` are
  `external_body` and listed in `verus-ai-logs/tcb-allowed.md` (opaque pool facade /
  pool allocation primitive, "verified when `upool` is").
- Functions WITHOUT specs: `UserFrame::new`, `UserFrame::address`, `UserFrame::leak`,
  `UserFrame::share`, `UserFrame::refcount`, `UserFrame::drop`.
- View type: exists for both `UserFrame` (`= int`, the frame address, `closed`) and
  `Upool` (`= FrameAllocView`, `uninterp`). Do not modify these or the listed
  spec/view defs (`byte_at_address`, `FrameAllocView`, `FrameAllocView::wf`,
  `Inner::inv`, `frame_addr_of`, `View for Inner`, `Inner::internal_inv`).

### Assessment
- Coverage: **partial** — only `Upool::alloc` carries a contract; all `UserFrame`
  methods and `Upool::new` are unspecified despite real callers depending on their
  ownership/refcount semantics.
- Strength: `Upool::alloc`'s ensures is adequate (covers both arms incl. error-path
  state preservation and exhaustion). `Upool` being `external_body` means its abstract
  state is uninterpreted — acceptable per the TCB list, but `UserFrame` is *not*
  `external_body`, so its refcount-affecting methods (`share`, `drop`, `leak`) need
  specs to capture the per-frame reference-count discipline that CoW/fork callers rely
  on.
- View design: caller-abstract and stable — `UserFrame@` exposes only the frame
  address (exactly what every caller reasons about), not handle internals.
