## Response to Turn 1

### What I Changed

I attempted the discharge exactly as instructed (Fix Request steps 1–4). The
honest result after exhausting the allowed edits is **NEEDS_GUIDANCE / ROLLBACK**:
the six mutating-shim `ensures` are *logically false* against the frozen,
argument-free `uninterp spec fn phys_view()`, and the only `instance()`
strengthening that could discharge them is provably inconsistent (derives
`false`). Per the reviewer's own item 4, this is the case that qualifies for a
ROLLBACK to the specification phase. I produced the required isolated reproducers
and the failed-`instance()`-strengthening evidence (all run with the project's
pinned Verus `0.2026.05.24`).

Files touched (evidence artifacts only — **no spec weakened, no `frame.rs` change
left in place**):

- `verus-ai-logs/nanvix-phys-phys-frame/reproducers/01_shim_fails.rs` — isolated
  model of the exact architecture; reproduces the identical failure.
- `verus-ai-logs/nanvix-phys-phys-frame/reproducers/02_goal_is_false.rs` — proves
  the shim's frozen postcondition's **negation** from its own premises under the
  strongest sound `instance()` bridge.
- `verus-ai-logs/nanvix-phys-phys-frame/reproducers/03_strengthening_derives_false.rs`
  — proves that the only `instance()` strengthening that could discharge the shim
  (reflecting post-state into the constant view) derives `false` from two bridge
  evaluations.

`src/kernel/src/mm/phys/frame.rs` is unchanged from the committed `admit=6` PASS
state (verified clean `git status`); the six `admit()`s remain because removing
any of them produces an unsatisfiable obligation (shown below).

#### Step 1 attempted (real proof): remove `admit()` from `alloc`

I deleted `proof! { admit(); }` from the `alloc` shim (frame.rs:755) and ran the
verifier. Exact output:

```
error: postcondition not satisfied
   --> src/kernel/src/mm/phys/frame.rs:742:13
    |
742 |             Ok(frame) => {
    |             ^^^^^^^^^ failed this postcondition
...
752 |     instance().alloc()
    |     ------------------ at the end of the function body

verification results:: 31 verified, 1 errors
```

The failing clause is the `Ok` branch: `phys_view().frames.allocated_frames
.contains(frame@)`.

#### Step 1 attempted (strengthen `instance()`): why no sound contract works

`instance()` is a non-mutating accessor; the strongest sound fact it can ensure
about the returned reference is a single-state pin at its own return point:
`(*result)@ == phys_view().frames`. Feeding that (plus `Inner::alloc`'s frozen
`Ok` ensures `old(self)@.free_frames.contains(frame@)`) into the shim, the goal is
not merely unproven — its **negation is provable**. Reproducer 02 proves exactly
this and verifies (`1 verified, 0 errors`):

```rust
proof fn shim_goal_is_false(frame: int)
    requires
        view_fn().inv(), view_fn().initialized,
        view_fn().frames.free_frames.contains(frame),   // from instance()+Inner::alloc
    ensures
        !view_fn().frames.allocated_frames.contains(frame),   // == NOT(shim goal)
{ /* wf disjointness */ }
```

The only way to make the shim pass is therefore to add an `instance()` ensures
that reflects the *post-mutation* state back into the constant `phys_view()`.
Because `phys_view()` is one argument-free constant, evaluating such a bridge at
the pre- and post-mutation points forces `pre_state == post_state`, i.e. `false`,
for any state-changing operation. Reproducer 03 proves this and verifies
(`1 verified, 0 errors`):

```rust
proof fn two_bridge_evaluations_derive_false(pre, post, frame)
    requires
        pre.wf(), pre.free_frames.contains(frame),
        post == FrameAllocView { allocated: pre.allocated.insert(frame),
                                 free: pre.free.remove(frame) },
        bridge(pre),   // instance() observes view==pre  (before alloc)
        bridge(post),  // would be needed to discharge the post-state ensures
    ensures false,     // <-- verifies: the strengthened contract is inconsistent
{ assert(pre == post); /* yet frame is free in pre, not in post */ }
```

I also confirmed the strengthening is *not even expressible* without editing the
do-not-modify `mod.spec.rs`: there is no `old(phys_view())`, no state-indexed
view, and no ownership token threaded through the frozen `Inner::*` contracts, so
the post-state cannot be named at all inside `frame.rs`/`frame.proof.rs`.

I did **not** replace any `admit()` with `external_body` or `assume()` (both
forbidden here).

### Verification

Final state of the module is the committed `admit=6` PASS (I restored `frame.rs`
after the experiments; `git status` shows it unmodified):

```
make verify-kernel MODULE=mm::phys
verification results:: 32 verified, 0 errors (partial verification with `--verify-*`)
  ⚠️  admit: 6
  cheating: assume=0 external_body=22 admit=6 trusted=0 no_decreases=0 cfg_gate=9
  status: CHEATING_DETECTED   (exit 0)
```

Build (cross-target compile of the guest kernel) is clean:

```
./z build -- check-kernel
    Checking kernel v0.16.17 (/home/ruize/nanvix-phy/src/kernel)
{"reason":"build-finished","success":true}
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.57s
[OK]    Build complete.
```

Isolated reproducers (project-pinned Verus 0.2026.05.24):

```
verus .../reproducers/01_shim_fails.rs        -> 0 verified, 1 errors  (same Ok-branch postcondition failure)
verus .../reproducers/02_goal_is_false.rs     -> 1 verified, 0 errors  (shim goal's NEGATION proven)
verus .../reproducers/03_strengthening_derives_false.rs -> 1 verified, 0 errors  (post-state bridge => false)
```

### Result: NEEDS_GUIDANCE

`admit=0` is **not achievable under the allowed edits**, and I have proven this
rather than asserted it (reproducers 01–03 above, plus the real-code postcondition
error). This is precisely the condition the Fix Request's item 4 defines as
eligible for **ROLLBACK to the specification phase**.

**Root cause (spec-phase):** the abstract subsystem state is modeled as
`uninterp spec fn phys_view() -> PhysMemView` — an argument-free **constant**. A
constant cannot express a pre/post mutation, so a shim whose `ensures` asserts a
post-state membership fact about `phys_view()` is unprovable while `Inner::*` only
guarantees that fact about its own `final(self)@`. The six mutating shims
(`alloc`, `alloc_contiguous`, `free`, `book`, `alloc_range`, `share`) all share
this shape.

**Required spec-phase fix (outside this module's do-not-modify boundary):** make
the view diff-able — e.g. thread `old(phys_view())`/`new` (a state-indexed view)
or an ownership/permission token through the frozen `Inner::*` boundary so the
shim can relate the post-mutation state to the observable view. The companion
query shims that are *not* state-changing (`is_covered`, `refcount`, and now
`free_count`, which I discharged with a real proof) verify with zero `admit`,
confirming the limitation is specific to state mutation, not to the bridge
mechanism itself.

**Exact errors from each approach tried:**
1. Remove `alloc` admit → `error: postcondition not satisfied` at frame.rs:742
   (`Ok(frame)` branch), `31 verified, 1 errors`.
2. Strengthen `instance()` with the strongest *sound* single-state bridge → shim
   goal's negation is provable (reproducer 02, `1 verified`), so the shim is
   unprovable.
3. Strengthen `instance()` to reflect post-state (the only thing that could help)
   → derives `false` (reproducer 03, `1 verified`); inconsistent/unsound.
4. Express the post-state link inside `frame.rs`/`frame.proof.rs` → impossible
   without editing the do-not-modify `mod.spec.rs` (no `old(phys_view())`, no
   state index, no token in the frozen `Inner::*` contracts).
