# Integrity Audit Review: cache

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

## Reviewers

Two independent sub-agents reviewed with different models:
- **Claude (claude-opus-4.6)**: see `review_r1.claude.md`
- **GPT (gpt-5.3-codex)**: see `review_r1.gpt.md`

Both reviewers agree: **PASS**. All findings below are independently confirmed by both.

## Cheating Items Still Present

All counts independently verified by both reviewers (grep across all 4 source files):

| Item | Count | Notes |
|------|-------|-------|
| admit() | 0 | — |
| assume() | 0 | — |
| external_body (functions) | 6 | btreemap_remove, deref, get, put, **find_lru_victim**, axiom_cache_lru_of_remove |
| external_body (type specs) | 2 | ExBTreeMap, ExCacheGuard |
| trusted | 0 | — |
| exec_allows_no_decreases | 0 | — |
| cfg-gated exec code | 0 | Only ghost includes and test module cfgs |
| assume_specification | 5 | BTreeMap::{new, len, is_empty, insert, clear} in lib.vstd_btree.rs |
| broadcast axiom | 2 | axiom_btree_map_view_finite_dom, axiom_spec_btree_map_len |
| external_type_specification | 4 | ExBTreeMap (w/ external_body), ExGlobal, ExCacheEntry, ExCacheGuard (w/ external_body) |

All counts match the fix_report claims. ✅

### Each external_body item — necessity assessment

| # | Item | Classification | Verdict | Reason |
|---|------|----------------|---------|--------|
| 1 | `btreemap_remove` | STDLIB_WRAPPER | **KEEP** | Thin wrapper (single `m.remove(k)` call). BTreeMap::remove has Borrow\<Q\> generic that blocks direct assume_specification on alloc path. |
| 2 | `CacheGuard::deref` | VERUS_LIMITATION | **KEEP** | CacheGuard is opaque (&mut V field). Field access unverifiable. Fundamental limitation. |
| 3 | `Cache::get` | VERUS_LIMITATION | **KEEP** | get_mut has no vstd spec + returns Option\<&mut V\> (Verus limitation). CacheGuard construction also needs &mut. Even with remove+insert rewrite, still need get_mut to obtain &mut V for the guard. Circular. |
| 4 | `Cache::put` | VERUS_LIMITATION | **KEEP** | Uses get_mut for in-place update. Theoretically eliminable via remove+insert rewrite (see Challengeable Items below), but this changes exec algorithm and source integrity policy blocks it. Both reviewers agree: KEEP under current policy. |
| 5 | `Cache::find_lru_victim` | VERUS_LIMITATION | **KEEP** | Uses `iter().min_by_key()` chain. vstd has BTreeMap::iter specs but they are cfg(std)-gated — unavailable on this no_std target. Porting ~80 lines of iter specs + rewriting to manual loop is feasible but substantial exec code modification. Both reviewers agree: KEEP. |
| 6 | `axiom_cache_lru_of_remove` | VERUS_LIMITATION | **KEEP** | Axiom on uninterpreted cache_lru_of. Proving it would require building sort-by-value infrastructure over vstd Map (no ordering primitives exist). Axiom is narrow and obviously sound. |
| 7 | `ExBTreeMap` | EXTERNAL_TYPE | **KEEP** | vstd btree specs are cfg(std)-gated. no_std target cannot use them. |
| 8 | `ExCacheGuard` | VERUS_LIMITATION | **KEEP** | &mut V in struct field — Verus does not support &mut types in struct fields. |

## Challengeable Items (should have been eliminated)

**None identified.** Both reviewers agree no item is straightforwardly eliminable without either:
- Structural exec code modifications (blocked by source integrity policy), OR
- Trading one trust item for another (net trust reshuffling, not elimination)

Two items are **theoretically reducible** but correctly documented:

1. **Cache::put** — Could replace get_mut with btreemap_remove+insert. Would eliminate 1 external_body but add 1 STDLIB_WRAPPER (btreemap_contains_key). Requires algorithmic change (in-place mutation → remove+insert). Claude review calls this "technically eliminable with exec code modification" but agrees KEEP is justified under source integrity policy.

2. **Cache::find_lru_victim** — Could port ~80 lines of BTreeMap iter specs from vstd + rewrite iterator chain as manual loop. Substantial effort for marginal trust reduction. Both reviewers agree: KEEP.

Neither constitutes a FAIL because:
- The escalation ladder's "minimal equivalent rewrite" (step 3) is designed for small expression-level changes, not algorithmic restructuring
- Both would introduce new trust items (assume_specifications), not eliminate trust entirely
- Current items are correctly classified and documented with justifications

## AST Mismatches

| Function | Status | Cause | Verdict |
|----------|--------|-------|---------|
| `Cache::new` | MISMATCH | `Self { .. }` → `let result = Self { .. }; proof! { .. } result` — named return for ensures clause | **ACCEPT** (pre-approved deviation) |
| `Cache::remove` | MISMATCH | `self.entries.remove(key)` → `btreemap_remove(&mut self.entries, key); proof! { .. }` — stdlib wrapper for Borrow\<Q\> | **ACCEPT** (escalation ladder step 4, documented with VERUS REWRITE comment) |
| `Cache::evict` | MISMATCH | Inline iterator chain extracted to `find_lru_victim` + `self.entries.remove(&key)` → `btreemap_remove` + proof block | **ACCEPT** (extraction + stdlib wrapper, documented with VERUS REWRITE comments at lib.rs:352,354) |
| `Cache::find_lru_victim` | EXTRA | New function extracted from evict, contains the original iterator chain body | **ACCEPT** (necessary extraction, VERUS REWRITE comment at lib.rs:326) |
| `btreemap_remove` | EXTRA | New stdlib wrapper function, body is single `m.remove(k)` call | **ACCEPT** (STDLIB_WRAPPER in trust.md) |

All mismatches are legitimate and documented. No unauthorized exec code modifications. ✅

## Issues (highest priority first)

### ISSUE-1 (MEDIUM): Errors corrected from prior review round

The prior review_r1.md (now superseded by this document) had several factual errors:

1. **Naming error:** Listed `Cache::evict` as an external_body function. The actual external_body function is `Cache::find_lru_victim` (lib.rs:315-331). `Cache::evict` (lib.rs:338-360) is body-verified. Both reviewers independently flagged this.

2. **Coverage count:** Stated "8/9" but correct is **9/10** (10 exec functions, 9 with contracts, deref_mut excluded). The error arose from conflating `find_lru_victim` with `evict`.

3. **AST consistency incomplete:** Listed 2 mismatches + 1 extra; correct is **3 mismatches + 2 extras** (evict mismatch and find_lru_victim extra were missing).

4. **assume_specification fidelity:** Claimed "No strengthening or weakening of postconditions" but fix_report and trust.md correctly note that 2 of 5 specs are stronger than upstream (dropped `obeys_cmp_spec` guards).

All corrections are reflected in this review.

### ISSUE-2 (LOW): Inaccurate claim about BTreeMap::iter specs

**Location:** trust.md line 103-104
**Claim:** "Iterator combinators (iter, min_by_key, map) have no vstd specs"
**Reality:** vstd has comprehensive BTreeMap::iter support (ExMapIter type, assume_specification for iter and Iter::next, ForLoopGhostIterator impl). These are cfg(std)-gated, not absent. Only min_by_key has no vstd spec.
**Correction needed:** Change to "BTreeMap::iter specs exist in vstd but are unavailable on this no_std target (cfg(std)-gated). The min_by_key combinator has no vstd spec."
**Impact:** Documentation accuracy only — does not change the KEEP verdict.
**Both reviewers flagged this independently.** ✅

### ISSUE-3 (LOW): assume_specification fidelity deviation

Two of 5 assume_specifications (BTreeMap::insert, BTreeMap::len axiom) drop upstream `obeys_cmp_spec` / `key_obeys_cmp_spec` guards, making local specs unconditionally stronger than upstream vstd. Practical risk is low (all standard types satisfy well-formed Ord). Correctly documented in trust.md lines 185-201.

### ISSUE-4 (INFORMATIONAL): Counter overflow trust assumption

The external_body on Cache::get and Cache::put masks counter overflow (u64). Spec uses abstract Seq ordering immune to overflow, but implementation depends on no wrap. Correctly documented in trust.md and bugs.md BUG-1. At 10B ops/sec, overflow requires ~58 years — physically unreachable.

### ISSUE-5 (INFORMATIONAL): axiom_cache_lru_of_remove is unproven trusted glue

Sound but unproven axiom over an uninterpreted function. Provable in principle with substantial new spec/proof infrastructure (~100+ lines). Correctly classified as VERUS_LIMITATION. Both reviewers agree: reasonable trust boundary.

### ISSUE-6 (INFORMATIONAL): deref_mut excluded from verification

`CacheGuard::deref_mut` (lib.rs:102-105) returns `&mut V` — Verus cannot handle this. Function has no spec and is excluded from verification. Mutation-through-guard semantics are unmodeled. Correctly documented in trust.md.

## Verification

```
make verify-cache → Exit code: 0
  verification: 0 errors
  cheating: assume=0 external_body=8 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 9/10 exec functions have contracts (deref_mut correctly excluded)
```

## Result: PASS

**Rationale:** All checklist items verified. Zero admit/assume/trusted/no_decreases/cfg-gated exec code. All 8 external_body items challenged independently by two reviewers using different models — all are genuine trust boundaries that cannot be eliminated without structural exec code modifications (which would trade one trust type for another, not eliminate trust). AST mismatches (3 mismatches + 2 extras) are all justified pre-approved deviations with VERUS REWRITE comments. trust.md documents all items with justifications. Verification passes with 0 errors.

**Corrections applied from prior review round:** Fixed naming (find_lru_victim not evict), coverage count (9/10 not 8/9), complete AST analysis (3+2 not 2+1), and assume_specification fidelity accuracy.

**Recommendations:**
1. Fix ISSUE-2: Update trust.md line 103-104 to clarify BTreeMap::iter specs exist but are cfg(std)-gated.
2. Note ISSUE-3: Document the assume_specification fidelity deviation (dropped obeys_cmp_spec guards) more prominently.
