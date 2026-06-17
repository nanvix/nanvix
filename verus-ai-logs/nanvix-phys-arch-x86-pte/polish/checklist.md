# Polish Report: arch-x86-pte

Module: `src/libs/arch/src/x86/mem/paging/pte.rs`
In-scope functions: `PageTableEntry::new`, `PageTableEntryFlags::new`,
`PageTableEntry::is_present`, `PageTableEntryFlags::is_present`.

Final verification: `make verify-arch MODULE=x86::mem::paging::pte` →
`6 verified, 0 errors` (exit 0), `admit=0`, `assume=0`. Source/spec/proof files
byte-identical to baseline (no net change required).

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks.py --all` reports 1 inline proof block total, 0 over the
    5-line threshold. Nothing qualifies for extraction.
- Blocks kept inline: 1
  - `PageTableEntry::new` (pte.rs:314) — `proof! { use_type_invariant(frame); }`
    (single line, ≤ 5). Discharges the `result.inv()` postcondition (frame bound
    from the `FrameNumber` type invariant). Verified required: removing it yields
    `error: postcondition not satisfied` (`5 verified, 1 errors`), so it is kept.

## Minimization
- Redundant assertions removed: 0
  - No `assert` statements exist in the four in-scope exec functions. The only
    proof artifact, `use_type_invariant(frame)`, was empirically tested and is
    load-bearing (see above), so it is not redundant.
- Redundant lemmas/hints removed: 0
  - The proof file (`pte.proof.rs`) is empty (`verus! { }`) — no lemmas to dedupe.
  - No redundant `by(...)`/trigger hints present.
- Dead spec functions removed: 0
  - All spec functions (`spec_cow_set`, `spec_pte_flags_new`, `spec_pte_new`,
    `PageTableEntryFlags::inv`, `PageTableEntry::inv`, the two `View` impls) are
    `pub` and referenced (in ensures clauses, the `View` impls, or `inv`). Per the
    proof-minimization skill, `pub` spec functions are retained as module API.
- Debug artifacts removed: 0
  - No TODO/FIXME comments, commented-out code, or property-ID annotations
    (`// FUNC-POST-*`, `// INV-*`) found in pte.rs / pte.spec.rs / pte.proof.rs.

## Summary
The module was already integration-clean: the sole inline proof block is a
single required line (below the extraction threshold), the proof file is empty,
and there are no redundant assertions, hints, lemmas, dead spec functions, or
debug artifacts. No changes were necessary; verification remains green.
