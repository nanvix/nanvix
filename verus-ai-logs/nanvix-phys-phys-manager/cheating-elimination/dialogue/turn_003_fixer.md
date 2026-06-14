## Response to Turn 3 (`cheating_report_3.md`)

### Report status
`cheating_report_3.md` is content-identical to `cheating_report_1.md` and
`cheating_report_2.md`: external_body 17 (16 on user fns), admit 12 = **8 in
`frame.rs` (out-of-scope `Inner::*` allocator internals)** + **4 in
`manager.proof.rs:16/35/55/216`**. R20p (`exec_allows_no_decreases_clause`),
R20c (`limitation_assume`), `assume`, and `trusted` are all **vacuous** in this
module (grep = 0). All 16 `external_body` are listed in `tcb-allowed.md` (the
task's explicit exception). So the only open item is the 4 manager admits.

### What I changed
No source change. `git diff a8d643993 -- manager.{rs,spec.rs,proof.rs}` is empty.
The change I would have to make to satisfy "zero admit" is **unsound or
out-of-scope**, proven below with a new, sharper root-cause this round.

### New decisive finding this round: the postcondition is *unsatisfiable* in-scope

Prior rounds said the 4 lemmas were "unprovable." This round I pinned the exact
reason, which is stronger — the bridged postconditions are **structurally
unsatisfiable** given the do-not-modify spec.

1. `manager.spec.rs:91` (do-not-modify `View for PhysMemoryManager`):
   ```rust
   closed spec fn view(&self) -> FrameAllocView { self.upool@ }
   ```
   So `self@ == self.upool@`, where `Upool::view` (`upool.rs:59`) is
   `uninterp spec fn view(&self) -> FrameAllocView` — an uninterpreted function
   **of the `upool` field value**.

2. `alloc_kernel_frame` body (`manager.rs:376–395`) calls only
   `frame::alloc()` / `frame::free()` (free functions, no `self`) and
   `KernelFrame::new(..)`. **None touches `self.upool`.** Verus therefore derives
   `self.upool` unchanged ⇒ `final(self)@ == old(self)@`.

3. The target postcondition (`manager.rs:371`) is
   `final(self)@ == old(self)@.alloc_one(kf@)`. Substituting (2), this reduces to
   ```
   old(self)@ == old(self)@.alloc_one(kf@)     // v == v.alloc_one(a)
   ```
   which is **false** (`alloc_one` moves `a` from `free_frames` to
   `allocated_frames`). `lemma_kernel_alloc_one(g_old, self@, kf@)` is invoked
   with `g_old == self@` (both `upool@`, unchanged), so its `ensures post ==
   pre.alloc_one(addr)` is exactly this false proposition. The `admit()` bridges a
   **false** goal — it cannot be turned into a real proof in-scope because the
   fact it would need (the global partition stepped) lives in neither `self.upool`
   nor any exported contract.

   `frame::alloc`'s full contract (`frame.rs:758–768`) exports only membership:
   `Ok(f) => f.inv() && phys_view().frames.allocated_frames.contains(f@)`;
   `Err(_) => phys_view().frames.free_frames.is_empty()`. It does **not** export
   `phys_view'() == phys_view().frames.alloc_one(f@)`, and `phys_view()`
   (`mod.spec.rs:98`) is a **parameter-free `uninterp` constant** — identical in
   every state — so no contract *can* express the step without a tracked token.

   Same shape for `lemma_kernel_alloc_contiguous` (book_all step, `manager.rs:510`)
   and `lemma_user_bulk_err_restored` (`m@ == pre` after a `Vec::clear()` whose
   `Drop`-driven `frame::free` side-effects are unmodeled in exec). And
   `lemma_manager_attached` asserts `m.upool@ == phys_view().frames` — two
   independent `uninterp` functions with no in-scope connecting fact.

### Verification (fresh, this round)

Removing all 4 admits, `make verify-kernel MODULE=mm::phys::manager`:
```
error: postcondition not satisfied  manager.proof.rs:14:9    (m@ == phys_view().frames)
error: postcondition not satisfied  manager.proof.rs:31:9    (pre.free_frames.contains(addr))
error: postcondition not satisfied  manager.proof.rs:32:9    (post == pre.alloc_one(addr))
error: postcondition not satisfied  manager.proof.rs:49:9    (pre.all_free(kernel_addr_set(frames)))
error: postcondition not satisfied  manager.proof.rs:50:9    (post == pre.book_all(kernel_addr_set(frames)))
error: postcondition not satisfied  manager.proof.rs:214:9   (m@ == pre)
verification results:: 15 verified, 4 errors
```
Exactly the 4 targeted lemmas fail, no collateral. Restored.

Baseline (4 admits present): `make verify-kernel MODULE=mm::phys::manager`
→ `19 verified, 0 errors` (exit 0); cheating gate `assume=0 external_body=18
admit=24 trusted=0 no_decreases=0 cfg_gate=15` → `CHEATING_DETECTED`.

Why the three escape hatches are all closed:
- **Remove the admits** → 4 unsatisfiable postconditions (above).
- **`external_body` axiom** → independently reproduced UNSOUND for 3 of 4:
  `reproducers/alloc_one_unsound.rs`, `others_unsound.rs` each verify an
  `exploit() ensures false` (`1 verified, 0 errors`). Also forbidden by the
  directive ("`external_body` on proof fns must be removed").
- **Weaken the wrapper postcondition** → forbidden spec-drift (`spec_drift.py`
  must stay 0), and would silently drop the manager's allocation contract.
- **Fix `frame.rs`** (add a `&mut`-token to `frame::alloc`/`alloc_contiguous`
  and thread a versioned partition-step token; verify the 8 `Inner::*` admits)
  → the structural fix, but `frame.rs` functions are **unlisted**; the hard rule
  "Do not touch unlisted functions" forbids it. `frame` is a separate phase
  (`nanvix-phys-phys-frame`).

### Result: NEEDS_GUIDANCE (cross-module BLOCKER)

The 4 manager admits bridge postconditions that are **unsatisfiable within the
manager's editable scope** because `self@ == self.upool@` and kernel allocations
provably bypass `upool`, while the only state that *does* step (the global
`frame::INSTANCE` partition behind the constant `phys_view()`) is reachable only
through the out-of-scope, still-unverified `frame` free-function layer. No sound
in-scope edit removes them; faking them is unsound (reproduced) or forbidden.
This matches the reviewer's own independent re-verification in
`turn_002_driver.md` (11/12 PASS, 1 BLOCKED) and the `STOP` determination.

Unblock prerequisite (out of this phase): verify + token-instrument
`mm::phys::frame` so `frame::alloc`/`alloc_contiguous`/`free` export a versioned
partition-step token and a singleton-attachment token; the 4 manager lemmas then
discharge mechanically. Recorded in `verification_todo.md`; reproducers under
`cheating-elimination/reproducers/`.
