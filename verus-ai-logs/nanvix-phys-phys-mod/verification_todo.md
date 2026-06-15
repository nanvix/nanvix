# Verification TODO — phys-mod (`src/kernel/src/mm/phys/mod.rs`)

Scope: in-scope functions `init`, `book_physical_memory_regions`, `book_mmio_regions`
and the spec/proof files `mod.spec.rs` / `mod.proof.rs`.

## Remaining proof gaps

**None.** There are zero `admit()`/`assume()` proof gaps in the phys-mod scope:

- `init` is machine-verified (`86 verified, 0 errors` for `MODULE=mm::phys`). Its
  postconditions (`phys_view().live()`, all physical regions reserved, all covered MMIO
  frames reserved) are discharged purely from the trusted contracts of its callees
  (`frame::init`, `book_physical_memory_regions`, `book_mmio_regions`, `Upool::new`,
  `PhysMemoryManager::init`). `mod.proof.rs` carries no lemma body — the bridge facts come
  directly from those callees' `ensures`.

## Permanent trust boundaries (TCB-allowed `external_body`, not proof gaps)

These are NOT stuck proofs; they are design-forced foreign-type boundaries already listed in
`verus-ai-logs/tcb-allowed.md`. The verus-constraints escalation ladder is exhausted:

- `book_physical_memory_regions` / `book_mmio_regions` — iterate a foreign
  `alloc::collections::LinkedList` in a `for` loop. `vstd` ships **no** `LinkedList` model
  (`grep LinkedList ~/toolchain/verus` → empty) and the orphan rule forbids a downstream crate
  from implementing vstd's `View` / `ForLoopGhostIterator` for the foreign `LinkedList` /
  `linked_list::Iter`. A rewrite to an iterable container would change the exec signatures and
  data structures (ast-consistency violation), so `external_body` with a real `ensures` is the
  sanctioned outcome.
- `ExLinkedList` (`mod.spec.rs`) — `external_type_specification` (requires the mandatory
  `external_body`) registering the foreign `LinkedList` as a Verus-visible opaque type; the
  only way to name an unparseable foreign type in spec signatures.
