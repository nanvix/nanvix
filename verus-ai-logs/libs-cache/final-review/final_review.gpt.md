# Final Review — Verus Verification for `src/libs/cache`

## Executive Summary
The cache verification is strong on proof completeness and invariant-preserving transitions, but it is **not guardrail-clean** due to required `external_body` trust points in user-facing logic (`get`, `put`, guard deref path, wrapper/helper) and one unverified exec API (`deref_mut`). Verus proof status is green (22 verified, 0 errors), but release quality remains **conditional on human trust review**.

---

## 1) Spec Quality — **FAIL**

**What is good**
- Public API transitions are specified against `CacheView` using `spec_new/spec_get/spec_put/spec_remove/spec_clear` and invariant preservation.
- Success and failure/no-op paths are meaningful (e.g., `get` hit/miss split, `remove` no-op via `spec_remove`, zero-capacity `put` no-op via `spec_put`).
- Specs are readable and mostly caller-usable (map+LRU abstraction, capacity invariant).
- No obvious tautological ensures in core API specs.

**Why FAIL**
- `CacheGuard::deref_mut` has no Verus contract/verification (known `&mut` limitation), leaving mutation-through-guard semantics outside formal coverage.
- `cache_lru_of` for non-empty maps is uninterpreted and linked via one axiom; this is acceptable but increases non-local trust burden on reviewer comprehension.

---

## 2) Caller Coverage — **FAIL**

Compared against `caller_analysis.md` expectations for:
`new/get/put/remove/clear/deref/deref_mut/evict`.

- **Covered:** all functional expectations for `new/get/put/remove/clear/deref/evict` via contracts + transition lemmas.
- **Partially/uncovered:** `deref_mut` persistence expectation (mutation through guard reflected in cache) is not formally specified/verified.

**Coverage count:** **21 / 22 expectations covered** (~95.5%), with the missing one being the `deref_mut` guarantee.

---

## 3) Proof Completeness — **PASS**

- Remaining `admit()`: **0**.
- Invariant-preservation lemmas (`lemma_spec_new_inv/get_inv/put_inv/remove_inv/clear_inv`) are present and completed.
- No incomplete proof stubs found.

---

## 4) Trust Minimization — **FAIL**

Trust set is well-documented and mostly justified, but still too large for unconditional approval.

### external_body items challenged
1. `btreemap_remove` (stdlib wrapper) — justified by `Borrow<Q>` monomorphization limitation for `alloc::BTreeMap` specs.
2. `CacheGuard::deref` — justified because guard type is opaque (`&mut` field limitation).
3. `Cache::get` — justified (`get_mut` spec gap + `Option<&mut ...>` limitation + guard construction).
4. `Cache::put` — justified (same `get_mut` blocker on overwrite path).
5. `find_lru_victim` — justified (`iter/min_by_key` no no_std-verifiable path here without major rewrite).
6. `axiom_cache_lru_of_remove` (proof fn) — plausible and narrowly scoped, but still an axiom.
7. `ExCacheGuard` / `ExBTreeMap` external bodies — external type encapsulation, expected.

### assume_specification faithfulness
- 5 items in `lib.vstd_btree.rs` (`new`, `len`, `is_empty`, `insert`, `clear`).
- Trust audit correctly notes that local `len` axiom/`insert` drop upstream `obeys_cmp_spec`-style guards, making local assumptions stronger than upstream vstd.

**Conclusion:** minimal for current constraints, but not minimal enough for a no-trust boundary standard.

---

## 5) AST Consistency — **PASS**

- **15 matched, 3 mismatched, 0 missing, 2 extra**.
- `Cache::new` mismatch (named binding for proof call) is pre-approved and semantically equivalent.
- `Cache::remove` mismatch (wrapper+proof) is justified Verus rewrite for `Borrow<Q>` limitation.
- `Cache::evict` mismatch (helper extraction + wrapper) is justified Verus rewrite for iterator/min-by-key limits.
- Extras (`find_lru_victim`, `btreemap_remove`) are justified helper/wrapper additions.

No AST mismatch appears to be an unapproved semantic drift.

---

## 6) Verification Status — **PASS**

- Verus: **22 verified, 0 errors** (ground truth).
- `make verify-cache`: non-zero only due to `CHEATING_DETECTED` from expected `external_body` count; not a Verus proof failure.

---

## 7) Guardrails Compliance — **FAIL**

- `admit`: **0** ✅
- `assume`: **0** ✅
- `trusted`: **0** ✅
- `no_decreases`: **0** ✅
- `cfg_gate` (exec): **0** ✅
- `assume_specification`: **5** (documented no_std bridge) ⚠️
- `external_body`: **8** ⚠️
  - `axiom_cache_lru_of_remove`
  - `CacheGuard::deref`
  - `btreemap_remove`
  - `Cache::get`
  - `Cache::put`
  - `find_lru_victim`
  - `ExCacheGuard` (type)
  - `ExBTreeMap` (type)

Per stated policy, **external_body on user functions is a blocker requiring human review**.

---

## 8) Bug Reconciliation — **PASS**

- **BUG-1 counter overflow** remains technically valid and correctly classified as low severity / practically unreachable assumption.
- Current specs/proofs do not model overflow prevention; trust note is appropriate.
- No additional concrete implementation bugs were identified from this review beyond recorded trust/coverage gaps.

---

## Blockers

1. **Guardrail blocker:** `external_body` on user functions (`Cache::get`, `Cache::put`, `CacheGuard::deref`, `btreemap_remove`, `find_lru_victim`) plus proof axiom reliance.
2. **Coverage blocker:** `CacheGuard::deref_mut` remains unverified and uncontracted (Verus `&mut` return limitation).

---

## Overall Assessment

**Overall verdict: FAIL (conditional acceptance only).**

This effort is high quality and well-documented under current Verus/no_std constraints, with complete non-admit proofs and justified AST deviations. However, by strict review gates, it cannot be marked fully passing because key behavior still crosses a trusted boundary (`external_body`) and one API contract (`deref_mut`) is missing from formal verification.
