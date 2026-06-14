# Verification TODO — `mm::virt::identity_map`

Honest hand-off of genuinely-stuck proof obligations per the `verus-constraints`
skill. These are NOT accepted trust assumptions and NOT code bugs — they are the
cross-module **proving-phase ghost-token** obligations that the whole bottom-up
effort defers (identical state to every sibling `mm::phys::*` module, which still
carry exec `admit()`s for the same reason).

Status of this module after the proving pass:
- `make verify-kernel MODULE=mm::virt::identity_map` → **9 verified, 0 errors, 0 warnings**.
- All 5 transition lemmas in `identity_map.proof.rs` are fully proven (no `admit()`).
- The 3 in-scope exec functions retain `proof! { admit(); }` for the obligations below.

Specs were NOT weakened to reach this state. Every obligation below was attempted
by removing the `admit()` and is reproduced from the Verus error output.

## Root cause (shared by all 3 entries)

`identity_map_view()` is declared `uninterp spec fn identity_map_view() -> IdentityMapView`
in `identity_map.spec.rs`. With no arguments it denotes a single fixed-but-opaque
constant, and its connection to the concrete singleton state
(`KERNEL_PD_PADDR`/`KERNEL_CR3` + the BSS page-table pool) is — by design and by the
file's own comments — "realized in the proving phase by a ghost token over those
singletons." That ghost-token framework does not yet exist in the codebase, and
building it is out of scope here:
- it would require a spec-readable view of module-level atomics (a `PointsTo` /
  global-invariant token), changing exec structure / signatures of out-of-scope
  callers (`init`, `memcpy`, `memset`);
- it depends on `arch::…::paging::table::Table::write` exposing an `old@ -> @`
  slot-update postcondition, which `tcb-allowed.md` records as **deliberately
  deferred** (a contents postcondition on that `external_body` write was shown
  unsound) — and `table.rs` is not in scope for this task.

Because `identity_map_view()` is an opaque constant, an `Ok`-path obligation
`identity_map_view().mapped.contains(p)` and the `Err`-path obligation
`!identity_map_view().mapped.contains(p)` cannot both be discharged from the exec
code (the return decision is not tied to the fixed unknown set). This is a
fundamental consequence of the deferred view realization, not a missing lemma.

## Entries

- Function: `ensure_pte` (src/kernel/src/mm/virt/identity_map.rs:627)
- Status: UNPROVEN (exec `admit()` retained)
- Attempts: removed `admit()`; discharged the `pt.read` precondition is feasible via
  `use_type_invariant(pte_idx)` (TableIndex type-invariant gives `@ < PAGE_TABLE_LENGTH`).
- Blocker: postcondition L619 `Ok(_) => identity_map_view().mapped.contains(spec_page_base(phys_addr as int))`
  ("postcondition not satisfied"). Opaque global-view fact; needs the page-table
  permission token that ties `pt.write` to `identity_map_view().mapped`.

- Function: `identity_map_page` (src/kernel/src/mm/virt/identity_map.rs:718)
- Status: UNPROVEN (exec `admit()` retained)
- Attempts: removed `admit()`.
- Blocker: postcondition L708 `Ok(_) => identity_map_view().accessible(phys_addr@)`
  ("postcondition not satisfied", multiple instances). `accessible` is
  `!initialized || mapped.contains(page)` over the opaque global view; there is no
  spec link from the exec `pd_paddr == 0` test to `identity_map_view().initialized`,
  nor from `ensure_pt`/`ensure_pte` to `mapped`. Needs the same ghost token.

- Function: `ensure_pt` (src/kernel/src/mm/virt/identity_map.rs:533)
- Status: UNPROVEN (exec `admit()` retained)
- Attempts: removed `admit()`.
- Blockers (all "precondition/postcondition not satisfied"):
  1. L535 `pd.read(pde_idx)` precondition `pde_idx@ < PAGE_TABLE_LENGTH` —
     dischargeable via `use_type_invariant(pde_idx)`.
  2. L550 `PAGE_TABLE_ALLOCATOR.alloc_as::<…>()` precondition `bump_view(self).inv()` —
     needs a spec-readable invariant for the module-static allocator (ghost token).
  3. postcondition `Ok(pt_paddr) => spec_is_page_aligned(pt_paddr as int)`:
     - present-PDE path (`pt_paddr = pde.frame_address()`) IS provable
       (`frame_address` ensures `% FRAME_SIZE == 0`, and `FRAME_SIZE == PAGE_SIZE ==
       spec_page_size()`).
     - fresh-allocation path (`pt_paddr = slot.as_ptr() as usize`) is NOT provable:
       the `<[T]>::as_ptr` `assume_specification` (in `identity_map.spec.rs`) has an
       empty contract and is not linked to the allocator's `slot_ref_addr(slot)`
       alignment guarantee. Tightening `as_ptr` universally would be unsound (a
       slice's `as_ptr` is not generally page-aligned). Needs the ghost-pointer link
       provided by the deferred token.

## Not a bug

No auto-fixable or complex code bug was found in the three target functions
(no overflow/off-by-one/missing-bound/unchecked-cast issues). All blockers are
spec-framework deferrals, so nothing was recorded in `bugs.md`.
