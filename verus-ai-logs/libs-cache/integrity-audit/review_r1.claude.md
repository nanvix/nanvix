# Independent Review: cache (claude-opus-4.6)

**Reviewer:** claude-opus-4.6 (independent audit)
**Date:** 2026-04-23
**Crate:** `src/libs/cache`
**vstd version:** 0.0.0-2026-04-12-0118

## 1. Cheating Detection

### make verify-cache output

Verus verification: **PASS** (exit code 0, cached/no recompilation).
The `make` wrapper returns exit code 1 due to `CHEATING_DETECTED` (external_body count > 0).

| Item | My Count | fix_report Count | Match? |
|------|----------|-----------------|--------|
| admit() | 0 | 0 | ✅ |
| assume() | 0 | 0 | ✅ |
| external_body | 8 | 8 | ✅ |
| trusted | 0 | 0 | ✅ |
| no_decreases | 0 | 0 | ✅ |
| cfg-gated exec | 0 | 0 | ✅ |
| assume_specification | 5 | 5 | ✅ |

### Manual search results

**admit()/assume():** None found in any source file. Confirmed.

**trusted:** None found. Confirmed.

**exec_allows_no_decreases_clause:** None found. Confirmed.

**cfg-gated code:** Four `#[cfg]` uses found, all legitimate:
- `lib.rs:50,52,54` — `#[cfg(verus_keep_ghost)]` for `include!()` of spec/proof/vstd files. These are ghost-only includes, not exec code.
- `lib.rs:367` — `#[cfg(all(test, feature = "std"))]` for test module. Standard test gating.

No cfg-gated exec code found. Confirmed.

**external_body (8):**
1. `lib.rs:93` — `CacheGuard::deref`
2. `lib.rs:114` — `btreemap_remove`
3. `lib.rs:190` — `Cache::get`
4. `lib.rs:230` — `Cache::put`
5. `lib.rs:315` — `Cache::find_lru_victim`
6. `lib.spec.rs:24` — `ExCacheGuard` (type spec)
7. `lib.vstd_btree.rs:32` — `ExBTreeMap` (type spec)
8. `lib.proof.rs:401` — `axiom_cache_lru_of_remove`

**assume_specification (5):**
1. `BTreeMap::new` (vstd_btree.rs:69)
2. `BTreeMap::len` (vstd_btree.rs:88)
3. `BTreeMap::is_empty` (vstd_btree.rs:98)
4. `BTreeMap::insert` (vstd_btree.rs:108)
5. `BTreeMap::clear` (vstd_btree.rs:130)

**Additional trust surface (not counted as cheating):**
- 2 broadcast axioms (axiom_btree_map_view_finite_dom, axiom_spec_btree_map_len)
- 4 uninterp spec fns (btreemap_view_spec, spec_btree_map_len, cache_lru_of_nonempty, CacheGuard::view)
- 1 implicitly unverified function (CacheGuard::deref_mut — no `#[verus_verify]`, returns `&mut V`)

**Verdict:** Counts match fix_report exactly. No hidden cheating found.

## 2. Trust Item Challenge

### Item 1: ExBTreeMap (vstd_btree.rs:32) — EXTERNAL_TYPE

**Classification: CORRECT.** BTreeMap is from `alloc`, not defined in this crate. `external_type_specification` + `external_body` is the only way to reference it in verified code. Cannot be eliminated.

### Item 2: ExCacheGuard (spec.rs:24) — VERUS_LIMITATION

**Classification: CORRECT.** `CacheGuard<'a, V>` contains `value: &'a mut V`. Verus error: "does not yet support &mut types". Cannot be eliminated until Verus adds &mut field support.

**Could it be eliminated?** Only by removing `CacheGuard` entirely and having `get()` return `Option<&V>` (immutable ref). This would change the public API — callers lose write-through-guard capability. Not a viable elimination.

### Item 3: btreemap_remove (lib.rs:114) — STDLIB_WRAPPER

**Classification: CORRECT.** Verified in vstd: `BTreeMap::remove` uses `Borrow<Q>` generic which can't be monomorphized with `assume_specification` on `alloc::collections::BTreeMap`. The upstream vstd uses uninterpreted helpers (`borrowed_key_removed`) tied to `std::collections::BTreeMap`. This wrapper fixes Q=K. Body is single stdlib call. Cannot be eliminated.

### Item 4: CacheGuard::deref (lib.rs:93) — VERUS_LIMITATION

**Classification: CORRECT.** Cascading from ExCacheGuard being external_body — field access is opaque. Cannot be eliminated without eliminating Item 2 first.

### Item 5: Cache::get (lib.rs:190) — VERUS_LIMITATION

**Classification: CORRECT.** I verified:
- `get_mut` has **no spec in any vstd version** (searched all of `vstd-0.0.0-2026-04-12-0118/std_specs/`).
- Even if it did, it returns `Option<&mut V>`, which Verus cannot handle.

**Could it be eliminated via restructuring?** Theoretically, `get()` could be rewritten as:
```rust
if btreemap_contains_key(&self.entries, key) {
    // bump counter, clone value out, return it wrapped
}
```
However: (a) `contains_key` has the same `Borrow<Q>` issue, requiring another wrapper; (b) getting the value requires `BTreeMap::get` which also has `Borrow<Q>`; (c) the original code constructs `CacheGuard` with `&mut entry.value` — this fundamentally requires `get_mut`'s `&mut` return. Even a restructured version would need `external_body` because constructing `CacheGuard` requires a `&mut V` reference. **Cannot be eliminated.**

### Item 6: Cache::put (lib.rs:230) — VERUS_LIMITATION

**Classification: CORRECT.** Same `get_mut` blockers as Cache::get.

**Could it be eliminated via contains_key + remove + insert?** Yes, the existing-key path could be rewritten as `remove` + `insert` instead of in-place mutation. However:
1. This changes exec semantics (two tree traversals instead of one; temporary removal of the key).
2. A `btreemap_contains_key` wrapper would be needed (same `Borrow<Q>` issue).
3. The code also needs `self.entries.len() >= self.capacity` check, which already uses the assumed `len()` spec.

**Assessment: PARTIALLY ELIMINABLE.** The existing-key path *could* be restructured to avoid `get_mut` using `btreemap_contains_key` + `btreemap_remove` + `BTreeMap::insert`. This would add one more `STDLIB_WRAPPER` (`btreemap_contains_key`) but eliminate `Cache::put`'s `external_body`. The tradeoff: accepting a VERUS REWRITE (in-place mutation → remove+insert) to reduce trust. The fix_report correctly identifies this as violating source integrity. **I agree with keeping external_body here** — the rewrite changes observable behavior (different allocation patterns, briefly missing key) and the spec faithfully describes what the body does.

### Item 7: find_lru_victim (lib.rs:315) — VERUS_LIMITATION

**Classification: CORRECT.**

**Could a manual loop with vstd iterator specs work?** No:
1. vstd's BTreeMap iterator specs (`ForLoopGhostIteratorNew` for `btree_map::Iter`) are gated behind `cfg(all(feature = "alloc", feature = "std"))` in `vstd::std_specs::mod.rs:17-18`. Unavailable on no_std.
2. Even if available, `min_by_key` has **no vstd spec anywhere** (searched entire vstd crate).
3. A manual loop would still need iterator creation (no spec on no_std) and would require proving min-finding correctness — the same fundamental issue.

**Cannot be eliminated.**

### Item 8: axiom_cache_lru_of_remove (proof.rs:401) — VERUS_LIMITATION

**Classification: CORRECT.**

**Could this be proven rather than axiomatized?** No. The axiom relates `cache_lru_of(new_entries) == cache_lru_of(old_entries).filter(|k| k != key)`. The function `cache_lru_of` delegates to `cache_lru_of_nonempty` which is `uninterp spec fn` — it's opaque by design. To prove the axiom, you would need:
1. A constructive definition of `cache_lru_of` (sorting entries by `last_used`).
2. This requires iterating BTreeMap entries in spec — impossible because BTreeMap is opaque (`external_body`).
3. Even with a constructive definition, proving that removing an entry preserves sort order of remaining entries requires inductive reasoning over the sorted sequence.

The axiom is **sound** (removing an entry from a BTreeMap doesn't change other entries' `last_used` counters), but **unprovable** with the current abstraction level. **Cannot be eliminated.**

### assume_specification Items (5)

All 5 are `EXTERNAL_BOTTOM` — specs for `alloc::collections::BTreeMap` methods mirroring upstream vstd specs. The btree module is confirmed gated behind `cfg(all(feature = "alloc", feature = "std"))` in `vstd::std_specs::mod.rs:17-18`.

**Fidelity concern:** Two specs (`BTreeMap::insert`, `BTreeMap::len` axiom) drop upstream `obeys_cmp_spec` guards, making them unconditionally stronger. This is an additional trust assumption. Fix_report correctly identifies this. Low practical risk but should be documented (and it is, in trust.md).

## 3. AST Consistency

### AST checker results

- **Matched:** 15/18
- **Mismatched:** 3
- **Missing:** 0
- **Extra:** 2
- **Consistent:** NO (expected — verified code has proof blocks)

### Mismatch 1: Cache::new

**Diff:** `Self { ... }` → `let result = Self { ... }; proof!{...} result`

**Analysis:** Pre-approved deviation. Return value binding is needed to reference `result` in the `ensures` clause and proof block. `proof!{}` is erased at compile time. The exec semantics are identical (same struct construction, same field values). **JUSTIFIED.**

Fix_report classification: **CORRECT.**

### Mismatch 2: Cache::remove

**Diff:** `self.entries.remove(key)` → `btreemap_remove(&mut self.entries, key)` + proof block.

**Analysis:** VERUS REWRITE. `BTreeMap::remove`'s `Borrow<Q>` generic cannot be expressed with `assume_specification` for `alloc::collections::BTreeMap`. The wrapper fixes Q=K. Both calls have identical runtime behavior (single stdlib `remove` call). Proof block erased at compile time. **JUSTIFIED.**

**Could the original exec code be preserved?** Only if `assume_specification` could handle the `Borrow<Q>` bound on `alloc::collections::BTreeMap`. Upstream vstd's solution uses uninterpreted helpers (`borrowed_key_removed`) tied to `std::collections::BTreeMap` — a different type. Porting those helpers to `alloc::collections::BTreeMap` would require either (a) reimplementing the full `Borrow<Q>` machinery or (b) the orphan rule prevents implementing View for BTreeMap. The wrapper is the simplest correct solution. **Cannot be preserved.**

Fix_report classification: **CORRECT.**

### Mismatch 3: Cache::evict

**Diff:** Inline iterator chain → `Self::find_lru_victim(&self.entries)` + `btreemap_remove(...)` + proof block.

**Analysis:** VERUS REWRITE. The iterator chain `iter().min_by_key(...)` is unverifiable (no vstd specs for `min_by_key`, no no_std iterator specs). Extracting into `find_lru_victim` (external_body) isolates the unverifiable code. `btreemap_remove` replaces `self.entries.remove` for the same `Borrow<Q>` reason. **JUSTIFIED.**

**Could the original exec code be preserved?** No. The iterator chain and `min_by_key` closure have no vstd specs. Without external_body on the entire `evict` function, the chain cannot be verified. The current approach isolates only the unverifiable part (victim finding) while keeping eviction logic (remove victim) verified. This is strictly better than making all of `evict` external_body. **Cannot be preserved without expanding the trust surface.**

Fix_report classification: **CORRECT.**

### Extra functions

1. **Cache::find_lru_victim** — Extracted from evict. Isolates unverifiable iterator chain. **JUSTIFIED.**
2. **btreemap_remove** — Stdlib wrapper for `Borrow<Q>` limitation. **JUSTIFIED.**

## 4. Verification

```
make verify-cache: Exit code 0 (Verus itself)
```

Verification passes with 0 errors. The `make` target returns exit code 1 due to the cheating detection wrapper detecting 8 external_body items — this is expected behavior, not a verification failure.

Verified function count: 9/10 exec functions have contracts. The unverified function (`CacheGuard::deref_mut`) returns `&mut V`, which Verus cannot spec.

## 5. Bug vs Limitation Analysis

### external_body: btreemap_remove (lib.rs:114-123)

**Body:** `m.remove(k)` — single stdlib call.
**Spec:** `btreemap_view_spec(*m) == old(*m).remove(*k)`, returns removed value if present.
**Assessment:** Body exactly matches spec semantics. No bugs. **GENUINE LIMITATION** (Borrow<Q> issue).

### external_body: CacheGuard::deref (lib.rs:93-99)

**Body:** `self.value` — field access.
**Spec:** `*ret == self@` — dereferencing yields abstract value.
**Assessment:** Trivially correct. **GENUINE LIMITATION** (cascading from CacheGuard external_body).

### external_body: Cache::get (lib.rs:190-218)

**Body logic review:**
1. `get_mut(key)` → returns `Option<&mut CacheEntry<V>>`. Correct.
2. On hit: `self.counter += 1` — increment LRU counter. ⚠️ No overflow check.
3. `entry.last_used = self.counter` — bump entry recency. Correct.
4. Construct `CacheGuard { value: &mut entry.value }`. Correct.
5. On miss: return `None`. Correct.

**Spec consistency:** Spec says hit returns `spec_get(*key).1.unwrap()` and state transitions via `spec_get`. The abstract `spec_get` moves key to MRU position and returns the value. The body does the same (bumps counter = MRU). Consistent.

**Assessment:** No logic bugs. Counter overflow is documented in BUG-1. **GENUINE LIMITATION.**

### external_body: Cache::put (lib.rs:230-265)

**Body logic review:**
1. Zero-capacity check → return. Correct.
2. `get_mut(&key)` for existing-key update → update value + bump counter. Correct.
3. Evict if at capacity. Correct (calls `self.evict()`).
4. Insert new entry with bumped counter. Correct.

**Spec consistency:** Spec says `self@ == old(self)@.spec_put(key, value)`. The abstract `spec_put` handles zero-capacity, existing-key, at-capacity+new, below-capacity+new cases. Body handles the same four cases in the same order. Consistent.

**Potential issue:** The `entries.len() >= self.capacity` check uses `>=` but the spec uses `self.contents.dom().len() >= self.capacity`. These are consistent because `entries.len()` maps to `btreemap_view_spec.len()` via the assume_specification. No off-by-one.

**Assessment:** No logic bugs. **GENUINE LIMITATION.**

### external_body: find_lru_victim (lib.rs:315-331)

**Body:** `entries.iter().min_by_key(|(_, e)| e.last_used).map(|(k, _)| k.clone())`
**Spec:** Returns `cache_lru_of(*entries)[0]` (the LRU key) when non-empty, `None` when empty.
**Assessment:** `min_by_key` on `last_used` correctly finds the entry with the smallest counter — the least recently used. `clone()` on the key is correct (K: Clone). The spec connects to the abstract `cache_lru_of` ordering. No bugs. **GENUINE LIMITATION.**

### external_body: axiom_cache_lru_of_remove (proof.rs:401-411)

**Axiom:** `cache_lru_of(new) == cache_lru_of(old).filter(|k| k != key)` when `btreemap_view_spec(new) == btreemap_view_spec(old).remove(key)`.
**Soundness:** BTreeMap::remove doesn't modify other entries' `last_used` counters. Removing a key from the sorted order and filtering that key from the sorted sequence produce the same result. Sound.
**Assessment:** **GENUINE LIMITATION.** Cannot be proven because `cache_lru_of` is uninterpreted.

### BUG-1: Counter overflow (u64)

**Classification in bugs.md:** UNCONFIRMED, LOW severity, physically unreachable.

**My assessment:** Classification is **CORRECT**. This is a real code defect — `self.counter += 1` can overflow after 2^64 operations, corrupting LRU ordering. However:
- At realistic operation rates, overflow is physically unreachable (~58 years at 10 billion ops/sec).
- The spec uses abstract `Seq` ordering (not counters), so the spec is correct regardless.
- The gap exists only in the `external_body` trust boundary — the body may not satisfy the spec after overflow, but the spec itself is sound.
- Adding `requires self.counter < u64::MAX` is possible but would burden all callers with an unprovable obligation.

**Recommendation:** Correctly classified as LOW/UNCONFIRMED. Could optionally add a `debug_assert!(self.counter < u64::MAX)` for defense-in-depth, but this doesn't affect verification.

## 6. Conclusion

### Summary

| Category | Status |
|----------|--------|
| Cheating counts | ✅ Match fix_report exactly |
| Trust item classifications | ✅ All correct |
| AST consistency | ✅ All mismatches justified |
| Verification | ✅ Passes (exit 0) |
| Bug classification | ✅ BUG-1 correctly classified |
| Spec fidelity | ⚠️ Two assume_specs drop upstream guards (documented) |

### Could any items have been eliminated?

**No.** After thorough challenge of each trust item:

1. **ExBTreeMap** — Cannot eliminate (external type).
2. **ExCacheGuard** — Cannot eliminate (Verus &mut limitation).
3. **btreemap_remove** — Cannot eliminate (Borrow<Q> limitation).
4. **CacheGuard::deref** — Cannot eliminate (cascading from ExCacheGuard).
5. **Cache::get** — Cannot eliminate (no get_mut spec + &mut return + CacheGuard construction).
6. **Cache::put** — Theoretically restructurable (contains_key + remove + insert), but would require VERUS REWRITE changing exec semantics (temporary key absence, different allocation patterns). The current approach preserves source integrity. **Acceptable as-is.**
7. **find_lru_victim** — Cannot eliminate (no min_by_key spec, no no_std iterator specs).
8. **axiom_cache_lru_of_remove** — Cannot eliminate (uninterpreted cache_lru_of).

### Overall Verdict: **PASS**

The trust boundary is well-documented, minimal, and each item is genuinely irreducible given current Verus limitations. The primary improvement opportunity would be restructuring `Cache::put` to avoid `get_mut`, but this is a source-integrity tradeoff with marginal trust reduction (would still need wrappers). The existing classification and documentation are thorough and accurate.

The verification is sound, with the following acknowledged trust assumptions:
1. Eight `external_body` items with manually-reviewed spec/body consistency.
2. Five `assume_specification` items mirroring upstream vstd (two with dropped guards).
3. One unverified function (`deref_mut`) due to `&mut` return type limitation.
4. Counter overflow assumption (BUG-1, physically unreachable).
