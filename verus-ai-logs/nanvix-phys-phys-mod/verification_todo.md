# Verification TODOs — mm::phys (mod)

These are honest hand-offs of items that cannot be body-verified with the current
toolchain. Both are pre-approved in `verus-ai-logs/tcb-allowed.md`, carry full
`#[verus_spec]` contracts, and therefore do **not** trip the cheating gate. They are
recorded here only to document the resolution path.

## 1. `book_physical_memory_regions` (src/kernel/src/mm/phys/mod.rs:73) — `external_body`

- **Blocking pattern**: `for region in physical_memory_regions.iter()` over an
  `alloc::collections::LinkedList`.
- **Verus limitation**: `vstd` ships no model for `LinkedList` (confirmed: `vstd/std_specs`
  provides `vec`, `vecdeque`, `btree`, but no `linked_list`). A `for` loop over
  `LinkedList::iter()` requires `View` + `ForLoopGhostIteratorNew` +
  `ForLoopGhostIterator` impls for the foreign `LinkedList` / `linked_list::Iter` types.
- **Why it cannot be fixed in-crate**: Rust orphan rule (E0117) forbids the downstream
  `kernel` crate from implementing vstd-owned traits for std-owned foreign types; only
  `vstd` may. Registering the type (`ExLinkedList` `external_type_specification`) lets it
  appear in spec signatures but supplies no iteration semantics.
- **Contract retained**: `ensures` Ok ⇒ every frame in
  `phys_regions_frame_set(&physical_memory_regions)` is reserved in `phys_view().frames`.
- **Resolution path**: removable once `vstd` gains a `LinkedList` model, or if the kernel
  API switches the region containers to `Vec`/`VecDeque` (already modeled by `vstd`).

## 2. `book_mmio_regions` (src/kernel/src/mm/phys/mod.rs:103) — `external_body`

- Same `LinkedList`-iteration limitation and orphan-rule blocker as item 1.
- **Contract retained**: `ensures` Ok ⇒ every *covered* frame of
  `mmio_regions_frame_set(mmio_regions)` is reserved (uncovered MMIO frames are skipped,
  matching the `frame::is_covered` gate).
- **Resolution path**: identical to item 1.

`init` itself has no loop and **is** body-verified; it calls both helpers through their
contracts. There are no `admit()` / `assume()` / `assume_specification` in the module's
`mod.rs` / `mod.spec.rs` / `mod.proof.rs`.
