# Review: cache proving (GPT-5.3-Codex)

## 1. Spec Preservation
- Ran: `git diff 1cd84e654 -- src/libs/cache/src/lib.rs src/libs/cache/src/lib.spec.rs`.
- Result: **no diff** (zero changed lines).
- Cross-check: `git diff --name-only 1cd84e654 -- src/libs/cache/src` shows only `src/libs/cache/src/lib.proof.rs` changed.
- Conclusion: specification baseline is preserved; no requires/ensures weakening/removal/trivialization in `lib.rs` or `lib.spec.rs`.

## 2. Cheating Audit
### Forbidden patterns (lib.rs/lib.spec.rs/lib.proof.rs)
- `admit()`: **0**
- `assume(`: **0**
- `cfg(not(verus_keep_ghost))` on exec code: **0**
- `trusted`: **0**
- `verifier::external` (plain, i.e., not `_body`/`_type_specification`): **0** (only `external_body` and `external_type_specification` are present)

### `external_body` inventory and assessment
Found 9 actual `external_body` attributes:
1. `lib.spec.rs:21` — `ExBTreeMap` (`external_type_specification` + `external_body`)
2. `lib.spec.rs:37` — `ExCacheGuard` (`external_type_specification` + `external_body`)
3. `lib.rs:91` — `CacheGuard::deref`
4. `lib.rs:141` — `Cache::new`
5. `lib.rs:168` — `Cache::get`
6. `lib.rs:208` — `Cache::put`
7. `lib.rs:254` — `Cache::remove`
8. `lib.rs:271` — `Cache::clear`
9. `lib.rs:289` — `Cache::evict`

All are documented in `verus-ai-logs/libs-cache/trust.md` with matching rationale.

Escalation-ladder assessment:
- Root blocker is still valid: `alloc::collections::BTreeMap` lacks vstd specs (view + method specs), so cache bodies cannot be verified directly.
- `CacheGuard` has `&mut` field/return constraints that Verus does not currently support for full body verification.
- Given current toolchain/library limits, these `external_body` uses are **legitimate trust boundaries**, not avoidable shortcuts in this crate.

## 3. Proof Quality
The five former stubs are now real proofs (`lemma_spec_new_inv`, `lemma_spec_get_inv`, `lemma_spec_put_inv`, `lemma_spec_remove_inv`, `lemma_spec_clear_inv`) and no `admit()` remains.

### Helper lemmas
- `lemma_push_preserves_no_dup`: good local reasoning via index separation; not brute-force.
- `lemma_filter_preserves_no_dup`: structurally recursive proof over sequence; appropriate and reusable.
- `lemma_filter_neq_to_set`: clean set-extensional proof (both directions) using filter lemmas.
- `lemma_filter_neq_len`: bridges `no_duplicates` + set conversion to cardinality fact; necessary for len obligations.
- `lemma_subrange_no_dup`: standard index-lifting argument; minimal and correct.
- `lemma_drop_first_to_set`: proper extensional proof for eviction branch; necessary for `subrange(1, len)` set relation.

### Main invariant lemmas
- Strategy is sound: each transition lemma proves preservation of all invariant components (`size <= capacity`, no duplicates, set/domain equality, len equality) branch-by-branch.
- `spec_put` coverage is especially complete (capacity 0 / overwrite / evict / insert branches).
- `spec_get`, `spec_remove`, `spec_clear` are appropriately lightweight where transition is identity or trivially empty-state.
- Minor polish only: a few locals (`result`, `mru`) are introduced mainly for solver guidance/readability and could be trimmed, but this is not a correctness issue.

## 4. Rewrite Audit
- Searched `lib.rs` for `VERUS REWRITE` comments: **none found**.
- No exec rewrite markers exist; consistent with the claim that proving-phase edits were confined to `lib.proof.rs`.

## Summary
**No blockers found.**
- Spec baseline is preserved exactly in `lib.rs` and `lib.spec.rs`.
- Cheating audit is clean (`admit/assume/trusted/plain verifier::external` all zero).
- `external_body` usage is explicit, documented, and justified by current Verus/BTreeMap limits.
- Proofs are substantive and structurally sound, not placeholder/brute-force artifacts.
