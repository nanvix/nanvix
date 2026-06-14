# Bugs — Nanvix phys manager

## Observations (record-only; validated in the proving phase)

### OBS-1: `alloc_many_kernel_frames` lacks the `count == 0` fast-path guard

`alloc_many_user_frames` early-returns `Ok(())` when `count == 0`, but
`alloc_many_kernel_frames` has no such guard and unconditionally calls
`frame::alloc_contiguous(count)`. `Inner::alloc_contiguous` (and therefore the
`frame::alloc_contiguous` wrapper) `requires count > 0`, so a `count == 0` call
would violate that precondition.

- **Spec decision**: added `requires count > 0` to
  `PhysMemoryManager::alloc_many_kernel_frames`, matching the contiguous
  allocator it delegates to. The sole caller (`mm::virt::manager::alloc_kpages`,
  not yet verified) must establish `count > 0`; this becomes an obligation when
  that module is verified.
- **Not auto-fixed**: adding a `count == 0` early return would change exec
  behavior of an unverified caller path. Recorded for the proving phase to
  decide whether to add the guard (code fix) or keep the precondition.

### OBS-2: `alloc_many_user_frames` distinctness depends on allocator non-aliasing

The Ok-arm contract now asserts `user_addr_set(frames@).len() == count` — the
`count` returned handles own *distinct* physical frames. This is the property
that closes the duplicate-frame / double-free hazard: without it, two handles in
`frames` could alias the same address, so `book_all` would reserve fewer than
`count` frames while the caller believes it owns `count`, and dropping both would
free the same frame twice.

- **Spec decision**: added `user_addr_set(final(frames)@).len() == count` to the
  Ok arm of `alloc_many_user_frames`, and strengthened `lemma_user_bulk_ok`'s
  `ensures` with `user_addr_set(frames).len() == count` (body still `admit()`).
- **Proving-phase obligation**: distinctness must be discharged from the frame
  allocator's non-aliasing guarantee — each successful `frame::alloc` returns an
  address not currently in `free_frames`/`allocated_frames` overlap and removes
  it, so the per-iteration `push`ed addresses are pairwise distinct. This relies
  on `frame::alloc`'s (not-yet-verified) postcondition that the returned frame
  was free and becomes reserved. Recorded; no code change required.

### OBS-3: `alloc_kernel_frame` Err liveness vs. the `KernelFrame::new` failure path

**RESOLVED (spec phase, turn 2).** Turn 1's Fix D added an Err-arm clause
`old(self)@.free_count() == 0` (the liveness contrapositive: kernel allocation
fails only when no frame is free), discharged by an admitted lemma
`lemma_kernel_alloc_err_empty(pre) requires pre.wf() ensures
pre.free_count() == 0`. The reviewer (turn 2) correctly flagged this as
**unsound**: that lemma claims *every* well-formed partition has zero free
frames, which is `false`, and as a `pub proof fn` with `admit()` it was a
soundness landmine callable from any proof.

Root cause: `alloc_kernel_frame` has a **second** Err path —
`KernelFrame::new(frame_addr)` is fallible (`kframe.rs:84-100`:
`PageAligned::from_raw_value(...)?; crate::mm::virt::identity_map_page(...)?`)
and runs *after* `frame::alloc()` already returned `Ok`. On that path a frame
*was* free (so `old(self)@.free_count() >= 1`), the frame is freed back (so
`final(self)@ == old(self)@` still holds), and the function returns `Err`. So
`old(self)@.free_count() == 0` is **false** on a real, reachable Err return.

**Resolution**: the wrap-failure outcome is **not observable** in
`FrameAllocView` — no field distinguishes "exhaustion" from "handle-wrap
failure" — so no abstract `free_count` clause can be soundly asserted on the Err
arm. The Err arm was corrected to the strongest sound statement, the
frame-condition alone:

```rust
Err(_) => final(self)@ == old(self)@,
```

The false `lemma_kernel_alloc_err_empty` proof fn and both its call sites were
**deleted** (`manager.proof.rs`, `manager.rs`). Whole-crate `admit` count
dropped 11→10 accordingly. `frame::alloc()?` was restored (no explicit `match`
needed once the Err-path lemma call is gone).

Evidence retained: `frame.rs:702-712` (`frame::alloc` Err spec = `true`),
`kframe.rs:74-100` (`KernelFrame::new` Err spec = `true`, fallible
`identity_map_page`). The `free_count()==0` liveness fact remains genuinely
inexpressible at this abstraction; if a caller ever needs it, the fix is an
exec-behavior change (make `KernelFrame::new` infallible in the kernel-frame
context, or convert wrap-failure to a panic) — out of scope for the spec phase.


## Proving-phase outcome (admit discharge)

No code bugs were found while proving `manager.rs`. Two source changes were
required and both are sanctioned by `verus-ai-logs/tcb-allowed.md` (not bug
fixes):

- `PhysMemoryManager::init` and `kernel_watermark()` gained
  `#[verus_verify(external_body)]`. Verus cannot translate their bodies
  (`init` writes the `static mut PHYS_MEMORY_MANAGER`; `kernel_watermark`
  reads the build-time constant `config::kernel::KERNEL_WATERMARK` from a
  non-Verus crate). Both are already listed in `tcb-allowed.md`; without the
  attribute the crate does not compile under Verus.
- `check_user_watermark` was refactored to bind `let available =
  frame::free_count();` *before* the `kernel_watermark() + count` overflow
  check. This is behaviour-preserving (`free_count()` is a side-effect-free
  read) and lets the `available as nat == phys_view().frames.free_count()`
  postcondition supply the bound `free_count() <= usize::MAX` on every path,
  which the overflow `Err` arm relies on.

### Discharged (admit removed)

- **`lemma_free_count_bounded`** — deleted. The `<= usize::MAX` bound is now
  obtained soundly from `frame::free_count()`'s `usize`-typed result (see the
  `check_user_watermark` refactor above) instead of being assumed.
- **`lemma_kernel_bulk_err_restored`** — deleted (dead code, 0 call sites).
  The kernel contiguous-bulk `Err` arm proves `final(self)@ == old(self)@`
  directly from the loop invariant `self@ == g_old` (the kernel path never
  mutates `self@`: `frame::alloc_contiguous` is a free function).
- **`lemma_user_bulk_ok`** — deleted and replaced by a real inductive proof.
  `alloc_many_user_frames`'s loop now carries `user_bulk_inv`, discharged via
  proven helper lemmas (`lemma_user_addr_set_empty/_push`,
  `lemma_book_all_empty`, `lemma_book_all_alloc_one`, `lemma_user_bulk_base`,
  `lemma_user_bulk_step`). The `book_all`-accumulation and distinctness
  (`user_addr_set(frames@).len() == count`, OBS-2) follow from `Upool::alloc`'s
  real `&mut self` transition spec (`final@ == old@.alloc_one(uf@)`, `uf@`
  drawn from `old@.free_frames`), so each handle owns a distinct,
  previously-free frame.

### Remaining (irreducible within this module's scope — record only)

These 4 `admit()`s are the genuine §8 ghost-token / Drop trust boundary; they
are **not** dischargeable in `kernel::mm::phys::manager` without out-of-scope
changes, and forcing them would require unsound axioms:

- **`lemma_manager_attached`** (`m@ == phys_view().frames`). `m@ == m.upool@`
  (closed) and `phys_view()` are both `uninterp`; no axiom in scope links the
  manager/upool view to the global partition. The link is the §8 ghost-token
  attachment, realized by a token over the `frame::INSTANCE` singleton — which
  lives in the out-of-scope `frame` layer.
- **`lemma_kernel_alloc_one`** / **`lemma_kernel_alloc_contiguous`**
  (`post == pre.alloc_one(addr)` / `pre.book_all(...)`). `frame::alloc` and
  `frame::alloc_contiguous` are *free functions* that do not take `self`, and
  `phys_view()` is a **constant** (parameter-free) accessor, so the pre→post
  partition transition is inexpressible at this layer (asserting it alongside
  `lemma_manager_attached` would even force `pre == post`). Expressing the
  transition needs a versioned/stepped ghost token threaded through the frame
  layer (out of scope; `frame.rs` and `phys_view()` are not modifiable here).
  Compare `Upool::alloc`, which *can* express its transition because it takes
  `&mut self` — that is exactly why the user bulk path was dischargeable above.
- **`lemma_user_bulk_err_restored`** (`m@ == pre` after `frames.clear()`).
  `clear()` drops the already-allocated `UserFrame`s, which frees them via
  `UserFrame::drop` → `frame::free`. Verus does not model `Drop` side effects
  on `self@`, so the restoration of the partition is genuinely unobservable in
  the exec model and must be asserted.

Net: manager `admit()` count 7 → 4. `make verify-kernel MODULE=mm::phys`
reports 0 errors (42 verified).
