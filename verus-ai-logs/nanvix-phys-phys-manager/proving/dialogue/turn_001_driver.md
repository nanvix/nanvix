## Turn 1: Full checklist walk-through (proving phase, phys-manager)

### Progress
- Done (PASS):
  1. No specs weakened (spec-drift)
  2. Zero remaining `admit()`
  3. Zero `external_body` outside `tcb-allowed.md`
  4. Zero `assume` / `assume_specification` outside std/external trust boundaries
  5. No cfg-gated exec code (branches/expressions/match arms)
  6. Cheating audit (counts + locations reported)
  7. Verus-limitation reproducer requirement (N/A — no new limitation claims)
  8. Exec rewrites minimal & semantically equivalent
  9. Cross-module regression (`make verify-kernel` exit 0)
- Current: **10. Verification + build (0 errors, 0 warnings) — FAIL**
- Remaining: none (all other items verified PASS)

---

### Verification (per item)

Scope = manager module files:
`src/kernel/src/mm/phys/{manager.rs, manager.spec.rs, manager.proof.rs}`.
Tool of record: `make verify-kernel` → **verification exit 0** (cached, 0 errors).
Cheating scan: `assume=0 external_body=18 admit=3 cfg_gate=12` (whole-crate, see below).

**1. No specs weakened — PASS.**
Diffed the spec surface across the entire proving phase:
- `git diff d1fd9adcb..HEAD -- manager.spec.rs` → **empty** (no change to any
  `spec fn`, `open spec`, `inv`, view, or helper since specification END).
- `git diff d1fd9adcb..HEAD -- manager.rs` → all hunks are inside `proof!` /
  `proof_decl!` blocks, loop `invariant` clauses, and one exec body re-order
  (see item 8). Grepping the diff for `requires|ensures|verus_spec` shows **no
  `requires`/`ensures` contract line changed** — the additions
  (`g_old.all_free(...)`, `self@ == g_old.book_all(...)`) are *loop invariants*,
  not function postconditions. No guarantee weakened.

**2. Zero `admit()` — PASS (manager).**
`cheating-detail.txt`: the 3 `admit`s are `mm/virt/identity_map.rs:{533,627,718}`
— a different module (`mm::virt`), out of the phys-manager proof target. Zero
`admit` in any `manager.*` file (`grep -n admit manager*.rs` → none in fn bodies).

**3. `external_body` only if in `tcb-allowed.md` — PASS.**
Six `external_body` in manager files, each individually checked against
`verus-ai-logs/tcb-allowed.md`:
- `manager.rs:104 PhysMemoryManager::init` — listed (§ cross-module, `static mut`
  singleton bring-up).
- `manager.rs:536 kernel_watermark` — listed (§ external-bottom build-time const).
- `manager.proof.rs:24 lemma_manager_attached` — listed (§8 ghost-token).
- `manager.proof.rs:39 lemma_kernel_alloc_one` — listed (§8 ghost-token).
- `manager.proof.rs:52 lemma_kernel_alloc_contiguous` — listed (§8 ghost-token).
- `manager.proof.rs:165 lemma_user_bulk_err_restored` — listed (§8 ghost-token).
All six accounted for; none unlisted.

**4. Zero `assume` / `assume_specification` outside trust boundaries — PASS.**
`assume(...)` count = 0 (scan). Three `assume_specification` in `manager.spec.rs`:
`Result::and_then`, `Result::inspect_err`, `Vec::capacity` — all `core`/`std`
items for which vstd ships no spec. These are exactly the sanctioned
external-bottom std-boundary form; each carries a justifying comment.

**5. No cfg-gated exec code — PASS.**
`count_cfg_gates` over the three manager files = 3, and all three are
`#[cfg_attr(verus_keep_ghost, verus_spec(invariant ...))]` attaching **loop
invariants** (lines 234, 472, 491) — spec annotations, not exec
branches/expressions/match arms (the loop body is identical in both configs).
Same idiom is used by the already-verified `frame.rs` (3 occurrences). The
`#[cfg(not(verus_keep_ghost))] error!/warn!` lines are log statements, explicitly
excluded by the counter and not exec-logic gates.

**6. Cheating audit (counts + locations) — reported.**
Manager-module totals: `admit=0`, `external_body=6` (all in tcb-allowed, item 3),
`assume=0`, `assume_specification=3` (std boundaries, item 4),
`cfg_gate=3` (loop-invariant spec attrs, item 5). Whole-crate scan reports
`admit=3` (all `mm/virt`), `external_body=18`, `cfg_gate=12` — none attributable
to an un-sanctioned construct in the manager module.

**7. Verus-limitation reproducer — N/A / PASS.**
The proving phase introduces **no new "Verus cannot do X" limitation claim** in
the manager module. The `external_body` items are documented *trust boundaries*
(ghost-token / build-time const / `static mut`) in `tcb-allowed.md`, not
construct-level limitations. Nothing requires an isolated reproducer here.

**8. Exec rewrites minimal & semantically equivalent — PASS.**
One exec change in `check_user_watermark` (`// VERUS DEVIATION`, manager.rs:335):
hoist `let free = frame::free_count();` above the `checked_add` threshold compute,
then compare `free < watermark_threshold`. `frame::free_count()` is a pure read
(no side effects, same value); the only behavioral difference is that on the rare
`checked_add` overflow path it is now read once unconditionally (O(1)). Comment
documents this. Semantically equivalent and minimal. No other exec logic changed
(loop bodies, error paths, push/clear all preserved per the diff).

**9. Cross-module regression — PASS.**
`make verify-kernel` → exit 0, verification cached, **0 errors** across all
verified kernel modules (not just manager). No module regressed.

**10. Verification + build, 0 errors / 0 warnings — FAIL.**
- Verus side: `make verify-kernel` → 0 errors. OK.
- Build side: `./z build -- all` → **compile error in the kernel crate**:

```
error: unused variable: `i`
   --> src/kernel/src/mm/phys/manager.rs:245:13
    |
245 |         for i in 0..count {
    |             ^ help: ... prefix it with an underscore: `_i`
    = note: `-D unused-variables` implied by `-D warnings`
error: could not compile `kernel` (bin "kernel") due to 1 previous error
```

Root cause: the proving phase changed `for _ in 0..count` → `for i in 0..count`
in `alloc_many_user_frames` so the loop **invariant** can reference `i`
(`frames@.len() == i`, `user_addr_set(frames@).len() == i`). Under
`verus_keep_ghost`, `i` is consumed by the invariant, so Verus is happy. Under
the normal build the invariant is cfg-gated away, leaving `i` genuinely unused;
with `-D warnings` this is a hard error. So the proving change verifies but
breaks the production build → the "0 warnings" requirement is **not** met.

---

### Fix Request (item 10)

Make the normal (`verus_keep_ghost`-off) build compile cleanly with `-D warnings`
while keeping the Verus loop invariant intact in `alloc_many_user_frames`
(`src/kernel/src/mm/phys/manager.rs`, the `for i in 0..count` loop at line ~245).

The loop index `i` must stay named (the invariant needs it under Verus) but is
unused in exec. Apply the repo's sanctioned idiom for a binding consumed only by
spec — `#[allow(unused_variables)]` — to that loop. Add it alongside the existing
`#[cfg_attr(verus_keep_ghost, verus_spec(invariant ...))]` attribute on the loop,
e.g.:

```rust
#[allow(unused_variables)]
#[cfg_attr(verus_keep_ghost, verus_spec(
    invariant
        g_old == old(self)@,
        ...
        self@ == g_old.book_all(user_addr_set(frames@)),
))]
for i in 0..count {
```

(Precedent: `src/kernel/src/mm/kheap.rs:161` uses `#[allow(unused_variables)]` for
a binding used only by the Verus layer. Do **not** rename to `_i` — the invariant
references `i`. Do **not** delete/weaken any invariant clause. Do **not** wrap the
loop in a `#[cfg(...)]` exec gate — that would violate item 5.)

Constraints on the fix:
- No spec/invariant weakening (item 1 must stay PASS).
- No new cfg-gated exec code (item 5 must stay PASS).

Verify with BOTH, and paste the tails:
1. `make verify-kernel` → must stay 0 errors.
2. `./z build -- all` → must reach `Finished` for the `kernel` crate with **0
   warnings / 0 errors** (the `unused variable: \`i\`` error gone). If any other
   `-D warnings` diagnostic appears after this one (the build aborts at the first),
   fix those too and re-run until the kernel crate builds clean.

Justification is not a fix — change the code and paste both tool outputs.

---

### STOP status
Not creating STOP: checklist item 10 is FAIL (build breaks under `-D warnings`).
Fixer must address the unused-`i` build error and re-verify.
