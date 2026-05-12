# Polish Report: frame

## Proof Extraction
- Blocks extracted: 13 (from 15 original blocks >5 lines)

| # | Old location (frame.rs) | Function | New lemma (frame.proof.rs) |
|---|------------------------|----------|---------------------------|
| 1 | alloc Err: ~15 lines proving free_frames empty | alloc | `lemma_bitmap_full_means_free_empty` |
| 2 | alloc Ok: ~20 lines proving spec_alloc | alloc | `lemma_set_bit_updates_view` |
| 3 | free Ok: ~40 lines proving spec_free | free | `lemma_clear_bit_updates_view` |
| 4 | free Err: ~15 lines proving fa ∉ allocated | free | `lemma_addr_not_allocated` |
| 5 | book Ok: shared with alloc Ok | book | `lemma_set_bit_updates_view` (reuse) |
| 6 | book Err: ~10 lines proving fa ∉ free | book | `lemma_addr_not_free` |
| 7 | alloc_range overflow: ~15 lines bounding sfn+nf | alloc_range | `lemma_frame_quotients_bounded` |
| 8 | alloc_range efn properties: ~20 lines | alloc_range | `lemma_end_frame_number_properties` |
| 9 | alloc_range post-loop: ~30 lines proving spec_alloc_range | alloc_range | `lemma_alloc_range_updates_view` |
| 10 | alloc_range loop body coverage: ~10 lines | alloc_range | `lemma_coverage_transfers` |
| 11 | alloc_range loop set_int_range step: ~8 lines | alloc_range | `lemma_range_insert_step` |
| 12 | alloc_range coverage-check error: ~16 lines | alloc_range | `lemma_alloc_range_conflict` |
| 13 | alloc_range test-loop error: ~16 lines (duplicate of #12) | alloc_range | `lemma_alloc_range_conflict` (reuse) |

Helper lemmas added:
- `lemma_view_unchanged`: proves self@ == old@ when bitmap unchanged (used 3+ times)
- `lemma_frame_addr_injective`: frame_addr_of injectivity (used by set_bit/clear_bit lemmas)

- Blocks kept inline: 8 (each ≤5 functional lines — variable bindings + lemma calls tightly coupled to exec context)

## Minimization
- Redundant assertions removed: 9
  1. `assert(idx >= 0)` in alloc (trivially true for usize cast)
  2. `assert(fa == frame_addr_of(idx))` in alloc Ok (follows from postcondition)
  3. `assert(old(self).bitmap@.is_covered(idx))` in alloc (follows from bitmap.alloc() postcondition)
  4. `assert(frame_addr_of(idx) <= usize::MAX as int)` in alloc (follows from internal_inv)
  5. `assert(self.bitmap@.is_covered(index as int))` in booking loop (follows from lemma_coverage_transfers ensures)
  6. `assert(!old(self).bitmap@.set_bits.contains(index))` in booking loop (follows from loop invariant)
  7. `assert(!set_int_range(...).contains(index))` in booking loop (follows from set_int_range definition)
  8. `assert(self@ =~= old(self)@.spec_alloc_range(...))` in post-loop (follows from lemma_alloc_range_updates_view ensures)
  9. `assert(old_inner@.free_frames.contains(fa))` in lemma_set_bit_updates_view (follows from preconditions)
- Redundant lemmas/hints removed: 0
- Intermediate assertions removed from lemma_end_frame_number_properties: 3
  - `assert(sfn + nf == (start + size) / ps)` (follows from hoist_over_denominator)
  - `assert(sfn * ps == start)` (follows from fundamental_div_mod + commutativity)
  - `assert(start + size == sfn * ps + nf * ps)` (follows from nonlinear_arith block)

## Verification Status
- **24 verified, 0 errors** (exit 0)
- assume=0, admit=0, trusted=0
- 13/15 exec functions have contracts (instance, init unverified — pre-existing)
