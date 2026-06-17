# Polish Report: arch-paging-table

## Proof Extraction
- Blocks extracted: 1 shared lemma replacing 2 duplicated inline blocks
  - `pd_index` (table.rs old lines 110–114, 5-line `by (bit_vector)` index-bound proof)
    → `lemma_masked_index_bounded` (table.proof.rs)
  - `pt_index` (table.rs old lines 128–132, 5-line `by (bit_vector)` index-bound proof)
    → `lemma_masked_index_bounded` (table.proof.rs, same shared lemma — the two blocks
      differed only in the shift literal `22` vs `12`, now generic over `shift`)
- Blocks kept inline: 3 (each a single line / single assert)
  - `into_raw`: `proof! { use_type_invariant(self); }`
  - `pd_index`: `proof! { assert(PAGE_TABLE_LENGTH == 1024) by (compute); }`
    (required before the `let` for the exec `PAGE_TABLE_LENGTH - 1` overflow check)
  - `pt_index`: `proof! { assert(PAGE_TABLE_LENGTH == 1024) by (compute); }` (same reason)
  - (plus the two new 1-line `lemma_masked_index_bounded(...)` call sites)

## Minimization
- Redundant assertions removed: 2
  - `assert(crate::mem::PGTAB_SHIFT == 22) by (compute)` (pd_index)
  - `assert(crate::mem::PAGE_SHIFT == 12) by (compute)` (pt_index)
  - These pinned the per-function shift literal only to match a literal `bit_vector` proof.
    Making the extracted lemma generic over `shift` makes them unnecessary.
- Redundant lemmas/hints removed: 0
  - `lemma_entry_roundtrip` kept (unique trust anchor; no other lemma proves the same
    property; `pub` broadcast API for dependents).
  - All `pub` spec functions kept (module API).

## Result
- `make verify-arch`: 47 verified, 0 errors, exit 0.
- Cheating profile unchanged vs baseline: assume=0, admit=0, external_body=3
  (all on `tcb-allowed.md`: `Table::read`, `Table::write`).
- Normal build (`cargo build`, Verus erased) of the `arch` crate succeeds.
