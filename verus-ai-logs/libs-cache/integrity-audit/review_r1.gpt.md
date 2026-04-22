# Independent Integrity Audit Review — GPT

## Cheating Item Counts
Verified from source + `make verify-cache` output.

- `admit()`: **0** (`rg "admit\\(" src/libs/cache/src`)
- `assume()`: **0** (`rg "assume\\(" src/libs/cache/src`)
- `trusted`: **0**
- `external_body`: **8** total
  - Exec/proof fns (**6**): `CacheGuard::deref` (lib.rs:93), `btreemap_remove` (lib.rs:114), `Cache::get` (lib.rs:190), `Cache::put` (lib.rs:230), `Cache::find_lru_victim` (lib.rs:315), `axiom_cache_lru_of_remove` (lib.proof.rs:401)
  - Type specs (**2**): `ExBTreeMap` (lib.vstd_btree.rs:32), `ExCacheGuard` (lib.spec.rs:24)
- `assume_specification`: **5 declarations** (lib.vstd_btree.rs:69,88,98,108,130)
- `broadcast axiom`: **2 declarations** (lib.vstd_btree.rs:56,80)
- `external_type_specification`: **4** (`ExBTreeMap`, `ExGlobal`, `ExCacheEntry`, `ExCacheGuard`)
- cfg-gated exec code: **0**

`make verify-cache` confirms: verification exit 0, cheating `assume=0 external_body=8 admit=0 trusted=0 no_decreases=0 cfg_gate=0`, coverage `9/10` (only `deref_mut` excluded).

## Challenge Results
1. **btreemap_remove** (STDLIB_WRAPPER): keep. Thin wrapper over `m.remove(k)` with precise postconditions; direct alloc-path `Borrow<Q>` spec support is missing in this crate setup.
2. **CacheGuard::deref** (VERUS_LIMITATION): keep. `CacheGuard` is opaque (`&mut` field type limitation), so field-read body cannot be verified.
3. **Cache::get** (VERUS_LIMITATION): keep. Depends on `get_mut` and returning mutable-backed guard; this is exactly where Verus support is missing.
4. **Cache::put** (VERUS_LIMITATION): keep. Existing-key in-place update uses `get_mut`; eliminating requires structural rewrite (remove/reinsert path + extra trusted interface), not a trust-boundary reduction.
5. **Cache::find_lru_victim** (VERUS_LIMITATION): keep. Isolates unverifiable iterator/combinator chain into smallest external_body surface.
6. **axiom_cache_lru_of_remove** (VERUS_LIMITATION): keep. Needed because `cache_lru_of_nonempty` is uninterpreted; relation is narrow and targeted.
7. **ExBTreeMap** (EXTERNAL_TYPE): keep. Required no_std alloc BTreeMap visibility/type bridging.
8. **ExCacheGuard** (VERUS_LIMITATION): keep. Required for `&mut`-field struct type.

## AST Consistency Analysis
Recomputed with `/home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py`:
- Matched: 15
- Mismatched: 3 (`Cache::new`, `Cache::remove`, `Cache::evict`)
- Extra: 2 (`Cache::find_lru_victim`, `btreemap_remove`)

Per item:
- **Cache::new** mismatch: justified (pre-approved named-result rewrite + proof block). `VERUS REWRITE` comment: **No**.
- **Cache::remove** mismatch: justified (`remove` -> `btreemap_remove` + proof). `VERUS REWRITE`: **Yes** (lib.rs:284).
- **Cache::evict** mismatch: justified (extracted helper + wrapper + proof). `VERUS REWRITE`: **Yes** (lib.rs:352,354).
- **Cache::find_lru_victim** extra: justified extraction of iterator chain. `VERUS REWRITE`: **Yes** (lib.rs:326).
- **btreemap_remove** extra: justified stdlib wrapper. `VERUS REWRITE`: **No** (not a rewrite site; new wrapper function).

## Bug vs Limitation
- `btreemap_remove`: limitation/wrapper; no cache-specific defect evidence.
- `CacheGuard::deref`: limitation.
- `Cache::get`: limitation, but external_body means counter-overflow behavior is not mechanically checked.
- `Cache::put`: limitation, same overflow caveat.
- `Cache::find_lru_victim`: limitation; trusts iterator-chain behavior to match spec abstraction.
- `axiom_cache_lru_of_remove`: limitation/axiom trust, not an exec bug.
- `ExBTreeMap`: external type boundary.
- `ExCacheGuard`: limitation.

## Errors in Existing Review
In `integrity-audit/review_r1.md`:
1. **Misnamed external_body function**: lists `Cache::evict` instead of `Cache::find_lru_victim` (line 30/table and line 49).
2. **AST table incomplete/incorrect**: omits `Cache::evict` mismatch and lists only one extra in table section.
3. **Coverage claim incorrect**: says `8/9` (line 110); verifier output is **`9/10`**.

## Spec Quality Assessment
- `btreemap_remove`: strong and appropriate (state + return relation).
- `CacheGuard::deref`: minimal but consistent with opaque guard model.
- `Cache::get`: reasonably strong hit/miss split; ties returned guard view and abstract transition.
- `Cache::put`: strong abstract transition spec (`spec_put`) + invariant preservation.
- `Cache::find_lru_victim`: adequate for current abstraction, but relies on uninterpreted `cache_lru_of` (limited semantic grounding).
- `axiom_cache_lru_of_remove`: narrow and useful, but inherently trusted.
- Type external bodies (`ExBTreeMap`, `ExCacheGuard`): boundary declarations, not behavioral specs.

## Issues Found
1. **Medium**: Existing review (`review_r1.md`) has factual errors (misnaming `evict`, wrong coverage).
2. **Low**: `get`/`put` external_body leaves counter-overflow behavior as trust assumption (already documented in bugs/trust docs).
3. **Low**: `find_lru_victim`/`cache_lru_of` link is abstraction-heavy (acceptable but trusted).

## Conclusion
**PASS (with documentation corrections required).**

Trust boundary appears minimal under current no_std + Verus limitations and source-integrity constraints. I found no clear removable external_body that reduces net trust without comparable replacement. However, the prior review document contains concrete factual inaccuracies (notably `evict` vs `find_lru_victim`, and `8/9` vs `9/10`) and should be corrected.
