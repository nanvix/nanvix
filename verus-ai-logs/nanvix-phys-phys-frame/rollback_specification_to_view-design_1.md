ROLLBACK

## Root Cause

The View for `mm::phys` models the global frame-allocator singleton through

    pub uninterp spec fn phys_view() -> PhysMemView;        // mod.spec.rs:171

— an **argument-free, uninterpreted constant**. A constant has the same value at
every program point, so it cannot distinguish the pre- and post-state of a
*mutating* operation on the singleton. The only bridge from the live `Inner` to
the abstraction is the TCB accessor

    instance()  // frame.rs:644-652, external_body (tcb-allowed.md)
        ensures (*result).inv() && (*result)@ == phys_view().frames

which pins `phys_view().frames` to the **pre**-state. After `instance().alloc()`
mutates `*result`, `phys_view()` still equals the pre-state, where the just-
allocated frame is still *free*. By `FrameAllocView::wf` disjointness
(`allocated_frames.disjoint(free_frames)`), a post-state membership `ensures` on a
verified shim is not merely unproven — it is **provably false**.

Consequently the verified (non-`external_body`) free-function shims
(`alloc`, `alloc_contiguous`, `book`, `alloc_range`, and the increment in `share`)
**cannot express the post-state allocation effect that `caller_analysis.md`
requires**. The current specification round worked around this by weakening each
shim to a *pre-state* fact (e.g. `frame::alloc` Ok ⇒ `free_frames.contains(frame@)`
instead of `allocated_frames.contains(frame@) && refcounts[frame@]==1`). That
weakening:

- directly contradicts the documented caller expectation
  (`caller_analysis.md`: alloc Ok ⇒ "now in `allocated_frames`, with
  `refcounts[frame] == 1`"; "**Would break callers:** returning a frame not in
  `allocated_frames`/with refcount ≠ 1"),
- cascaded into `Upool::alloc` (weakened identically), and
- relocated the real guarantee into `manager::alloc_user_frame` /
  `manager::alloc_kernel_frame` (`external_body` axioms, manager.rs:267/352) that
  are **not** listed in `tcb-allowed.md` (which requires any such `external_body`
  to be removed).

view-design's own `view_design.md` (line 205) asserts "alloc/book post-state …
expressible ✅". That claim is false for the verified layer and is the precise
design gap that must be closed.

## Failed Local Fixes (attempted in THIS specification phase)

1. **Restore the caller-expected strong Ok-arm on `frame::alloc` in-tree** and run
   `make verify-kernel`:
       error: postcondition not satisfied  --> src/kernel/src/mm/phys/frame.rs:751
       error: postcondition not satisfied  (cascade into Upool::alloc, manager.rs:268)
       verification results:: 31 verified, 2 errors   (exit 101)
   (reverted with `git checkout -- frame.rs`).

2. **Re-spec the shims "equally strongly" without touching the View.** Impossible:
   `reproducers/02_goal_is_false.rs` (`1 verified, 0 errors`, re-run by me) proves
   the *negation* of the post-state shim postcondition from the shim's own premises
   under the strongest sound single-state `instance()` bridge. Any provable shim
   spec must therefore drop the allocation-effect fact — the forbidden weakening.

3. **Strengthen the one modifiable boundary `instance()` to reflect post-state.**
   `reproducers/03_strengthening_derives_false.rs` (`1 verified, 0 errors`, re-run
   by me) proves such a bridge forces `pre_state == post_state`, i.e. derives
   `false` for any mutating op — an unsound false axiom, strictly worse than `admit`.

4. **Name the post-state inside `frame.spec.rs` / `frame.proof.rs`.** Not
   expressible: there is no `old(phys_view())`, no state-indexed view, and no ghost
   token threaded through the frozen `Inner::*` contracts. `phys_view()` is one
   constant; its post-state cannot be referred to in this module.

`external_body`/`assume` on current-module shims are forbidden, so they were not
used. The pure-query shims (`is_covered`, `refcount`, `free_count`) verify with no
`admit`, confirming the defect is specific to *state mutation under a constant
view*, not to the bridge mechanism.

## What view-design Should Fix

Make the subsystem View **diff-able** so a verified mutating shim can relate its
post-state to the abstraction. Concretely, ONE of:

- Replace the argument-free `uninterp spec fn phys_view() -> PhysMemView` with a
  **state-indexed** abstraction (pre/post view pair) and thread `old`/`new` through
  `instance()` and the `Inner::*` mutator contracts, so `frame::alloc` can state
  `new_view().frames.allocated_frames.contains(frame@) && refcounts[frame@]==1`
  against the *post* view; **or**
- Introduce a **tracked ghost ownership/permission token** for the singleton
  (standard Verus pattern for global mutable state) that `instance()` yields and
  the `Inner::*` mutators consume-and-produce, carrying the
  `old(self)@ → final(self)@` transition (which `Inner::*` already specify) up to
  the shim.

The existing `PhysMemView::spec_initialize` / `spec_book_frame` / `spec_book_frames`
helpers already model per-operation deltas declaratively; the missing piece is the
**state-threading mechanism** connecting them to `phys_view()` across a mutation.
This edits `mod.spec.rs` (`phys_view()` / `PhysMemView`) and the frozen `Inner::*`
`#[verus_spec]` transition contracts — both view-design artifacts, outside the
specification phase's editable surface. view_design.md's "post-state expressible"
claim (line 205) must be reconciled with this requirement.

Also fix the downstream consequence: with a diff-able view, the strong guarantee
no longer needs to be asserted as `external_body` axioms in `manager.rs`
(alloc_user_frame / alloc_kernel_frame / alloc_many_user_frames), which are
currently not in `tcb-allowed.md`.

## Evidence

- In-tree, strong spec restored on `frame::alloc`: `error: postcondition not
  satisfied` at `frame.rs:751`, cascade into `Upool::alloc` (manager.rs:268),
  `31 verified, 2 errors` (exit 101). Reverted.
- `reproducers/02_goal_is_false.rs`                 → `1 verified, 0 errors` (post-state shim spec's NEGATION provable ⇒ spec is false).
- `reproducers/01_shim_fails.rs`                    → `0 verified, 1 errors` (faithful isolated model fails).
- `reproducers/03_strengthening_derives_false.rs`   → `1 verified, 0 errors` (post-state `instance()` bridge ⇒ false; unsound).
- `caller_analysis.md`: alloc/book/alloc_range/share callers require post-state
  `allocated_frames` membership + `refcount==1`; the weakened pre-state shims
  "would break callers".
- `tcb-allowed.md` does NOT list `manager::alloc_user_frame` /
  `alloc_kernel_frame` / `alloc_many_user_frames`, contradicting `bugs.md`.
- `make verify-kernel` (current, weakened): exit 0,
  `assume=0 external_body=22 admit=1 cfg_gate=9`.

All reproducers re-executed independently this turn with the project-pinned Verus
at `/home/ruize/verus-bin/verus`.