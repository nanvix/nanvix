# Bugs — `mm::virt::vmem`

## BUG-001: Ghost `vmem_view` placeholder does not reflect real address-space state

- **Severity:** spec-internal (does not affect runtime; `#[cfg(verus_keep_ghost)]`-gated).
- **Where:** `src/kernel/src/mm/virt/vmem.rs`, `Vmem::new` and `Vmem::clone` ghost-field
  initialization (the `vmem_view: Ghost<...>` field).
- **Reported by:** specification reviewer, turn 1 (FR-5).

### Description
The ghost `vmem_view` field was initialized to a placeholder abstract state
(`VmemView { user: Map::empty(), kernel: Map::empty(), pgdir: 0 }`). This is wrong
in two ways:

1. `pgdir: 0` is not the real page-directory base. `VmemView::inv()` requires
   `spec_is_physical_region(self.pgdir, page_size())` and `is_page_aligned(self.pgdir)`,
   neither of which `0` satisfies. So `inv()` cannot hold for the constructed value
   without `admit()`.
2. For `clone`, the postcondition `v@.kernel == from@.kernel` claims the clone
   carries the source's kernel mappings, but an `empty()` kernel map contradicts that.

The original `pgdir: 0` literal additionally caused a hard compile error
(`E0308: expected nat, found integer`) because `pgdir` is `nat` and `0` is an `int`
literal — this blocked Verus from running at all.

### Fix applied (this phase)
- Compile fix (FR-1): the placeholder construction was replaced with
  `Ghost::assume_new()`, which yields an unconstrained ghost value of the correct
  type and gets past `cargo check` so Verus runs.
- Correctness (FR-5): the ghost must be made to mirror the *real* built state — the
  populated kernel map and the actual page-directory physical base
  (`pgdir == pd.physical_address()@`). This requires the construction loops in
  `new`/`clone` to maintain a loop invariant relating the in-progress ghost map to
  the page-directory contents. Marked as proving-phase work: in the spec phase the
  `Ok`-arm postconditions are discharged with `proof! { admit(); }` (allowed by the
  task statement), so the unconstrained ghost is sound *for the spec phase*. The
  proving phase must replace `assume_new()` with a real construction and remove the
  `admit()`.

### Status
- Compile blocker: **FIXED** (`assume_new`).
- Semantic placeholder: **RECORDED** — to be eliminated in the proving phase together
  with the `admit()` removal (reviewer FR-2/FR-4).
