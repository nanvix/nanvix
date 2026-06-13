## Response to Turn 1

### What I Changed

All four fixes (A–D) were applied as code changes. Proof lemma bodies remain `admit()`.

**Fix A — `init`: remove the tautological Err arm**
- `src/kernel/src/mm/phys/manager.rs:101` — changed `Err(_) => true` to
  `Err(_) => crate::mm::phys::phys_view().manager_ready`. `init` returns `Err`
  only on double-init (the singleton is already set), so the manager is already
  ready. `grep "Err(_) => true"` in `manager.rs` now returns nothing.

**Fix B — `alloc_many_kernel_frames`: restore contiguity + no-leak**
- `src/kernel/src/mm/phys/manager.rs:426` — Ok arm now asserts
  `kernel_frames_contiguous(final(frames)@, count as nat)` (the
  `∃ base. base%ps==0 ∧ ∀i. frames@[i]@ == base + i*ps` fact) alongside the
  existing `all_free`/`book_all` set effect and `len() == count`.
- `src/kernel/src/mm/phys/manager.rs:432` — Err arm now asserts
  `final(frames)@.len() == 0` (vec emptied, no leak) in addition to
  `final(self)@ == old(self)@`. Discharged from `frames.clear()` (vstd
  `Vec::clear` ensures `@ == Seq::empty()`) on the cleanup path; the
  input-check and `alloc_contiguous?` Err paths leave `frames` untouched at
  `len 0`.
- `src/kernel/src/mm/phys/manager.spec.rs:125` — new helper
  `kernel_frames_contiguous(frames, count)` encapsulating the nested
  `∃ base / ∀ i` contiguity (triggered on
  `region_frame_addrs(base, count*ps)`) to keep quantifier triggers controlled.
- `src/kernel/src/mm/phys/manager.proof.rs:75` — strengthened
  `lemma_kernel_alloc_contiguous` `ensures` with
  `kernel_frames_contiguous(frames, count)` (body still `admit()`).

**Fix C — `alloc_many_user_frames`: restore distinctness**
- `src/kernel/src/mm/phys/manager.rs:183` — Ok arm now asserts
  `user_addr_set(final(frames)@).len() == count`.
- `src/kernel/src/mm/phys/manager.proof.rs:114` — strengthened
  `lemma_user_bulk_ok` `ensures` with
  `user_addr_set(frames).len() == count` (body still `admit()`).
- Recorded **OBS-2** in `verus-ai-logs/nanvix-phys-phys-manager/bugs.md`.

**Fix D — `alloc_kernel_frame`: restore the liveness Err fact**
- `src/kernel/src/mm/phys/manager.rs:361` — Err arm now asserts
  `old(self)@.free_count() == 0` in addition to `final(self)@ == old(self)@`.
- `src/kernel/src/mm/phys/manager.rs:368-393` — expanded `frame::alloc()?` into
  an explicit `match` so the liveness lemma can be invoked on the early-return
  Err path (`:373`), and added the `else` branch on the `KernelFrame::new`
  result (`:390`).
- `src/kernel/src/mm/phys/manager.proof.rs:54` — new admitted lemma
  `lemma_kernel_alloc_err_empty(pre) requires pre.wf() ensures
  pre.free_count() == 0`.
- Recorded **OBS-3** in `bugs.md`: the liveness fact is soundly dischargeable
  only on the allocator-exhaustion path; the `KernelFrame::new` wrapping-failure
  branch (which returns `Err` after a *successful* `frame::alloc`, where
  `free_count() >= 1`) is an open contradiction flagged for the proving phase,
  with `frame.rs`/`kframe.rs` spec evidence. Not silently dropped.

### Verification

`make verify-kernel MODULE=mm::phys`:
```
=== Summary ===
  verification: 20 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=24 admit=11 trusted=0 no_decreases=0 cfg_gate=10
  coverage: 24/45 exec functions have contracts
  status: CHEATING_DETECTED   # admit()/external_body counts only — expected in spec phase
```

`make verify` (full, no regressions):
```
=== Results ===
  Exit code : 0
```

`./z build -- check-kernel` (non-verus dual compilation):
```
{"reason":"build-finished","success":true}
[OK]    Build complete.
```

### Result: FIXED

All four reviewer requests landed as code changes. `make verify-kernel` and
`make verify` are Exit 0, and the non-verus build is green. The Fix D
`KernelFrame::new` soundness caveat is recorded as OBS-3 with spec evidence (per
the reviewer's "do not silently drop" instruction) for the proving phase to
resolve.
