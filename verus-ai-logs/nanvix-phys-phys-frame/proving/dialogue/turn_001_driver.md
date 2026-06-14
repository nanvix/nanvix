## Turn 1: Zero remaining admit()

### Progress
- Done (PASS this turn):
  - **No specs weakened** — PASS (evidence below).
  - **Zero external_body unless TCB-allowed** — PASS (all 10 in `frame.rs` are listed in `tcb-allowed.md`).
  - **Zero assume/assume_specification** — PASS for `frame.rs` (`assume=0`; the only `assume_specification` in the subsystem is `kframe.spec.rs`, a documented external-bottom trust boundary, out of scope for this file).
  - **No cfg-gated exec code** — PASS for `frame.rs` (only two `#[cfg(verus_keep_ghost)] include!` ghost-spec/proof includes; no exec branches/expressions/match arms).
  - **Exec rewrites minimal & equivalent** — PASS (no `// VERUS REWRITE`; the single `free_count` `nbits`/`used` split is a semantically-equivalent intermediate-binding deviation, commented).
- Current: **Zero remaining admit()** — **FAIL** (6 admits). This is the focus of this turn.
- Remaining (blocked on the admit fix, will re-verify once admits are gone):
  - **Cheating audit** — counts reported below; cannot pass while `admit=6`.
  - **Claimed Verus limitation has an isolated reproducer** — NOT satisfied (no isolated reproducer exists yet).
  - **Cross-module regression (`make verify-kernel` all modules)** — to be run after admits resolved.
  - **Verification + build: 0 errors/0 warnings** — module currently reports `32 verified, 0 errors` but ONLY because of the 6 admits; not a real pass.

### Verification

Commands run:
- `make verify-kernel MODULE=mm::phys` → `status: CHEATING_DETECTED`, `assume=0 external_body=22 admit=6 cfg_gate=9` globally; module verifies `32 verified, 0 errors` but with `admit=6`.

Exact admit locations in `src/kernel/src/mm/phys/frame.rs` (6):
- `frame.rs:756` — `alloc` shim (`proof! { admit(); }` before `instance().alloc()`)
- `frame.rs:797` — `alloc_contiguous` shim
- `frame.rs:857` — `free` shim
- `frame.rs:902` — `book` shim
- `frame.rs:927` — `alloc_range` shim
- `frame.rs:956` — `share` shim

`external_body` in `frame.rs` (10, all TCB-allowed — PASS): `Inner::{alloc(137), alloc_contiguous(210), free(290), share(368), refcount(428), book(481), is_covered(517), alloc_range(565)}`, `instance(652)`, `init(689)`. Each appears in `verus-ai-logs/tcb-allowed.md` (Inner::* methods, `instance` bridge axiom, `init` skip/exclude).

Spec-drift evidence (PASS): `frame.spec.rs` defines only the do-not-modify items (`View for Inner`, `Inner::internal_inv`, `Inner::inv`, `frame_addr_of`) plus the proof helper `lemma_free_count`. No shim/`Inner::*` `#[verus_spec]` contract was weakened — they retain full post-state `ensures`. `free_count` was strengthened from a deferred admit to a body-verified proof via `lemma_free_count`. No guarantee was downgraded.

**Empirical confirmation the admits are load-bearing (not removable as-is):**
I removed the `admit()` from the `alloc` shim and ran `make verify-kernel MODULE=mm::phys`:
```
error: postcondition not satisfied
752 |     instance().alloc()
verification results:: 31 verified, 1 errors
```
So the post-state ensures of `alloc` (`Ok(frame) => phys_view().frames.allocated_frames.contains(frame@) && refcounts[frame@] == 1`) is NOT currently dischargeable. (I reverted this experiment; HEAD restored to the `admit=6` PASS commit `52d779150`, worktree clean.)

The prover's own `verification-todo.md` argues this obligation is *false* against the frozen, argument-free `uninterp spec fn phys_view()`: `instance()` ensures `(*result)@ == phys_view().frames` pins the bridge to the PRE state; after `instance().alloc()` mutates `*result`, `phys_view().frames` still equals the pre state, where `frame@` is free, so `phys_view().frames.allocated_frames.contains(frame@)` contradicts `FrameAllocView::wf` disjointness.

### Fix Request

**Item: Zero remaining admit() — discharge all 6 admits in `frame.rs`.**

Do this, in order, for `alloc`, `alloc_contiguous`, `free`, `book`, `alloc_range`, `share`:

1. **Attempt a real proof first.** Note that `instance()` is NOT on the do-not-modify spec list (it is a *target* function, per `verification-plan.json` background). The frozen items are the `Inner::*` method contracts, `phys_view()`, `PhysMemView`, and `FrameAllocView`. So you MAY strengthen/re-spec `instance()` (and add proof helpers in `frame.proof.rs`) if that lets the shims discharge their post-state `ensures` soundly. Try to construct a bridge (e.g., a contract/token on `instance()` and the post-call) that ties `final(*result)@` back to `phys_view().frames` WITHOUT introducing an inconsistency. Then delete each `proof! { admit(); }`.

2. **Verify after each removal:** run `make verify-kernel MODULE=mm::phys`. The acceptance bar is: `admit=0` in the summary, `0 errors`, `0 warnings`. Do not replace an `admit()` with `external_body` (forbidden on current-module functions and these are not on the TCB list for that purpose) or with `assume()`.

3. **Do NOT submit prose as the fix.** `verification-todo.md` already explains *why* it's hard; that is a justification, not a fix. Justification is not a fix — change the code/specs and show `make verify-kernel MODULE=mm::phys` with `admit=0`, or do step 4.

4. **If — and only if — you conclude it is genuinely impossible under the allowed edits**, you must prove that claim, not assert it. Provide an **isolated minimal reproducer** (separate `verus!` snippet) that:
   - models the exact construct: an `uninterp spec fn` constant view, an `external_body` accessor with `ensures (*r)@ == view()`, a mutating method with a frozen `old/final` contract, and a shim asserting a post-state fact about `view()`; and
   - shows that with `instance()`'s contract free to change, no sound contract discharges the shim ensures (demonstrate that the only "successful" version derives `False`, i.e. pre==post, from two bridge evaluations).

   Also explicitly show that strengthening `instance()` (the one modifiable boundary) cannot work, with the verifier output. Only then is this eligible for ROLLBACK to the specification phase (root cause: argument-free constant `phys_view()` cannot express a pre/post mutation; spec phase must make the view state-indexed or thread an `old(phys_view())`/ownership token through the `Inner::*` boundary). Until that isolated reproducer + failed `instance()`-strengthening evidence exist, ROLLBACK will not be accepted.

Report back with the `make verify-kernel MODULE=mm::phys` summary line (`assume= external_body= admit= cfg_gate=`) so I can verify `admit=0`.
