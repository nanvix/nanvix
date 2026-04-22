# Independent Integrity Audit Review — GPT

## Cheating Item Counts
Verified by direct inspection of `lib.rs`, `lib.spec.rs`, `lib.proof.rs`, `lib.vstd_btree.rs`:

- `admit()`: **0**
- `assume()`: **0**
- `external_body`: **8** ✅
  - Functions: `CacheGuard::deref`, `btreemap_remove`, `Cache::get`, `Cache::put`, `Cache::evict`, `axiom_cache_lru_of_remove`
  - Type specs with body: `ExCacheGuard`, `ExBTreeMap`
- `trusted`: **0**
- `exec_allows_no_decreases_clause`: **0**
- cfg-gated exec code: **0** (only ghost includes and test module cfgs observed)
- `assume_specification` in `lib.vstd_btree.rs`: **5** ✅
- `broadcast axiom` in `lib.vstd_btree.rs`: **2** ✅

## Challenge Results
1. `btreemap_remove` — **KEEP**. Wrapper is a narrow trust shim around `remove`; replacing with broader `assume_specification` for `remove::<Q>` on `alloc::BTreeMap` would not reduce trust.
2. `CacheGuard::deref` — **KEEP**. Depends on opaque `CacheGuard` (`&mut V` field limitation).
3. `Cache::get` — **KEEP**. Fundamental blocker is `CacheGuard`/`&mut` modeling; not just `get_mut`.
4. `Cache::put` — **KEEP**. `remove+insert` rewrite is possible semantically, but is a structural exec rewrite (source-integrity violation for this audit target).
5. `Cache::evict` — **KEEP**. Current body uses iterator combinators (`min_by_key`, `map`) without usable specs here; manual-loop rewrite would alter exec code.
6. `axiom_cache_lru_of_remove` — **KEEP (high-risk trust)**. Could be provable only with a fully interpreted LRU-order spec and substantial new proof machinery.
7. `ExBTreeMap` — **KEEP**. Needed on this no_std path.
8. `ExCacheGuard` — **KEEP**. Verus limitation on `&mut` in struct fields.
9. `ExCacheEntry` — **KEEP**. Needed to reason about private internal type in specs.

## AST Consistency Analysis
Given mismatch set (2 mismatches + 1 extra), all are legitimate:
- `Cache::new`: named return + ghost proof block only; exec semantics unchanged.
- `Cache::remove`: `self.entries.remove(key)` replaced by one-call wrapper `btreemap_remove(...)` plus ghost proof; semantically equivalent stdlib wrapper deviation.
- Extra `btreemap_remove`: intentional wrapper, single-call body.

## Bug vs Limitation
- `CacheGuard::deref`: limitation, no bug evidence.
- `btreemap_remove`: limitation/workaround, no bug evidence.
- `Cache::get`: limitation; logic appears correct modulo counter overflow assumption.
- `Cache::put`: limitation; logic appears correct modulo counter overflow assumption.
- `Cache::evict`: limitation; victim selection correct if `last_used` monotonic (no overflow).
- `axiom_cache_lru_of_remove`: limitation but trust-sensitive (axiom could mask spec inconsistency if abused).

## vstd Search Results
From `~/.cargo/registry/src/.../vstd-0.0.0-2026-04-05-0114/std_specs`:
- `std_specs/mod.rs` gates btree behind `#[cfg(all(feature = "alloc", feature = "std"))]`.
- `btree.rs` has specs for `contains_key`, `get`, `remove`, `iter`, `keys`, `values`.
- `get_mut` spec: **not found**.
- Therefore: upstream has broad `std`-BTreeMap support, but this crate’s no_std adaptation intentionally carries only a subset (5 assumes + 2 axioms).

## Issues Found
1. **Axiom risk (highest):** `axiom_cache_lru_of_remove` is unproven trusted glue over an uninterpreted LRU function.
2. **Model/runtime gap:** counter overflow (`u64`) can break concrete LRU ordering; documented but still trusted.
3. **Documentation precision:** claims about missing BTreeMap iteration support should explicitly say “in local no_std adaptation”, not globally in upstream vstd.

## Conclusion
**PASS** (strict integrity outcome): no clearly eliminable trust item was found without source-integrity-breaking exec rewrites or enlarging trust elsewhere. Trust boundary is not zero, but appears locally minimal under current Verus/no_std constraints.
