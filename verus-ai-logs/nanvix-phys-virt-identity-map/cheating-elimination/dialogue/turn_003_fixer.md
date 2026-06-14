## Response to cheating_report_2 (admit: 3)

### Status of the flagged items

`cheating_report_2.md` flags exactly the 3 `admit()`s at `identity_map.rs:534`
(`ensure_pt`), `:632` (`ensure_pte`), `:719` (`identity_map_page`). The `external_body`
flagged last turn (`ExPageTableBss`) is **gone** — `external_body` count stays 0 in this
module. No `assume`, `trusted`, `limitation_assume`, or
`exec_allows_no_decreases_clause` exist here (the report's boilerplate mentions of those
are 0 in this module: `assume=0 trusted=0 no_decreases=0`).

### What I re-investigated this turn (per the escalation ladder)

I did **not** assume the prior blocker; I re-derived it from the dependency specs.

**1. `ensure_pt` — sole hard blocker is `bump_view(self).inv()` (not a view contradiction).**
`ensure_pt`'s contract has NO result-dependent contradiction: both `Ok`/`Err` only
require `identity_map_view().inv()`, which is given as a precondition over the fixed
uninterp view, so it re-derives trivially. The blocker is the `alloc_as` precondition
`bump_view(self).inv()` (bump_allocator `lib.rs:350`). I exhaustively searched the
bump_allocator spec/proof crate:
- `bump_view` is `uninterp` (`lib.spec.rs:177`) with **no** establishing axiom.
- **Every** proof lemma in `lib.proof.rs` (`lemma_geometry:49`, `lemma_exhausted_boundary:107`,
  `lemma_alloc_transition:121`) **requires** `v.inv()` as a *precondition* — none
  *produces* it.
- `FixedSizeBumpAllocator::new()` (`lib.rs:243`) has **no** `ensures`.
- The spec file's own header (`lib.spec.rs:12-17`) states view-attachment "requires an
  atomic-ghost / PointsTo token … Until then `BumpView` is referenced by the proof
  lemmas" — i.e. explicitly deferred to the proving phase.

There is therefore **no in-scope way** to obtain `bump_view(PAGE_TABLE_ALLOCATOR).inv()`.
Asserting it would be a disguised `assume`. `bump_allocator` is out of scope.

**2. `ensure_pte` / `identity_map_page` — result-dependent opposite postconditions over a
parameterless uninterp constant (mathematically unsatisfiable).**
`identity_map_view()` is `uninterp spec fn …() -> IdentityMapView` (`spec.rs:36`): one
fixed value `V`. With `accessible(p) = !initialized || mapped.contains(p)` (`spec.rs:77`):
- `ensure_pte`: `Ok ⇒ V.mapped.contains(p)` AND `Err ⇒ !V.mapped.contains(p)`.
- `identity_map_page`: `Ok ⇒ V.accessible(p)` AND `Err ⇒ !V.accessible(p)`.

For one fixed `V`, `contains(p)`/`accessible(p)` has a single truth value, but the exec
return (PTE-present check, frame-range, atomic `pd_paddr == 0`) is independent of `V`, and
**both exits are reachable** (alloc can genuinely return `OutOfMemory`, reads can fail).
So exactly one branch reduces to `⇒ false`. No constant `V` (incl. concrete
`{initialized:false, mapped:∅}`, tested previously → same 2 postcondition errors) and no
lemma can satisfy both. The `pd_paddr==0 ⇔ !initialized` link is unestablishable because
`AtomicUsize::load()`'s result is not spec-readable (vstd: "no support for reasoning about
the values inside the atomics"); recovering it needs a ghost atomic — an exec **type
change** to the `static KERNEL_PD_PADDR` used by out-of-scope callers (`init:140`,
`sync_kernel_pdes:290`, the `test` module).

**3. The read-index preconditions (`:534`/`:631`, `index@ < PAGE_TABLE_LENGTH`).**
`use_type_invariant(pde_idx)` → `Verus Internal Error: missing type invariant function`
(TableIndex's invariant is cross-crate-unusable). Adding `requires …@ < PAGE_TABLE_LENGTH`
is a contract change that forces the out-of-scope caller `init` to prove it — forbidden.
Moot anyway: even discharged, the `bump_view`/view blockers remain.

### Why this cannot be eliminated in scope

Sound elimination requires the cross-module **proving-phase ghost-token framework**
(replace the `uninterp` `identity_map_view()`/`bump_view()` with a `Tracked`/`PointsTo`
token threaded through out-of-scope callers `init`/`KernelFrame::new`, and/or a sound
contents postcondition + permission token on `arch::…::Table::write`). Every realization
is forbidden by the task hard rules: no exec signature/type change, no spec weakening, no
new `external_body` outside `tcb-allowed.md`, no new `assume_specification`/`axiom`,
`table.rs` and the callers off-limits. This is the identical deferral every sibling
`mm::phys::*` singleton free function uses and that bump_allocator's own view-attachment
uses. Independently re-confirmed by the reviewer's `STOP` (`BLOCKED`) adjudication.

### Verification (current committed state)

```
make verify-kernel  → exit 0
  verification: 76 verified, 0 errors
  cheating: assume=0 external_body=11 admit=31 trusted=0 no_decreases=0 cfg_gate=15
  module mm::virt::identity_map: 9 verified, 0 errors; external_body 0
```
Exec AST byte-identical to base (`ast_consistency.py` → `Consistent: 14 functions,
1 structs match`). The only diff vs base is the 3 ghost `proof! { admit(); }` lines
(stripped before hashing).

### Result: NEEDS_GUIDANCE

`external_body` is FIXED (1 → 0, prior turn). The 3 `admit()`s are a re-verified
impossibility under the module-scoped hard rules — escalated to the proving-phase /
cross-module owner. Honest hand-off in `verification_todo.md`. Removing them yields a
failing build (`6 verified, 3 errors`, exit 101), which is strictly worse than the
documented, verifying baseline.
