# Polish Report: phys-mod

Scope: `src/kernel/src/mm/phys/mod.rs` (+ `mod.spec.rs`, `mod.proof.rs`).
In-scope functions: `init`, `book_physical_memory_regions`, `book_mmio_regions`.

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks.py mod.rs --threshold 5 --all` reports
    "No proof blocks found." (tree-sitter parser confirmed working, so this is a
    genuine result, not a backend false-negative).
  - `init` is body-verified and discharges its postcondition directly from the
    dependency contracts of `frame::init` and `PhysMemoryManager::init`; it
    contains no inline `proof { }` / `proof!{ }` blocks, no `assert`s.
  - `book_physical_memory_regions` and `book_mmio_regions` are pre-approved
    `external_body` (TCB-listed, `LinkedList`-iteration limitation) and have no
    proof bodies.
- Blocks kept inline: 0 (no inline proof blocks exist in the module).

## Minimization
- Redundant assertions removed: 0 (no `assert` statements exist in `mod.rs`).
- Redundant lemmas/hints removed: 0
  - `mod.proof.rs` defines no lemmas (only an explanatory comment), so there is
    nothing to deduplicate.
  - The two `#[trigger]` annotations in the `forall` ensures-clauses of
    `book_mmio_regions`/`init` are required quantifier triggers, not removable
    hints.
- Dead spec functions removed: 0
  - Every spec fn in `mod.spec.rs` is `pub` (module API surface). Per the
    proof-minimization skill, `pub` spec functions are kept.
  - `no_free_frames`, `all_free`, `is_free`, `covers`, `reserved`,
    `all_reserved`, `book_all`, `book_covered`, `region_frame_addrs`,
    `phys_regions_frame_set`, `mmio_regions_frame_set` are referenced by sibling
    submodules (`frame`, `manager`, `upool`) and/or the in-scope contracts.
  - `byte_at_address` has no callers but is on the protected
    do-not-modify list and is `pub`; retained.
- Debug artifacts removed: 0 (no TODO/FIXME, property-ID tags, or commented-out
  code found in `mod.rs`/`mod.spec.rs`/`mod.proof.rs`).

## Verification
- `make verify-kernel MODULE=mm::phys`: PASS — `101 verified, 0 errors`.
- In-scope functions contain 0 admits. The global `admit=4` / `external_body=23`
  counts originate entirely from pre-existing, documented markers in sibling
  submodules (`manager.proof.rs`, `frame.rs`, etc.) outside this task's scope.

## Conclusion
The `mm::phys` module file is already integration-ready: it carries no inline
proof blocks, no redundant assertions/lemmas/hints, and no dead spec functions.
No source changes were required; verification remains green.
