# Verification TODO — `mm::virt::identity_map` (cheating-elimination pass)

Honest hand-off of genuinely-stuck proof obligations per the `verus-constraints`
escalation ladder. These are **not** accepted trust assumptions and **not** code
bugs. They are the cross-module *proving-phase ghost-token* obligations the whole
bottom-up effort defers (the same state every sibling `mm::phys::*` singleton free
function is in).

The 3 in-scope exec functions (`ensure_pt`, `ensure_pte`, `identity_map_page`)
retain `proof! { admit(); }`. Every obligation below was reproduced empirically in
this pass by deleting the `admit()` and running
`make verify-kernel MODULE=mm::virt::identity_map`
(result: `6 verified, 3 errors`, exit 101). The exec code is byte-identical to
`verus-ai-prove-bottom-up` except for those 3 ghost `admit()` lines.

## Root cause (shared by all 3 entries) — a logical impossibility, not a missing lemma

`identity_map_view()` is declared `uninterp spec fn identity_map_view() -> IdentityMapView`
(`identity_map.spec.rs:36`). With **no parameters** it denotes one fixed-but-opaque
constant value `V`; `requires` only tells the verifier `V.inv()`. Its connection to
the concrete singleton state (`KERNEL_PD_PADDR`/`KERNEL_CR3` + the BSS page-table
pool) is — by the file's own design comments — "realized in the proving phase by a
ghost token over those singletons." That framework does not exist in the codebase.

Two of the three functions carry **result-dependent, opposite** postconditions over
this constant:

- `ensure_pte`: `Ok => V.mapped.contains(p)` **and** `Err => !V.mapped.contains(p)`.
- `identity_map_page`: `Ok => V.accessible(p)` **and** `Err => !V.accessible(p)`.

For a single fixed unknown `V`, exactly one of `contains(p)` / `!contains(p)` is true,
but the exec **return decision** (PTE read success, frame-number range, `pd_paddr==0`
init test) is not tied to that fixed set. Therefore **no proof, lemma, or `vstd`
search can discharge both branches** — it is logically impossible while the view is a
parameterless constant. Confirmed empirically (postcondition errors at
`identity_map.rs:618` and `identity_map.rs:706`).

Eliminating the admits soundly requires making the view depend on runtime singleton
state via a ghost token. Every realization of that token is **out of scope / forbidden**
by this task's hard rules:

1. Spec-readable atomics would require replacing `core::sync::atomic::AtomicUsize`
   with a `vstd` atomic-ghost / `PointsTo` type — an exec **type/signature change** to
   module statics and to out-of-scope callers (`init`, `boot_init`, `memset`/`memcpy`).
2. A trusted `external_body` accessor (the `mm::phys::frame::instance()` pattern) —
   forbidden: `external_body` is allowed only for functions listed in
   `verus-ai-logs/tcb-allowed.md`, and none of these 3 functions is listed.
3. An `assume_specification` tying `AtomicUsize::load` (or `Table::write`) to the
   view — forbidden: `assume_specification`/`axiom` are external-bottom and
   human-approved only. `tcb-allowed.md` additionally records that a contents
   postcondition on the `external_body` `Table::write` was shown **unsound** and is
   deliberately deferred; `table.rs` is also out of scope.

## Entries (with the exact Verus error reproduced this pass)

### `ensure_pt` (src/kernel/src/mm/virt/identity_map.rs:533) — UNPROVEN
- `error: precondition not satisfied` at `:534` — `pd.read(pde_idx)` needs
  `pde_idx@ < PAGE_TABLE_LENGTH`. *Dischargeable* via `use_type_invariant(pde_idx)`
  (TableIndex type invariant), but does not by itself unblock the function.
- `error: precondition not satisfied` at `:549` — `PAGE_TABLE_ALLOCATOR.alloc_as::<…>()`
  needs `bump_view(self).inv()`. `bump_view` is `uninterp` over a module-level
  `static`; its `inv()` cannot be established without the ghost token. **BLOCKER.**
- postcondition `Ok(pt_paddr) => spec_is_page_aligned(pt_paddr as int)`: the
  present-PDE path (`pde.frame_address()`) is provable; the fresh-allocation path
  (`slot.as_ptr() as usize`) is not — `<[T]>::as_ptr` has an empty contract and is
  not linked to `slot_ref_addr` alignment; tightening it universally is unsound.
  **BLOCKER.**

### `ensure_pte` (src/kernel/src/mm/virt/identity_map.rs:627) — UNPROVEN
- `error: precondition not satisfied` at `:631` — `pt.read(pte_idx)` needs
  `pte_idx@ < PAGE_TABLE_LENGTH` (dischargeable via `use_type_invariant`).
- `error: postcondition not satisfied` at `:618` —
  `Ok => identity_map_view().mapped.contains(spec_page_base(phys_addr))` (and the
  symmetric `Err` clause). Opaque, result-dependent global-view fact. **BLOCKER.**

### `identity_map_page` (src/kernel/src/mm/virt/identity_map.rs:718) — UNPROVEN
- `error: postcondition not satisfied` at `:706` (multiple instances) —
  `Ok => identity_map_view().accessible(phys_addr@)` (and symmetric `Err`). No spec
  link from the exec `pd_paddr == 0` test to `identity_map_view().initialized`, nor
  from `ensure_pt`/`ensure_pte` to `mapped`. **BLOCKER.**

## Not a bug

No overflow / off-by-one / missing-bound / unchecked-cast issue exists in the three
functions. All blockers are spec-framework deferrals (the parameterless-constant
view + the deferred ghost-token realization), so nothing was recorded in `bugs.md`.

## Escalation ladder followed

1. Searched `vstd` (atomics, layout, sets) and the verified dependency specs
   (`bump_allocator`, `arch::…::paging::table`). No existing spec links a
   parameterless `uninterp` view to interior-mutable singleton state.
2. Built the minimal reproducer empirically (deleted the 3 `admit()`s; captured the
   3 errors above).
3. Tried equivalent rewrites: a constant `open spec` definition of the view
   (`initialized:false, mapped:empty`) satisfies the `Ok` clause of one branch but
   makes the opposite `Err` clause `false` — proving the impossibility is intrinsic,
   not a proof-engineering gap. Reverted (would also gut the spec dishonestly).
