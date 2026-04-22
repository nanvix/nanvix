# Integrity Audit Review: cache

**Date:** 2026-04-22
**Reviewers:** Claude Opus 4.6, GPT-5.3-Codex (independent parallel reviews)
**Consolidated by:** Claude Opus 4.6

## Checklist
### Integrity Audit
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code (only imports/derives/debug_assert/logging allowed)
- [x] Every external_body challenged — only genuinely unverifiable items survive
- [x] AST consistency: each mismatch analyzed (allowed deviation vs restore needed)
- [x] All exec rewrites have VERUS REWRITE comment and minimal reproducer
- [x] trust.md minimized — only genuinely unverifiable items remain
- [x] make verify passes (0 errors, 0 warnings)

## Cheating Items Still Present

| Item | Count | Truly necessary? |
|------|-------|-----------------|
| admit() | 0 | N/A |
| assume() | 0 | N/A |
| external_body (type) | 2 | Yes — ExBTreeMap (no_std + private fields), ExCacheGuard (&mut struct field) |
| external_body (function) | 7 | Yes — all blocked by no_std btree or &mut limitations (see below) |
| trusted | 0 | N/A |
| no_decreases | 0 | N/A |
| cfg-gated exec | 0 | N/A |
| assume_specification | 0 | N/A |

### Function-level external_body details

| Function | Root blocker | Independent blockers |
|----------|-------------|---------------------|
| CacheGuard::deref | CacheGuard is external_body (opaque &mut field) | 1 |
| Cache::new | no_std blocks vstd btree specs + uninterp view | 2 |
| Cache::get | get_mut no vstd spec + &mut return + CacheGuard &mut | 3 |
| Cache::put | get_mut no vstd spec + calls evict + no_std btree | 3 |
| Cache::remove | no_std blocks vstd btree specs + uninterp view | 2 |
| Cache::clear | no_std blocks vstd btree specs + uninterp view | 2 |
| Cache::evict | min_by_key combinator no spec + no_std btree + BTreeMap::remove | 3 |

## Challengeable Items (should have been eliminated)

**None.** Both reviewers agree no external_body item can be eliminated under
current constraints. Three functions (new, remove, clear) are theoretically
eliminable with custom `assume_specification` + concrete view, but the
fix_report correctly rejected this because:
1. Only 3 of 7 function-level items would be eliminated
2. The 4 most complex methods (get, put, evict, deref) remain external_body
3. Trust shifts to assume_specification axioms, not eliminated
4. Engineering cost (concrete view, spec-level sort) is disproportionate

## AST Mismatches

**None.** AST consistency check: 18/18 functions match, 3/3 structs match.
All exec code is identical to the `dev` branch. No deviations, no rewrites.

## Reviewer Disagreement

The two independent reviewers initially disagreed on the verdict:

**Claude Opus 4.6: PASS** — All items correctly classified, trust.md minimal.

**GPT-5.3-Codex: FAIL** — Raised three concerns:
1. **Stale comments in lib.spec.rs** claiming "BTreeMap has no vstd specs" — factually
   imprecise (vstd has specs, they're just gated behind `cfg(std)`)
2. **evict blocker framing** too narrow — iterator chain could be rewritten as a loop
3. **put get_mut blocker** partially challengeable — body could use remove+insert

### Resolution

1. **Stale comments: VALID.** Fixed in this review — updated two comments in
   `lib.spec.rs` (lines 15-16, 171-172) to accurately state that vstd provides
   BTreeMap specs but they require `cfg(std)`, incompatible with no_std.

2. **evict loop rewrite: VALID concern, but does not change verdict.** Even with
   a for-loop rewrite, BTreeMap::iter and BTreeMap::remove specs are inaccessible
   on no_std. The blocker is the no_std constraint, not the iterator chain per se.
   The fix_report's analysis correctly identifies both blockers.

3. **put remove+insert rewrite: VALID concern, but does not change verdict.** Even
   rewritten, put still needs BTreeMap::remove and BTreeMap::insert specs (no_std
   blocked) plus calls self.evict() (still external_body). The exec code change
   alone doesn't eliminate the external_body.

## Issues (highest priority first)

1. **FIXED: Stale comments in lib.spec.rs** — Two comments incorrectly stated
   "BTreeMap has no vstd specs." Corrected to reference the `cfg(std)` gating.

2. **LOW: BUG-1 counter overflow** — `self.counter: u64` incremented without
   overflow check in `get`/`put`. Documented in bugs.md. Physically unreachable
   (~58 years at 10B ops/sec). Correctly classified as trust assumption.

3. **INFO: CacheGuard::deref_mut excluded** — Returns `&mut V`, Verus cannot
   annotate. Mutation-through-guard semantics unmodeled. Correctly documented.

## Verification Results

```
make verify-cache
  verification: 11 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=9 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 7/8 exec functions have contracts
```

All 5 invariant preservation lemmas + 5 helper lemmas fully proven.
No admit, assume, or trusted constructs anywhere.

## vstd Evidence

- `vstd/std_specs/btree.rs` provides `assume_specification` for: new, insert,
  get (immutable), remove, clear, len, is_empty, contains_key, keys, values,
  iter, Iter::next
- `get_mut`: **NOT specified** (0 occurrences in vstd — confirmed by both reviewers)
- Module gated: `#[cfg(all(feature = "alloc", feature = "std"))]` in
  `vstd/std_specs/mod.rs:17-18`
- btree.rs imports: `use std::collections::{BTreeMap, BTreeSet}` — hard std dependency
- Verus version: `0.2026.04.12.f1166c4`

## Result: PASS

All checklist items verified. The trust boundary is minimal — every external_body
has been independently challenged by two reviewers using different models, and none
can be eliminated under current constraints (no_std target, missing get_mut spec,
&mut struct field limitation). Documentation precision issue (stale comments) was
fixed during this review. Trust.md accurately documents all trust items with
correct classifications.
