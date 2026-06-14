## Turn 2: Zero admit() remaining — adjudication (independently verified)

### Progress
- Done / PASS (re-confirmed this turn, unchanged):
  - Zero assume() — PASS (`assume=0`).
  - Zero trusted — PASS (`trusted=0`).
  - Zero exec_allows_no_decreases_clause — PASS (`no_decreases=0`).
  - Zero cfg-gated exec code — PASS (only `include!` ghost imports + `error!` logging are cfg-gated).
- Current: **Zero admit() remaining** — **FAIL, but blocked** (see Adjudication). 3 admits at
  `identity_map.rs:534` (`ensure_pt`), `:632` (`ensure_pte`), `:719` (`identity_map_page`).
- Remaining (cannot be cleared while the admits stand / depend on real proofs):
  - external_body in TCB (ExPageTableBss + `as_ptr`/`FixedSizeBumpAllocator::new` assume_specs — still unlisted).
  - AST consistency, exec-rewrite comments, spec-drift, cross-module regression, final 0-error/0-warning build.

### Verification (I did NOT trust the fixer's claims — I reproduced everything)

1. **Reproduced admit removal myself.** Deleted all three `proof! { admit(); }`, ran
   `make verify-kernel MODULE=mm::virt::identity_map`:
   ```
   error: precondition not satisfied   identity_map.rs:534:44  (pd.read(pde_idx))   <- table.rs:205 index@ < PAGE_TABLE_LENGTH
   error: precondition not satisfied   identity_map.rs:549:9   (alloc_as)          <- bump_allocator inv()
   error: postcondition not satisfied  identity_map.rs:618     (ensure_pte Ok: mapped.contains(p))
   error: precondition not satisfied   identity_map.rs:631:40  (pt.read(pte_idx))  <- table.rs:205
   error: postcondition not satisfied  identity_map.rs:706     (identity_map_page Ok/Err: accessible(p), ×3 exits)
   verification results:: 6 verified, 3 errors   (exit 101)
   ```
   The fixer's "6 verified, 3 errors" report is accurate.

2. **Read the dependency contracts directly** (not taken on faith):
   - `arch/.../table.rs:101-134` — `pd_index`/`pt_index` DO `ensures result@ < PAGE_TABLE_LENGTH`.
     But `ensure_pt(pd, pde_idx)` / `ensure_pte(pt, pte_idx, _)` receive the index as an opaque
     `TableIndex` **parameter with no `requires`**, so the bound does not flow in. Recovering it
     needs either (a) a new `requires …@ < PAGE_TABLE_LENGTH` (a contract change), or (b) the
     `TableIndex` type invariant, which is cross-crate-unusable here (`Verus Internal Error:
     missing type invariant function`).
   - `arch/.../table.rs:241-250` — `Table::write` is `external_body` with **only** `requires
     index@ < PAGE_TABLE_LENGTH` and **no contents `ensures`**. The inline comment (and
     `tcb-allowed.md`) document that a contents postcondition pinning `spec_table_word` to the
     written `entry` would be **unsound** for an assumed `external_body` (two distinct writes to one
     slot ⇒ `e1 == e2` ⇒ `false`). So a `write` cannot establish that the slot/page became present.

3. **Confirmed the postcondition impossibility structurally** (independent of the fixer's prose).
   `identity_map_view()` is a parameterless `uninterp spec fn` ⇒ one fixed value `V` everywhere in a
   proof context. The contracts demand a result-dependent pair over that single `V`:
   - `ensure_pte`: `Ok ⇒ V.mapped.contains(p)` **and** `Err ⇒ !V.mapped.contains(p)`.
   - `identity_map_page`: `Ok ⇒ V.accessible(p)` **and** `Err ⇒ !V.accessible(p)`.
   For a fixed `V`, `contains(p)`/`accessible(p)` has one truth value, while the exec return
   decision (PTE-present check, frame-range, `pd_paddr == 0`) is unrelated to `V`. So exactly one
   clause reduces to `⇒ false`. There is no `old()` to express a pre/post transition (these are free
   functions with no `&mut`), and `write` has no contents `ensures`, so nothing links the exec state
   to `V.mapped`. The fixer's concrete-view experiment (`V = {initialized:false, mapped:∅}`) failed
   the exact same two postconditions — matching my analysis.

### Adjudication: BLOCKED (not a justification — a verified impossibility under the task constraints)

This is **not** the fixer hand-waving; I reproduced every failure and verified the impossibility from
the spec structure myself. Reaching `admit=0` for these three functions requires at least one of:

1. A spec-readable model of the singleton state — replace the `uninterp` `identity_map_view()` /
   `bump_view()` with a ghost token (`Tracked`/`PointsTo`/atomic-ghost) so the view can transition
   `v -> v'` across `write`. This changes module statics (`KERNEL_PD_PADDR`, `PAGE_TABLE_ALLOCATOR`)
   and threads a token through **out-of-scope callers** (`init`, `KernelFrame::new`).
2. A sound `old@ -> @` contents postcondition + permission token on `arch::…::Table::write`, which
   `tcb-allowed.md` documents as deliberately deferred *because it is unsound for an `external_body`*.
   `table.rs` is out of scope.
3. Adding `requires …@ < PAGE_TABLE_LENGTH` to `ensure_pt`/`ensure_pte` (fixes only the 534/631 read
   preconditions, not 549/618/706) — a contract change.

All three are forbidden by this task's scope (only the `identity_map.*` files; callers `init` /
`KernelFrame::new` and `arch/table.rs` must not be touched) and by the Turn-1 constraints (no
spec weakening, no `assume`, no unlisted `external_body`, no signature change). This is the same
proving-phase ghost-token deferral every sibling `mm::phys::*` singleton free function uses (frame.rs
`alloc`/`free`/… are all `admit`-ed), so it is a cross-module phase boundary, not a local proof gap.

**Repo state restored:** my reproduction run was auto-committed by the verify harness as a failing
state (`a884c22a8`). I restored the admits-present, verifying baseline
(`git checkout 55b6ab858 -- identity_map.rs`; `make verify-kernel` ⇒ exit 0, CHEATING_DETECTED).

### Fix Request (deferred to the proving phase — escalation, not local action)
The admit elimination for `ensure_pt` / `ensure_pte` / `identity_map_page` must be done together with
the cross-module proving-phase ghost-token work (items 1–2 above). It cannot be completed inside the
`mm::virt::identity_map` module alone without weakening contracts or introducing new cheating.
Escalate to the proving-phase / cross-module owner. Until then this checklist item remains FAIL.
