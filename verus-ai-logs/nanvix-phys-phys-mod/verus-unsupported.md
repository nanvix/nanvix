# Verus-unsupported constructs in `mm::phys`

## std `LinkedList` for-loops in `book_physical_memory_regions` / `book_mmio_regions`

### What
Both helper functions iterate a `alloc::collections::LinkedList` of memory
regions with a `for region in list.iter() { ... }` loop:

- `book_physical_memory_regions(physical_memory_regions: LinkedList<TruncatedMemoryRegion<...>>)`
- `book_mmio_regions(mmio_regions: &LinkedList<TruncatedMemoryRegion<...>>)`

### Why it cannot be body-verified here
1. `vstd` ships ghost-iterator (`View` + `ForLoopGhostIterator`) specifications
   only for `slice::Iter`, `vec::Iter`, and `VecDeque` — **not** for
   `LinkedList` / `linked_list::Iter`.
2. Supplying that machinery requires `impl vstd::view::View for
   alloc::collections::linked_list::Iter<'_, T>` and
   `impl vstd::pervasive::ForLoopGhostIterator for ...Iter<...>` from the
   `kernel` crate. Both the trait (`vstd`) **and** the type (`alloc`) are
   foreign to `kernel`, so Rust's orphan rule rejects the impls:

   ```
   error[E0117]: only traits defined in the current crate can be implemented
                 for types defined outside of the crate
   ```
3. `vstd` is a pinned, versioned dependency
   (`= "0.0.0-2026-05-31-0205"`); it cannot be extended in-tree to add the
   missing `LinkedList` iterator specification. (The copy under
   `verus-ai-exp/verus-ai/vstd/` is a read-only reference, not the build input.)

There is no code bug here — the limitation is in the verification tooling's std
coverage, not in the kernel logic.

### Mitigation
- A type-only spec is provided for `LinkedList` (`ExLinkedList` via
  `#[verifier::external_type_specification]`) so the verified `init` function can
  name `LinkedList` parameters.
- The two looping helpers are annotated `#[verus_verify(external_body)]` so Verus
  skips their (un-expressible) loop bodies while still recognizing their signatures.
- The *abstract* effect of these loops is still specified and proof-obligated via
  the `PhysMemView` transition vocabulary in `mod.spec.rs`
  (`spec_book_frame`, `spec_book_frames`, `region_frames`) and the lemma
  signatures in `mod.proof.rs`
  (`lemma_book_region_reserves_region_frames`, `lemma_book_mmio_skip_untracked`,
  `lemma_book_mmio_books_tracked`). When a future `vstd` adds `LinkedList`
  iterator support, the `external_body` markers can be removed and these helpers
  body-verified against the existing lemmas.

### Re-evaluation trigger
Remove the `external_body` markers on `book_physical_memory_regions` /
`book_mmio_regions` once either:
- `vstd` gains `LinkedList` ghost-iterator support, or
- the regions are passed as a slice/`Vec` (which `vstd` already supports).
