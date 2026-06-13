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

The Err-arm contract now asserts `old(self)@.free_count() == 0` (the liveness
contrapositive: kernel allocation fails only when no frame is free). This holds
for the **allocator-exhaustion path** (`frame::alloc()` returns `Err`), whose
discharge needs `frame::alloc`'s `Err` spec to expose
`phys_view().frames.free_count() == 0` — the recorded cross-module obligation.

However, `alloc_kernel_frame` has a **second** Err path:
`KernelFrame::new(frame_addr)` can fail (its spec is `Err(_) => true`, and
`identity_map_page` is fallible) *after* `frame::alloc()` already succeeded. On
that path the just-allocated frame is freed back (so `final(self)@ ==
old(self)@` still holds), but `old(self)@.free_count() >= 1` — because a frame
was available to allocate — so `old(self)@.free_count() == 0` is **false** there.

- **Current state**: the contract is wired through the admitted lemma
  `lemma_kernel_alloc_err_empty(pre) requires pre.wf() ensures
  pre.free_count() == 0`, invoked on *both* Err paths, so `make verify-kernel`
  passes (admit). The lemma as stated is **not soundly dischargeable for the
  `KernelFrame::new` branch** and must be resolved in the proving phase by one
  of:
    1. Strengthening `KernelFrame::new` to be infallible in the kernel-frame
       context (the page-table pool is BSS-backed, so `identity_map_page` does
       not recursively allocate — see `kframe.rs:84-100`), proving that branch
       dead; **or**
    2. Reclassifying the wrapping failure as a panic/abort rather than a
       recoverable `Err`, so the only reachable `Err` is exhaustion; **or**
    3. Weakening the Err arm to the sound disjunction "exhaustion *or*
       wrapping-failure", dropping the unconditional `free_count() == 0`.
  Evidence: `frame.rs:702-712` (`frame::alloc` Err spec = `true`),
  `kframe.rs:74-100` (`KernelFrame::new` Err spec = `true`, fallible
  `identity_map_page`). Recorded per reviewer instruction (Fix D) — not silently
  dropped.
