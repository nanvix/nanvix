# verification-todo.md — `mm::virt::identity_map`

Honest hand-off of genuinely-stuck proofs (per the `proving-guide` skill). These
are **not** code bugs and **not** accepted trust assumptions; they are proof
obligations that cannot be discharged in-body under the task's hard rules
(fixed exec signatures, no new trust boundaries, no `admit`/`assume`, no
`external_body` on in-scope functions, no spec weakening).

The three in-scope functions all reference the **parameter-free, uninterpreted
global view** `identity_map_view()` in their contracts while mutating/reading
global singleton state (the `KERNEL_PD_PADDR` atomic and the BSS page-table
pool). This is the same architectural pattern as every analogous function in the
sibling `mm::phys` module — and in that (completed) module **every** such
function is `#[verus_verify(external_body)]` and listed in `tcb-allowed.md`
(`frame::alloc`, `frame::book`, `book_physical_memory_regions`,
`book_mmio_regions`, `instance`, `manager::init`, `kframe::new`, …), with the
ghost-token "attachment" lemmas (`lemma_manager_attached`,
`lemma_kernel_alloc_one`, …) left `admit()`-ed. The connecting `v -> v'`
transition is **deferred to a proving-phase ghost/permission token that is never
realized** in this codebase.

These three functions are the `mm::virt` counterparts of those TCB wrappers, so
they require the identical treatment. They cannot be verified in-body without
either (a) threading a `Tracked<...>` permission token through their signatures
— forbidden because the signatures are fixed and their out-of-scope callers
(`init`, `KernelFrame::new`) must not change — or (b) registering them as TCB
`external_body` — forbidden because introducing/recording a new trust boundary
during proving is not allowed (`verus-constraints`).

---

## Function: `ensure_pt` (identity_map.rs:533) — UNPROVEN

- Status: UNPROVEN (placeholder `proof! { admit(); }` retained, honestly).
- Obligations that ARE dischardgeable from available facts:
  - `pd.read(pde_idx)` precondition `index@ < PAGE_TABLE_LENGTH` — via
    `use_type_invariant(pde_idx)` (`TableIndex::inv`).
  - Present-PDE fast path: `spec_is_page_aligned(pde.frame_address())` — from
    `frame_address`'s `result % FRAME_SIZE == 0` plus `FRAME_SIZE == PAGE_SIZE`.
  - The two `inv()` ensures arms — trivial: `identity_map_view()` is
    parameter-free, so the post-state value equals the precondition value.
- Irreducible blockers (no rule-compliant discharge):
  1. **`PAGE_TABLE_ALLOCATOR.alloc_as::<…>()` precondition `bump_view(self).inv()`.**
     `bump_view` (bump_allocator/src/lib.spec.rs) is `uninterp` with **no**
     establishing lemma and **no** type-invariant (it is a free `uninterp spec
     fn` precisely to avoid a duplicate-`impl` front-end panic). No caller can
     establish it for the global static; it is part of the deferred ghost token.
  2. **Allocation-path alignment of `pt_paddr = slot.as_ptr() as usize`.**
     `<[T]>::as_ptr` is an `assume_specification` with **no** `ensures`, and the
     allocator's alignment guarantee is stated over the `uninterp`
     `slot_ref_addr(...)` of the *`MaybeUninit` slot*, with no fact connecting it
     to `assume_init_mut().as_ptr()`. So no alignment fact is available.
- Attempts: removed `admit()` and verified; Verus reports exactly
  `failed precondition bump_view(self).inv()` (lib.rs:350) and the missing
  alignment fact. Searched all of `src` for any lemma/broadcast/type-invariant
  that *ensures* `bump_view(_).inv()` — none exists.

## Function: `ensure_pte` (identity_map.rs:627) — UNPROVEN

- Status: UNPROVEN (placeholder retained).
- Irreducible blockers:
  1. **`Ok(_) => identity_map_view().mapped.contains(spec_page_base(phys_addr))`.**
     The page is installed via `pt.write(pte_idx, new_pte)`, but `Table::write`
     (arch/.../table.rs:246) deliberately carries **no contents `ensures`** (the
     TCB note proves a contents postcondition would be *unsound* for an
     `external_body` write). Therefore the write produces **no observable state
     change**, and `identity_map_view()` (uninterp, parameter-free) is an
     unconstrained constant whose `mapped` set cannot be shown to contain any
     specific page. `inv()` only guarantees alignment + the pre-init emptiness
     law — never membership.
  2. **`Err(_) => !identity_map_view().mapped.contains(...)`** — symmetric; not
     derivable from `inv()` either.
- A concrete (non-`uninterp`) redefinition of `identity_map_view()` cannot help:
  any definition strong enough to make `mapped.contains(P)` provable for an
  arbitrary call argument would have to contain all page-aligned addresses,
  trivializing the postcondition (spec drift / weakening — rejected).

## Function: `identity_map_page` (identity_map.rs:718) — UNPROVEN

- Status: UNPROVEN (placeholder retained).
- Irreducible blockers:
  1. **Pre-init no-op path:** `pd_paddr == 0` returns `Ok(())`, requiring
     `identity_map_view().accessible(phys_addr@)`, i.e.
     `!identity_map_view().initialized || …`. Linking the runtime atomic value
     `pd_paddr == 0` to `!identity_map_view().initialized` needs an
     atomic→view fact. `KERNEL_PD_PADDR.load()` has **no** spec connecting its
     result to the view (the atomic is `#[verus_verify]` only so Verus can
     *name* it). Not derivable.
  2. **Post-init path:** reduces to `ensure_pte`'s `mapped.contains` obligation
     (blocked above) plus `ensure_pt` (blocked above), and the `Err` arm needs
     the same atomic→view and write→view links.

---

## Recommended resolution

Treat `ensure_pt`, `ensure_pte`, `identity_map_page` exactly as their already
TCB-listed `mm::phys` analogues: realize the `mm::virt` identity-map
ghost/permission token (the one `kframe::new`'s TCB entry already cites as "not
realized in `mm::phys`"). That realization is a cross-cutting infrastructure
change — adding a tracked page-table-permission resource threaded through
`Table::read`/`write`, the atomics, and the allocator — and is outside the
in-body, fixed-signature scope of this proving task. Until then these three
functions are in the identical deferred state as `frame::alloc` / `book` /
`instance`, and the honest representation is the retained `admit()` placeholder
plus this record.
