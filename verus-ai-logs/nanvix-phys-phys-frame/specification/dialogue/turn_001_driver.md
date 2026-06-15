## Turn 1 (re-review after proving→specification rollback): Caller coverage / No specs weakened → FAIL → ROLLBACK to view-design

### Progress
- Done (verified this turn): all 17 checklist items walked with tool-backed evidence.
- Current: 4 interlocking FAILs (items 2, 9, 11, 14) sharing ONE root cause in
  view-design; local fix attempted in-tree and proven impossible.
- Remaining: none — rollback filed (see `../ROLLBACK` … actually `dialogue/ROLLBACK`).

### Verification (commands I ran, not trusted verbally)

`make verify-kernel` → exit 0, but `status: CHEATING_DETECTED`
(`assume=0 external_body=22 admit=1 cfg_gate=9`). The single `admit` is in
`frame::free`.

Read the actual contracts:
- `Inner::alloc` (frame.rs:115-136) has a CORRECT two-state spec:
  `final(self)@ == FrameAllocView{ allocated_frames: old(self)@.allocated_frames.insert(frame@), … refcounts.insert(frame@,1) }`.
- The free-function shim `frame::alloc` (frame.rs:731-759) can only state facts
  over `phys_view()` — an **argument-free `uninterp spec fn phys_view() -> PhysMemView`**
  (mod.spec.rs:171) pinned by `instance()` to the **pre**-call state
  (`(*result)@ == phys_view().frames`). Its current Ok-arm is
  `frame.inv() && phys_view().frames.free_frames.contains(frame@)` — a **pre-state**
  fact (the frame is still FREE), the disjoint-OPPOSITE of what callers need.

**Caller expectation (authoritative `caller_analysis.md`):** for `alloc` (Ok),
"returned frame is … now in `allocated_frames`, with `refcounts[frame] == 1`";
"**Would break callers:** … returning a frame not in `allocated_frames`/with
refcount ≠ 1." The shim delivers `free_frames.contains` → caller coverage FAILS.
Same shape for `book`, `alloc_range`, `alloc_contiguous`, and the increment in `share`.

**Where did the strong guarantee go?** It was laundered up the call chain into
`manager::alloc_user_frame` (manager.rs:267, `external_body`, ensures
`phys_view().frames.allocated_frames.contains(frame@)`) and
`manager::alloc_kernel_frame` (manager.rs:352, same). These `external_body`
functions are **NOT** in `tcb-allowed.md` (which states "Any `external_body`
outside this list must be removed"). `bugs.md` claims they are "all listed in
tcb-allowed.md" — that claim is **false** (verified by reading tcb-allowed.md).

**Local fix attempted in-tree (this phase):** I restored the caller-expected
strong Ok-arm on `frame::alloc`
(`allocated_frames.contains(frame@) && refcounts.contains_key(frame@) && refcounts[frame@]==1`)
and ran `make verify-kernel`:
```
error: postcondition not satisfied  --> src/kernel/src/mm/phys/frame.rs:751
error: postcondition not satisfied  (cascades into Upool::alloc, manager.rs:268)
verification results:: 31 verified, 2 errors   (exit 101)
```
Then `git checkout -- frame.rs` (reverted clean).

**Reproducers re-run by me** (`/home/ruize/verus-bin/verus`):
- `02_goal_is_false.rs` → `1 verified, 0 errors` — the NEGATION of the strong shim
  postcondition is provable from the only sound `instance()` bridge + `wf`
  disjointness ⇒ the strong spec is *false*, not merely unproven.
- `01_shim_fails.rs` → `0 verified, 1 errors` — faithful isolated model fails.
- `03_strengthening_derives_false.rs` → `1 verified, 0 errors` — strengthening
  `instance()` to reflect post-state derives `false` (unsound).

### Full checklist verdict
1. Function coverage — PASS (every in-scope shim + `Inner::*` + `instance` has `#[verus_spec]`).
2. Caller coverage — **FAIL** (alloc/book/alloc_range/alloc_contiguous/share Ok-arms are pre-state; caller_analysis requires post-state `allocated_frames`+`refcount==1`; "would break callers").
3. View consistency — structurally PASS (specs reference `phys_view().frames`, preserve `inv()`), but the View **cannot express post-state** — the root defect.
4. No tautological ensures — WEAK: `alloc_contiguous` Err`=> true` and `init` Err`=> true` remain (contract-constrained; secondary).
5. No subsumed ensures — PASS.
6. Error paths meaningful — mostly PASS; `alloc_contiguous` Err arm is `true`.
7. No internal `assume_specification` — PASS (none).
8. vstd before assume_spec — N/A.
9. Specs written for the caller — **FAIL** (shim specs are NOT usable in caller proofs to obtain the allocation effect; callers fall back to unsanctioned `external_body` axioms).
10. Trait obligations — PASS (`free`: `opens_invariants none` + `no_unwind`, no requires) — but body carries `admit()`.
11. Spec completeness (advisory) — **FAIL vs caller expectations** (the weakening does NOT match caller expectations; caller_analysis says it "would break callers" — so it is NOT acceptable intentional nondeterminism).
12. Loop invariants — PASS (only loop is in `init`, which is `external_body`).
13. No cheating on own functions — `admit=1` in `frame::free` (deferred). Flagged but secondary to the core defect.
14. No specs weakened — **FAIL** (post-state effects weakened to pre-state facts; weakening cascaded into `Upool::alloc` and into `manager` `external_body` axioms not in tcb-allowed.md).
15. Bug awareness — PARTIAL: `bugs.md` records the `phys_view()` limitation, but mislabels it "out of scope / not a defect to fix here" and falsely claims the manager fns are tcb-allowed. The limitation is a **rollback trigger**, not an acceptable spec outcome.
16. Cross-module regression — PASS *only because* the guarantee was removed (all modules verify with the weakened specs).
17. Verification + build — PASS at exit 0 with weakened specs + 1 admit; **FAILS `31 verified, 2 errors`** the moment the caller-expected spec is restored.

### Root cause and decision
Items 2, 9, 11, 14 share ONE root cause: `phys_view()` is a **stateless,
argument-free `uninterp` constant** (a view-design artifact in `mod.spec.rs`).
A constant has the same value at every program point, so a *verified* (non-
`external_body`) mutating shim cannot relate its post-state to the abstraction.
The PhysMemView transition helpers (`spec_book_frame`, `spec_book_frames`,
`spec_initialize`) exist but are unusable by the shims for exactly this reason.
view_design.md line 205 claims "alloc/book post-state … expressible ✅" — that
claim is **provably false** for the verified layer (reproducer 02 + in-tree test).

No specification-phase-local fix can recover the caller-required guarantee
(attempted in-tree → `31 verified, 2 errors`; any provable shim spec must drop the
allocation-effect fact, which is the forbidden weakening). The fix requires
changing the View itself (state-indexed `phys_view` or a tracked ghost token) in
`mod.spec.rs` plus the frozen `Inner::*` transition threading — **view-design scope**.

### Result: ROLLBACK to view-design (see `dialogue/ROLLBACK`)
Not RESOLVED. STOP file intentionally not created.
