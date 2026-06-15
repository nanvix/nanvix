## Turn 2: Zero remaining admit() — verifying fixer's ROLLBACK claim

### Progress
- Done (PASS): No specs weakened; Zero external_body unless TCB-allowed (10 in frame.rs all listed); Zero assume/assume_specification (assume=0); No cfg-gated exec code in frame.rs; Exec rewrites minimal/equivalent.
- Current: **Zero remaining admit()** — still **FAIL (admit=6)**, but I have now independently CONFIRMED the fixer's claim that this is a **spec-phase root cause** (ROLLBACK), not a provable-in-phase obligation.
- Remaining (cannot pass until spec-phase redesign lands): Cheating audit (admit=6); Isolated reproducer (now SATISFIED — see below); Cross-module regression; Verification/build 0-err-0-warn (module reports 32 verified/0 errors only by virtue of the 6 admits).

### Verification (I ran everything myself — did not trust the fixer's prose)

1. **Spec architecture (read `mod.spec.rs`):** the abstract subsystem state is
   `pub uninterp spec fn phys_view() -> PhysMemView;` (mod.spec.rs:171) — an
   **argument-free constant**. `instance()`'s only sound bridge is
   `(*result)@ == phys_view().frames` pinned at its return (pre-state). `Inner::*`
   methods have correct `old(self)@ → final(self)@` diffing contracts, but there is
   no `old(phys_view())`, no state index, and no ghost token threaded through the
   frozen `Inner::*` boundary, so a post-mutation fact about the constant
   `phys_view()` cannot be named inside `frame.rs`/`frame.proof.rs`.

2. **Isolated reproducers — I ran all three with the pinned `/home/ruize/verus-bin/verus`:**
   - `reproducers/01_shim_fails.rs` → `0 verified, 1 errors`. Faithful model
     (uninterp constant view + `external_body` `instance()` with
     `(*r)@ == view().frames` + frozen `old/final` `Inner::alloc` + shim asserting
     post-state membership). Same `instance().alloc()` postcondition failure as real code.
   - `reproducers/02_goal_is_false.rs` → `1 verified, 0 errors`. Proves the shim's
     frozen postcondition's **negation**: under the strongest *sound* single-state
     `instance()` bridge, `view().frames.free_frames.contains(frame)` (from
     `Inner::alloc`'s Ok ensures) + `wf` disjointness ⇒
     `!view().frames.allocated_frames.contains(frame)`. The spec is **false**, not merely unproven.
   - `reproducers/03_strengthening_derives_false.rs` → `1 verified, 0 errors`.
     Proves the only `instance()` strengthening that could discharge the shim
     (reflecting post-state into the constant view) is **inconsistent**: two bridge
     evaluations across the mutation force `pre_state == post_state` ⇒ `false`.

   This precisely satisfies Turn-1 Fix-Request item 4: the limitation is reproduced
   in isolation AND the one modifiable boundary (`instance()`) is shown to be
   unable to help (its only helpful strengthening derives `false`).

3. **Real-code corroboration:** in Turn 1 I removed the `alloc` admit and got
   `error: postcondition not satisfied` at `instance().alloc()` (`31 verified, 1 errors`).
   Reproducer 01 reproduces this in isolation. The fixer left `frame.rs` unmodified
   (`git status` clean) and the committed module is `32 verified, 0 errors, admit=6`.

4. **Per-function:** the FAIL is identical in shape for all six mutating shims —
   `alloc` (756), `alloc_contiguous` (797), `free` (857), `book` (902),
   `alloc_range` (927), `share` (956). Each tail-calls an `Inner::*` mutator and
   asserts a post-state membership fact about the constant `phys_view()`. The pure
   queries (`is_covered`, `refcount`, `free_count`) verify with zero admit because
   pre==post for them — confirming the defect is specific to *state mutation* under
   a constant view, not to the bridge mechanism.

### Decision

The fixer did NOT merely justify — they delivered machine-checked reproducers that
I independently re-ran and confirmed. The six shim postconditions are **logically
false/inconsistent** against the frozen, argument-free `uninterp spec fn
phys_view()`. The only in-phase levers are exhausted:
- Strengthen the proof → impossible (goal is false: reproducer 02).
- Re-spec the (non-frozen) shim contracts equally strongly → impossible: any
  post-state effect fact about the constant `phys_view()` is false, so a *provable*
  shim spec must drop the effect = **weakening** (forbidden by "No specs weakened").
- Strengthen `instance()` (the one modifiable boundary) → inconsistent (reproducer 03).
- `external_body`/`assume` → forbidden on current-module functions.

Root cause is upstream in the **specification** phase: `phys_view()` is a constant
and cannot model the singleton's mutation. **ROLLBACK filed** (see
`proving/dialogue/ROLLBACK`). No STOP created — the checklist is not RESOLVED.

### Fix Request
None to the fixer — the local fix space is provably exhausted. The required change
belongs to the specification phase (make `phys_view()` diff-able: thread
`old/new`/state-index it, or thread a ghost ownership token through the `Inner::*`
boundary so the mutating shims can relate their post-state to the observable view).
Details in the ROLLBACK file.
