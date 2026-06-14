## Turn 2: Zero `admit()` — independently re-verified; genuine cross-module BLOCKER

### Progress
- Done (PASS / re-confirmed; source byte-identical to phase start, `git diff <phase-start> -- manager.{rs,spec.rs,proof.rs}` empty):
  - [x] Zero `assume()`
  - [x] Zero trusted
  - [x] Zero `exec_allows_no_decreases_clause`
  - [x] cfg-gated exec = logging only (9 sites)
  - [x] No unlisted `external_body` (`init`, `kernel_watermark` both in `tcb-allowed.md`)
  - [x] AST consistency (matched=8, mismatched=0)
  - [x] Exec rewrites comment+reproducer (vacuous — 0 exec rewrites)
  - [x] Each `external_body` listed
  - [x] No specs weakened (`spec_drift.py` = 0 on all 3 files)
  - [x] Cross-module regression (`verify-kernel` all modules, exit 0)
  - [x] Verification/build exit 0
- Current: **[ ] Zero `admit()` — FAIL (4 remain), but proven unfixable in-scope → BLOCKER**
- Remaining: none uninspected.

### Verification

The fixer made **no source change** (diff empty) and returned `NEEDS_GUIDANCE`,
arguing the 4 admits cannot be removed in-scope. Per the rules I did **not** accept
this on its word — I independently reproduced all three of its claims.

**Claim A (unprovable in-scope).** I removed all 4 `admit()`s from
`manager.proof.rs` and ran `make verify-kernel MODULE=mm::phys::manager`:
```
error: postcondition not satisfied  manager.proof.rs:14:9   (m@ == phys_view().frames)
error: postcondition not satisfied  manager.proof.rs:31:9   (pre.free_frames.contains(addr))
error: postcondition not satisfied  manager.proof.rs:32:9   (post == pre.alloc_one(addr))
error: postcondition not satisfied  manager.proof.rs:49:9   (pre.all_free(kernel_addr_set(frames)))
error: postcondition not satisfied  manager.proof.rs:50:9   (post == pre.book_all(kernel_addr_set(frames)))
error: postcondition not satisfied  manager.proof.rs:214:9  (m@ == pre)
verification results:: 15 verified, 4 errors
```
Exactly the 4 targeted lemmas fail, no collateral. File restored to baseline
(4 admits, `git diff` empty). **Confirmed.**

**Claim B (external_body conversion is UNSOUND — the tcb-boundary fallback I
proposed in Turn 1).** I ran the committed reproducers with standalone Verus:
```
reproducers/alloc_one_unsound.rs   → 1 verified, 0 errors   (exploit() ensures false VERIFIES)
reproducers/others_unsound.rs      → 1 verified, 0 errors   (exploit ensures false VERIFIES)
reproducers/alloc_one_realbody.rs  → 0 verified, 1 errors   (real body unprovable)
```
This is decisive and I verify it by signature too: `lemma_kernel_alloc_one(pre,
post, addr: int)` has only `requires pre.wf()` yet `ensures
pre.free_frames.contains(addr)` for an **arbitrary** `addr` — not a theorem. As an
`external_body` axiom it lets any caller derive `false`. Same for
`lemma_user_bulk_err_restored` (`ensures m@ == pre` for arbitrary `pre`) and
`lemma_kernel_alloc_contiguous`. So my Turn-1 fallback ("convert to a
tcb-allowed boundary") is **unsafe here** — it would replace a locally-gated
`admit()` with a globally-false axiom, strictly worse. I withdraw that fallback
for these three lemmas. Only `lemma_manager_attached` (`m@ == phys_view().frames`,
both `uninterp`) is a sound axiom, but converting it alone cannot reach `admit=0`.

**Claim C (no in-scope token; root cause is the frame layer).** I read the actual
contract of the callee. `frame::alloc()` (frame.rs:758–771) is a **free function
(no `&mut self`)** whose `#[verus_spec]` exports only:
```
Ok(frame) => frame.inv() && phys_view().frames.allocated_frames.contains(frame@)
Err(_)    => phys_view().frames.free_frames.is_empty()
```
It does **not** export the partition step `phys_view'().frames ==
phys_view().frames.alloc_one(frame@)`. Because it takes no `self`, Verus proves
`final(self)@ == old(self)@` across the call, while the wrapper
`alloc_kernel_frame` (manager.rs:363–375) must ensure `final(self)@ ==
old(self)@.alloc_one(kf@)`. The only thing bridging that contradiction is
`lemma_kernel_alloc_one`'s `admit()` (called at manager.rs:391).
`frame::alloc_contiguous` (frame.rs:786–800) likewise exports only base alignment +
range bound. Every token producer (`frame::instance`, `Upool::new`, `init`) is an
`external_body` in the **out-of-scope** frame/upool layer, and `phys_view()` is a
parameter-free `uninterp` constant that cannot express a `v → v'` step. **Confirmed.**

**Baseline restored:** `make verify-kernel` → `42 verified, 0 errors`;
cheating gate `assume=0 external_body=18 admit=24 cfg_gate=15` → `CHEATING_DETECTED`
(the 4 manager admits at proof.rs:16/35/55/216). Spec-drift = 0; AST matched=8/0.

### Determination

The 4 `admit()`s are a **genuine cross-module blocker**, independently proven:
1. unprovable in-scope (15 verified, 4 errors);
2. unsound to convert to `external_body` for 3 of 4 (reproducers verify a proof of
   `false`); and
3. dischargeable only by the out-of-scope `frame` free-function layer
   (`nanvix-phys-phys-frame`, itself carrying 8 admits) exporting (a) a singleton
   attachment token and (b) a versioned partition-step token threaded through
   `frame::alloc`/`alloc_contiguous`/`free`.

No sound in-scope fix exists. Removing the admits = 4 errors; faking them via
`external_body` = unsound; weakening the wrapper postconditions = forbidden
spec-drift. The fixer correctly refused all three.

### Fix Request → escalation

There is no further manager-scope action that is both sound and removes the
admits. The correct sequencing fix is **out of this phase's scope**: verify and
token-instrument `mm::phys::frame` first (target `nanvix-phys-phys-frame`), then
the four manager lemmas discharge mechanically. Marking this item **BLOCKED**
(cross-module dependency), not RESOLVED — `admit=0` is not achievable in-scope and
must not be faked. Reproducers verified and committed under
`cheating-elimination/reproducers/`.
