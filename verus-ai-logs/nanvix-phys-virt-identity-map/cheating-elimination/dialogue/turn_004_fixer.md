## Response to cheating_report_3 (admit: 3)

### Flagged items

`cheating_report_3.md` flags the same 3 `admit()`s: `identity_map.rs:534` (`ensure_pt`),
`:632` (`ensure_pte`), `:719` (`identity_map_page`). No `assume`, `trusted`,
`external_body`, `limitation_assume`, or `exec_allows_no_decreases_clause` exist in this
module (`assume=0 trusted=0 external_body=0 no_decreases=0` for `mm::virt::identity_map`).

### New work this turn (did not rely on prior conclusions)

**1. Checked whether a sibling module already solved this pattern — it has not.**
`mm::phys` carries the identical singleton-global-view deferral with admits still in
place (`frame.rs`: 14, `manager.proof.rs`: 8, `mod.proof.rs`: 2). The cross-module
ghost-token proving phase has not been done anywhere in the crate; these three admits
are the same project-wide deferral, not a local omission.

**2. Re-derived the blocker from the current dependency specs (fresh reproduction).**
Deleting all 3 admits → `make verify-kernel MODULE=mm::virt::identity_map` →
`6 verified, 3 errors` (7 obligations): `:534`/`:631` read index bounds, `:549`
`alloc_as` needs `bump_view(self).inv()`, `:618` `ensure_pte` `Ok` postcondition, `:706`
×3 `identity_map_page` postconditions. Unchanged against current arch specs.

**3. Attempted the read-bound discharge via the type invariant — hit a verifier bug.**
`TableIndex::inv` is `#[verifier::type_invariant]` (`arch/.../table.spec.rs:24`).
Adding `proof! { use_type_invariant(pde_idx); }` in `ensure_pt`:
```
error: Verus Internal Error: missing type invariant function
   --> src/kernel/src/mm/virt/identity_map.rs:534:33
```
The type invariant lives in the `arch` crate and cannot be invoked from the `kernel`
crate — a cross-crate `use_type_invariant` limitation. (Even if it worked, only the read
bounds clear; `:549` and the postconditions remain.)

**4. Checked whether the `Err` branches are dead (which would make their postconditions
vacuous) — they are reachable.** `Table::read` returns `Option<E>` pinned to the opaque
`spec_table_read(self@.addr, index@)` (`table.rs:207`), so it may be `None`;
`FrameNumber::from_raw_value` returns `None` out of range. Neither failure exit can be
proven dead.

**5. Proved no concrete view value can satisfy the contracts (exhaustive over polarity).**
`ensure_pte`: `Ok ⇒ V.mapped.contains(p)` AND `Err ⇒ !V.mapped.contains(p)`;
`identity_map_page`: `Ok ⇒ V.accessible(p)` AND `Err ⇒ !V.accessible(p)`. `identity_map_view()`
is `uninterp` (one fixed `V`). With `V.mapped = ∅`: `ensure_pte`-`Err` and
`identity_map_page`-`Ok` verify, their opposites become `⇒ false`. With `V.mapped =`
all-aligned + `initialized=true`: the polarity flips. Every constant `V` leaves exactly
one branch per function `⇒ false`, and both branches are reachable ⇒ unsatisfiable for
any constant. The only resolution is a **state-dependent** view, i.e. a ghost token over
the `KERNEL_PD_PADDR`/`KERNEL_CR3` atomics and the BSS pool.

### Why elimination is out of scope (unchanged, re-verified)

A sound state-dependent view requires either (a) replacing
`core::sync::atomic::AtomicUsize`/`AtomicU32` statics with `vstd` atomic-ghost/`PointsTo`
and threading a `Tracked` token through the out-of-scope callers `init`,
`sync_kernel_pdes`, `KernelFrame::new`; or (b) a trusted accessor / `assume_specification`
tying `AtomicUsize::load` and `bump_view` to the view — forbidden (unlisted in
`tcb-allowed.md`; human-approval-only; `tcb-allowed.md` records the `Table::write`
contents postcondition as deliberately-deferred-because-unsound). `bump_allocator`'s own
`bump_view` is `uninterp` and its `lib.proof.rs` lemmas all *require* `inv()` (none
produces it); view-attachment is explicitly deferred in `lib.spec.rs:12-17`. All of this
is forbidden by the task hard rules (no exec signature/type change, no spec weakening, no
new `external_body`/`assume_specification`, `table.rs`/callers off-limits).

This matches the reviewer's own independently-reproduced `STOP` (`BLOCKED`) adjudication
in `dialogue/turn_002_driver.md`.

### Verification (current committed baseline)

```
make verify-kernel  → exit 0
  verification: 76 verified, 0 errors
  cheating: assume=0 external_body=11 admit=31 trusted=0 no_decreases=0 cfg_gate=15
  module mm::virt::identity_map: 9 verified, 0 errors
```
Exec AST byte-identical to base (`ast_consistency.py` → `Consistent: 14 functions,
1 structs match`). Removing the admits yields a strictly worse failing state
(`6 verified, 3 errors`, exit 101).

### Result: NEEDS_GUIDANCE

The 3 `admit()`s are an exhaustively-evidenced impossibility under the module-scoped hard
rules (verified again this turn, incl. the cross-crate `use_type_invariant` verifier
bug). Escalated to the proving-phase / cross-module owner; honest hand-off updated in
`verification_todo.md`. I will not introduce unsound assumptions to force a green gate.
