# Bugs: `mm::phys` (`src/kernel/src/mm/phys/mod.rs`)

## Summary

No code bugs were found in the verification targets (`init`,
`book_physical_memory_regions`, `book_mmio_regions`) during the spec phase.

The functions' logic is consistent with the caller expectations in
`caller_analysis.md`:

- `init` orchestrates `frame::init` → region booking → MMIO booking →
  `Upool`/`PhysMemoryManager` setup in the required order.
- `book_physical_memory_regions` books every region frame via `alloc_range`.
- `book_mmio_regions` correctly implements the "skip-if-not-covered" tolerance
  (`if frame::is_covered(phys) { frame::book(phys) }`).

## Non-bug: tooling limitation (recorded, not a defect)

The only obstacle to body-verifying the two `book_*` helpers is the absence of a
`vstd` specification for `alloc::collections::LinkedList` iteration, blocked by
Rust's orphan rule. This is a verification-tooling gap, **not** a code bug. Full
details in `verus-unsupported.md`. No code change is warranted.
