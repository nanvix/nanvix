## Response to Turn 2

### What I Changed

Applied Fix Request D2 — replaced the unsound Fix D with the strongest *sound*
spec. The reviewer was correct: the old Err clause `old(self)@.free_count() == 0`
and its backing lemma were false on the reachable `KernelFrame::new`
wrapping-failure path (a frame *was* free, then freed back), and the lemma
`requires pre.wf() ensures pre.free_count() == 0` was an unconditionally-false
soundness landmine.

**`src/kernel/src/mm/phys/manager.rs` — `alloc_kernel_frame`**
- Err arm (now L359): reverted to the single sound clause
  `Err(_) => final(self)@ == old(self)@` — deleted `&&& old(self)@.free_count() == 0`.
- Body: removed the explicit `match frame::alloc()` wrapper and both
  `lemma_kernel_alloc_err_empty(g_old)` calls (the early-return `proof!` block and
  the `else` branch). Restored `let frame_addr = frame::alloc()?;`. The Ok-path
  `lemma_kernel_alloc_one(...)` call is unchanged.

**`src/kernel/src/mm/phys/manager.proof.rs`**
- Deleted the entire false `lemma_kernel_alloc_err_empty` proof fn (and its doc
  comment). It is no longer callable from anywhere in the crate.

**`verus-ai-logs/nanvix-phys-phys-manager/bugs.md`**
- Updated OBS-3 from "open contradiction tracked" to **RESOLVED (spec phase)**:
  the Err arm is now `final(self)@ == old(self)@`; `free_count()==0` is genuinely
  inexpressible at the `FrameAllocView` abstraction because the
  `KernelFrame::new` wrapping failure is a real Err mode invisible to the view.
  Retained the `kframe.rs:84` / `identity_map_page` evidence.

Reviewer greps confirmed:
```
$ grep -n "free_count() == 0" src/kernel/src/mm/phys/manager.rs
(no match)
$ grep -rn "lemma_kernel_alloc_err_empty" src/kernel/src/mm/phys/
(no match — lemma and both call sites gone)
```

### Verification

`make verify-kernel MODULE=mm::phys`:
```
=== Summary ===
  verification: 19 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=24 admit=10 trusted=0 no_decreases=0 cfg_gate=10
  coverage: 24/45 exec functions have contracts
```
Whole-crate `admit` count dropped **11 → 10** (manager.proof.rs 9 → 8) — the
false lemma is gone, exactly as the reviewer required.

`make verify` (full, no regressions):
```
Exit code : 0
  ... mm::phys: cheating: assume=0 external_body=24 admit=10 trusted=0 cfg_gate=10
```

`./z build -- check-kernel` (non-verus dual compilation):
```
{"reason":"build-finished","success":true}
[OK]    Build complete.
```

### Result: FIXED

The unsound Err clause and its false admitted lemma are removed; the Err arm now
states the strongest sound postcondition (`final(self)@ == old(self)@`).
`make verify-kernel` and `make verify` are Exit 0, the non-verus build is green,
the whole-crate admit count fell 11→10, and both reviewer greps return nothing.
OBS-3 is updated to record the resolution.
