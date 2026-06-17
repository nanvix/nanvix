# Polish Report: hal-memory-region

Scope (verification-order targets): `TruncatedMemoryRegion::start`,
`MemoryRegion::start`, `TruncatedMemoryRegion::size`, `MemoryRegion::size`.

Files: `region.rs` (exec), `region.spec.rs` (View/invariants), `region.proof.rs` (empty).

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks.py --all` reports **no proof blocks and no loop
    invariants** in `region.rs`. The four in-scope accessors discharge their
    `ensures` directly from the View definition, with no inline `proof { ... }`.
  - `region.proof.rs` contains no lemmas (`verus! { }`), so there was nothing
    to consolidate into.
- Blocks kept inline: 0 (none exist).

## Minimization
- Redundant assertions removed: 0
  - No `assert` / `proof!` / `proof_with!` statements exist in any of the three
    files, so there were none to prune.
- Redundant lemmas/hints removed: 0
  - The proof file is empty; no `by(...)` hints or duplicate lemmas exist.
- Dead spec functions removed: 0
  - All spec fns are `pub open spec` (`wf`, `is_page_aligned`,
    `spec_set_cache_policy`, both `inv`). Per proof-minimization, `pub` spec
    fns are part of the module API and are kept. `wf`/`is_page_aligned` are
    additionally consumed by the two `inv` predicates, and
    `spec_set_cache_policy` is the abstract transition for the out-of-scope
    `set_cache_policy`.
- Verification-related comment condensed: 1
  - `MemoryRegion::start` (region.rs:210): the 11-line `// VERUS REWRITE ...`
    block (with embedded minimal-reproducer and a stale `phys.rs:277-288`
    line reference) was condensed to a 3-line rationale. The `// VERUS REWRITE`
    tag (discouraged by verus-constraints) is removed while the essential
    "use `clone_address`, not `Clone::clone`" decision is preserved so it is
    not naively reverted. Exec logic (`self.start.clone_address()`) and the
    `#[verus_spec]` contract are unchanged.

## Verification
- `make verify-kernel MODULE=hal::mem::types::region` → **5 verified, 0 errors**,
  status CLEAN (assume=0, admit=0/4 module-local, no new external_body/trusted,
  no cheating in module).
- Diff since polish START is comment-only (`git diff e47c54a5a HEAD --
  region.rs`); `region.spec.rs` / `region.proof.rs` unchanged.
