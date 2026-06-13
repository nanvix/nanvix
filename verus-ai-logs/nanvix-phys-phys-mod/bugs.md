# Bugs — mm::phys

## Code bugs

None found. The three target functions (`init`, `book_physical_memory_regions`,
`book_mmio_regions`) are logically correct; no overflow, off-by-one, or impossible
path was detected during speccing.

## Verifier limitation (not a code bug) — LinkedList iteration

**What**: `book_physical_memory_regions` and `book_mmio_regions` iterate an
`alloc::collections::LinkedList` with a `for region in list.iter()` loop. Verus
cannot reason about either function's body.

**Why**: `vstd` ships no specification for `LinkedList`. Supporting a `for` loop over
`list.iter()` requires implementing vstd's `View`, `ForLoopGhostIteratorNew`, and
`ForLoopGhostIterator` traits for the foreign `LinkedList` / `linked_list::Iter`
types. Rust's orphan rule (E0117) forbids a downstream crate (the kernel) from
implementing vstd-owned traits for std-owned types — only `vstd` itself may do so,
and the pinned toolchain `vstd` does not. Registering the type alone
(`external_type_specification`, done in `mod.spec.rs` as `ExLinkedList`) lets the type
appear in spec signatures but does not provide iteration semantics.

**Consequence / deviation**: both helpers are marked `#[verus_verify(external_body)]`
with meaningful `requires`/`ensures` describing their effect on the abstract
`phys_view()` (every physical region frame becomes reserved; every *covered* MMIO frame
becomes reserved, uncovered skipped). They are listed in `tcb-allowed.md`. `init` itself
has no loop and **is** body-verified, calling the two helpers through their contracts.

**Resolution path**: if/when `vstd` gains a `LinkedList` model (or these lists are
replaced by `Vec`/`VecDeque`, which `vstd` already supports), the `external_body`
markers can be removed and the loop bodies verified directly.

**How Verus helped**: surfaced that the data structure choice (`LinkedList`) is opaque
to verification; a `Vec`/`VecDeque` would be fully verifiable here.
