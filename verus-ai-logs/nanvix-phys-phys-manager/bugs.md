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


### OBS-4: §8 ghost-token attachment unbuildable under cheating-elimination rules (proving-phase blocker)

The four `manager.proof.rs` `admit()` lemmas (`lemma_manager_attached`,
`lemma_kernel_alloc_one`, `lemma_kernel_alloc_contiguous`,
`lemma_user_bulk_err_restored`) all depend on the §8 global-state attachment
(`self@ == phys_view().frames`). `phys_view()` and the manager/pool views are
all `uninterp`; relating them across exec calls requires a `tracked` ghost token
threaded through `frame::alloc`/`free`/`alloc_contiguous` signatures — forbidden
by the cheating-elimination source-integrity rules, and not expressible as an
AI-written `axiom`. The lemma specs are correct under the intended attachment; the
attachment infrastructure is simply absent. Full self-contained write-up:
`verus-ai-logs/nanvix-phys-phys-frame/cheating-elimination/bugs.md`. Unblock =
build the §8 token machinery (proving phase) or human-sanction the attachment axiom.
