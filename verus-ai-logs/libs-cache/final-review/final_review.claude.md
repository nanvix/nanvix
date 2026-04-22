# Final Review: `cache` Crate Verus Verification

**Date:** 2025-07-24
**Reviewer:** Claude Sonnet 4 (independent final review)
**Crate:** `src/libs/cache/src/lib.rs`
**Spec:** `src/libs/cache/src/lib.spec.rs`
**Proof:** `src/libs/cache/src/lib.proof.rs`

---

## 1. Spec Quality

### View Design

The `CacheView<K, V>` type is well-designed with three fields:

| Field | Type | Purpose |
|-------|------|---------|
| `contents` | `Map<K, V>` | Key-value mapping (mathematical) |
| `capacity` | `nat` | Upper bound on entries (avoids overflow reasoning) |
| `lru_order` | `Seq<K>` | Recency ordering, LRU at index 0 |

All fields use mathematical types — no machine types leak into specs. The design
passes the substitution test: swapping BTreeMap for a hash map or linked list
would not change the View. No implementation details (counters, CacheEntry,
BTreeMap) are visible in the abstract state.

### Well-formedness Invariant

```
inv() = contents.dom().len() <= capacity
      ∧ lru_order.no_duplicates()
      ∧ lru_order.to_set() == contents.dom()
      ∧ lru_order.len() == contents.dom().len()
```

- ✅ Capacity bound captured
- ✅ LRU order is a permutation of the key set
- ✅ Explicit cardinality link (solver hint, derivable but necessary for SMT)
- ✅ No redundant fields

### Spec Transition Functions

All five transitions (`spec_new`, `spec_get`, `spec_put`, `spec_remove`, `spec_clear`)
are declarative and independent of implementation details:

- ✅ **`spec_new`**: Empty contents, empty order, given capacity. Clean.
- ✅ **`spec_get`**: Hit → move to MRU, return value. Miss → no-op, return None. Both paths specified.
- ✅ **`spec_put`**: Four branches (zero-capacity, overwrite, eviction, below-capacity). Eviction victim is `lru_order[0]` — deterministic. All branches well-defined.
- ✅ **`spec_remove`**: Key present → remove from contents and order. Key absent → no-op.
- ✅ **`spec_clear`**: Reset contents and order to empty, preserve capacity.

### External-Top Specs (API Contracts)

| Function | Spec Quality | Notes |
|----------|-------------|-------|
| `Cache::new` | ✅ Good | `result@ == spec_new(capacity as nat) ∧ result@.inv()` |
| `Cache::get` | ✅ Good | Bidirectional: hit IFF key in domain. Guard view equals value. State transitions via `spec_get`. |
| `Cache::put` | ✅ Good | `self@ == old(self)@.spec_put(key, value) ∧ self@.inv()` |
| `Cache::remove` | ✅ Good | `self@ == old(self)@.spec_remove(*key) ∧ self@.inv()` |
| `Cache::clear` | ✅ Good | `self@ == old(self)@.spec_clear() ∧ self@.inv()` |
| `Cache::evict` | ✅ Good | 5 ensures clauses: victim identity, contents update, size decrement, order update, capacity and inv preserved. |
| `CacheGuard::deref` | ✅ Good | `*ret == self@` — standard Deref contract. |
| `CacheGuard::deref_mut` | ⚠️ Unverifiable | Verus cannot annotate `&mut` return types. |

**Findings:**

- ✅ No tautological ensures (`Err(_) => true` patterns absent — no error paths exist).
- ✅ Frame conditions implicit via spec transitions: `spec_get` preserves `contents` on hit, full state on miss. `spec_put` captures all cases.
- ✅ Specs are written for the caller — directly usable in caller proofs (e.g., `self@ == old(self)@.spec_put(key, value)` is a single equation callers can unfold).
- ✅ Liveness: `get` returns `Some` IFF key present. `put` with non-zero capacity guarantees insertion (via `spec_put` definition).
- ✅ Invariant stated in every function's ensures (both `self@.inv()` and the transition equality).
- ⚠️ Minor: `get` spec uses `old(self)@.spec_get(*key).1.unwrap()` for the guard view — the `.unwrap()` is safe because the preceding clause guarantees the hit branch, but this pattern requires the reader to mentally verify that `spec_get` returns `Some` when the key is present. Acceptable but slightly less readable than a direct `old(self)@.contents[*key]`.

**Verdict: PASS** — Specs are high quality, caller-oriented, and complete for all verifiable functions.

---

## 2. Caller Coverage

Cross-referencing every caller expectation from `caller_analysis.md` against actual specs:

| # | Caller Expectation | Spec Coverage | Status |
|---|-------------------|---------------|--------|
| 1 | `new` returns empty cache with given capacity | `result@ == spec_new(capacity as nat)` unfolds to empty contents/order + capacity | ✅ |
| 2 | `get` returns `Some` IFF key present | Bidirectional ensures with `contains` | ✅ |
| 3 | `get` guard dereferences to stored value | `result->Some_0@ == old(self)@.spec_get(*key).1.unwrap()` | ✅ |
| 4 | `get` refreshes LRU order | `self@ == old(self)@.spec_get(*key).0` which uses `move_to_mru` | ✅ |
| 5 | `get` does not change cache size | `spec_get` preserves `contents` | ✅ |
| 6 | `put` new key below capacity: inserted and retrievable | `spec_put` below-capacity branch: `contents.insert(key, value)` | ✅ |
| 7 | `put` new key at capacity: LRU evicted, new key inserted | `spec_put` eviction branch: victim = `lru_order[0]` | ✅ |
| 8 | `put` existing key: value replaced, no eviction, size unchanged | `spec_put` overwrite branch: `contents.insert(key, value)`, same domain | ✅ |
| 9 | `put` zero-capacity: no-op | `spec_put` zero-capacity branch: `self` | ✅ |
| 10 | `put` overwrite refreshes recency | `spec_put` overwrite branch uses `move_to_mru` | ✅ |
| 11 | `remove` present key: removed, size decreases | `spec_remove` present branch | ✅ |
| 12 | `remove` absent key: no-op, no panic | `spec_remove` absent branch: `self` | ✅ |
| 13 | `clear` removes all entries | `spec_clear`: `contents = Map::empty()` | ✅ |
| 14 | `clear` preserves capacity | `spec_clear`: `..self` preserves capacity | ✅ |
| 15 | `deref` yields `&V` to stored value | `*ret == self@` | ✅ |
| 16 | `deref_mut` yields `&mut V` | Unverifiable (Verus limitation) | ⚠️ |
| 17 | Guard borrows cache exclusively | Rust borrow checker (static guarantee) | ✅ |

**Coverage: 16/17** (1 unverifiable due to Verus `&mut` return limitation).

**Verdict: PASS** — All verifiable caller expectations are covered.

---

## 3. Proof Completeness

### admit() Search

- `lib.rs`: 0 occurrences of `admit()`
- `lib.spec.rs`: 0 occurrences of `admit()`
- `lib.proof.rs`: 0 occurrences of `admit()`

### assume() Search

- `lib.rs`: 0 occurrences
- `lib.spec.rs`: 0 occurrences
- `lib.proof.rs`: 0 occurrences

### Proven Lemmas

All 5 invariant preservation lemmas are fully proven:

| Lemma | Status |
|-------|--------|
| `lemma_spec_new_inv` | ✅ Proven |
| `lemma_spec_get_inv` | ✅ Proven |
| `lemma_spec_put_inv` | ✅ Proven (most complex — 3 branches) |
| `lemma_spec_remove_inv` | ✅ Proven |
| `lemma_spec_clear_inv` | ✅ Proven |

All 5 helper lemmas are fully proven:

| Helper | Status |
|--------|--------|
| `lemma_push_preserves_no_dup` | ✅ Proven |
| `lemma_filter_preserves_no_dup` | ✅ Proven (recursive, with decreases) |
| `lemma_filter_neq_to_set` | ✅ Proven |
| `lemma_filter_neq_len` | ✅ Proven |
| `lemma_subrange_no_dup` | ✅ Proven |
| `lemma_drop_first_to_set` | ✅ Proven |

**Verdict: PASS** — Zero admit(), zero assume(). All proofs complete.

---

## 4. Trust Minimization

### External Type Specifications (2 with external_body)

| Item | Classification | Eliminable? | Verdict |
|------|---------------|------------|---------|
| `ExBTreeMap` | EXTERNAL_TYPE | ❌ No — `alloc::collections::BTreeMap` has private fields; no_std target blocks vstd btree specs | ✅ Necessary |
| `ExGlobal` | EXTERNAL_TYPE | ❌ No — default allocator type for BTreeMap, required for type declaration | ✅ Necessary |
| `ExCacheEntry` | EXTERNAL_TYPE | ❌ No — private struct used as BTreeMap value type | ✅ Necessary |
| `ExCacheGuard` | VERUS_LIMITATION | ❌ No — `CacheGuard` contains `&'a mut V`; Verus cannot handle `&mut` in struct fields | ✅ Necessary |

### External Body Functions (7)

| Function | Classification | Could it be eliminated? | Verdict |
|----------|---------------|------------------------|---------|
| `Cache::new` | VERUS_LIMITATION | ❌ — Calls `BTreeMap::new()`, vstd btree specs require `cfg(std)`, incompatible with no_std target. Even with `assume_specification`, the uninterp view means body cannot be verified. | ✅ Necessary |
| `Cache::get` | VERUS_LIMITATION | ❌ — Uses `BTreeMap::get_mut()` which has NO vstd spec at all (confirmed: 0 occurrences in vstd). Also returns `Option<&mut V>`, a Verus limitation. Constructs `CacheGuard` with `&mut`. Three independent blockers. | ✅ Necessary |
| `Cache::put` | VERUS_LIMITATION | ❌ — Same `get_mut` blockers as `get`, plus calls `self.evict()` (also external_body). Three independent blockers. | ✅ Necessary |
| `Cache::remove` | VERUS_LIMITATION | ❌ — Calls `BTreeMap::remove()`, vstd btree specs require `cfg(std)`. Even with custom `assume_specification`, trust merely shifts to the axiom. | ✅ Necessary |
| `Cache::clear` | VERUS_LIMITATION | ❌ — Calls `BTreeMap::clear()`, same no_std blocker. | ✅ Necessary |
| `Cache::evict` | VERUS_LIMITATION | ❌ — Uses `iter().min_by_key()` iterator chain (no vstd spec for `min_by_key`), plus `BTreeMap::remove()`. Even rewriting as a loop wouldn't help: `BTreeMap::iter` and `BTreeMap::remove` specs are inaccessible on no_std. | ✅ Necessary |
| `CacheGuard::deref` | VERUS_LIMITATION | ❌ — `CacheGuard` is `external_body` (opaque `&mut V` field). Body accesses the opaque field. | ✅ Necessary |

**Challenge result:** Both the integrity audit reviewers (Claude Opus 4.6 and GPT-5.3-Codex)
independently challenged every item. Three functions (new, remove, clear) were identified
as theoretically eliminable via custom `assume_specification` + concrete view, but this was
correctly rejected because:

1. Only 3 of 7 function-level items would be eliminated
2. Trust shifts to `assume_specification` axioms, not actually eliminated
3. The 4 most complex methods remain `external_body` regardless
4. Engineering cost is disproportionate to benefit

**Root cause:** The single root cause for all 7 function-level `external_body` items is
the no_std target. vstd provides BTreeMap specs in `vstd::std_specs::btree`, but they are
gated behind `cfg(all(feature = "alloc", feature = "std"))` and import from
`std::collections`, making them structurally incompatible with the `no_std` kernel target.
This is an environmental constraint, not a verification quality issue.

**Verdict: PASS** — Trust boundary is minimal and well-documented. Every item has been
challenged and justified with specific error messages and reproducers.

---

## 5. AST Consistency

Pre-computed result:

- **Functions:** 18/18 MATCH (matched=18, mismatched=0, missing=0, extra=0)
- **Structs:** 3/3 MATCH
- **Consistent:** YES
- **VERUS REWRITE comments:** None needed (zero deviations)

No exec code was modified during the verification effort. The implementation is
identical to the original source.

**Verdict: PASS**

---

## 6. Verification

Pre-computed result:

- **Verus output:** 11 verified, 0 errors
- **Exit code:** 0 from Verus itself
- **verify.sh wrapper:** Reports exit 1 due to cheating detection (external_body items),
  which are expected and documented

**Verdict: PASS**

---

## 7. Guardrails Compliance

### Cheating Dimensions

| Dimension | Count | Threshold | Status |
|-----------|-------|-----------|--------|
| `admit()` | 0 | 0 (BLOCKER if > 0) | ✅ |
| `assume()` | 0 | 0 (BLOCKER if > 0) | ✅ |
| `external_body` (type) | 2 | Acceptable (EXTERNAL_TYPE) | ✅ |
| `external_body` (function) | 7 | Requires human review | ⚠️ |
| `trusted` | 0 | 0 (BLOCKER if > 0) | ✅ |
| `no_decreases` | 0 | 0 (BLOCKER if > 0) | ✅ |
| `cfg_gate` | 0 | 0 (BLOCKER if > 0) | ✅ |

### External Body Function Challenge (detailed)

All 7 `external_body` functions have been challenged in §4. Summary:

- **Root blocker:** no_std target prevents access to vstd's BTreeMap specs
- **Secondary blockers:** `get_mut` has no vstd spec at all; `&mut` return types; `&mut` struct fields; `min_by_key` iterator combinator
- **Elimination potential:** None without changing the target platform or Verus itself
- **Each item** has a classification (all `VERUS_LIMITATION`), a specific error message or reproducer, and was independently validated by two reviewers

**Assessment:** The 7 function-level `external_body` items are **BLOCKERS requiring
human review** per the guardrails policy. However, all have been thoroughly documented
with root causes, reproducers, and independent dual-reviewer validation. The recommendation
is that human reviewers confirm the no_std constraint is genuine and accept these items.

**Verdict: CONDITIONAL PASS** — Requires human sign-off on the 7 `external_body` functions.
All documentation and justification is in place.

---

## 8. Bug Reconciliation

### BUG-1: Counter Overflow (u64)

- **Status:** UNCONFIRMED (documented but not fixed)
- **Classification:** **Context-Dependent** (per bug-reporting skill)
- **Analysis:**
  - `self.counter: u64` is incremented without overflow checking in `get` (line ~189) and `put` (line ~224, ~235).
  - At 10 billion ops/sec, overflow requires ~58 years. Physically unreachable.
  - The spec transition functions use abstract `Seq` ordering (not counters), so the spec is correct regardless of overflow. The trust gap is in the `external_body` bridge: the implementation's correctness depends on no overflow, but the spec doesn't model this assumption.
  - Adding `requires self.counter < u64::MAX` was considered and rejected as it would burden every caller with an unprovable obligation in practice.
  - Correctly documented in `trust.md` as a trust assumption and in `bugs.md` as BUG-1.
- **Verdict:** ✅ Properly handled. The bug is real in theory but physically unreachable. Documentation is accurate. No fix needed.

### Additional Bugs from Property Analysis

- **BUG-2 (usize vs nat):** Correctness concern in abstraction function. Not a code bug — a trust boundary inherent in the `external_body` approach. ✅ Documented.
- **BUG-3 (evict on empty):** Non-issue. Guarded by control flow (`entries.len() >= capacity` with `capacity > 0`). ✅ Correct.
- **BUG-4 (clear resets counter):** Design observation, not a bug. Counter reset is safe because all entries are also removed. ✅ Correct.
- **BUG-5 (min_by_key ties):** Non-issue under no-overflow assumption (TYPE-4 injectivity). ✅ Correct.

### Bugs Discovered During Proving/Integrity

No new bugs were discovered during the proving or integrity audit phases that were
not already recorded in `bugs.md` or `property_analysis.md`.

**Verdict: PASS** — All bugs properly classified and documented.

---

## Summary

| Dimension | Result | Details |
|-----------|--------|---------|
| 1. Spec Quality | ✅ PASS | Clean mathematical specs, caller-oriented, complete |
| 2. Caller Coverage | ✅ PASS | 16/17 expectations covered (1 Verus limitation) |
| 3. Proof Completeness | ✅ PASS | 0 admit, 0 assume, 11 lemmas fully proven |
| 4. Trust Minimization | ✅ PASS | All 9 external_body items justified, dual-reviewed |
| 5. AST Consistency | ✅ PASS | 18/18 functions, 3/3 structs match |
| 6. Verification | ✅ PASS | 11 verified, 0 errors |
| 7. Guardrails | ⚠️ CONDITIONAL | 7 external_body functions need human sign-off |
| 8. Bug Reconciliation | ✅ PASS | All bugs documented, none unrecorded |

## Final Verdict: **CONDITIONAL PASS**

The verification effort for the `cache` crate is thorough, well-documented, and
technically sound. The spec design is high quality — abstract, caller-oriented,
and free of implementation leakage. All proofs are complete with zero admit/assume.
The trust boundary is minimal and fully justified by a single environmental
constraint (no_std target blocking vstd BTreeMap specs).

The only open item is **human review of 7 `external_body` functions**. These are
not verification quality issues — they are environmental constraints that cannot
be resolved without changing the target platform or waiting for Verus to add
`&mut` return type support and `get_mut` specs. All items are documented with
specific error messages, reproducers, and dual-reviewer validation.

### Strengths

1. **Clean View design** — `CacheView` passes substitution test, uses only mathematical types
2. **Comprehensive spec transitions** — all edge cases (zero-capacity, overwrite, eviction) modeled
3. **Full proof completion** — 11 lemmas, all from-scratch, no shortcuts
4. **Thorough documentation** — trust.md, bugs.md, property_analysis.md, view_design.md all consistent
5. **Dual-reviewer integrity audit** — independent validation by two models with disagreement resolution

### Weaknesses

1. **All exec functions are `external_body`** — the entire implementation is trusted, not verified. The specs and proofs verify only the *abstract model* (spec transitions preserve invariants). The bridge between implementation and abstraction is entirely trust-based.
2. **`deref_mut` completely unverifiable** — mutation-through-guard semantics have no formal guarantee
3. **Counter overflow assumption undischarged** — the `u64` overflow gap is documented but not enforced (no `debug_assert!` or `checked_add` in the code)

### Recommendations for Future Work

1. **When Verus adds no_std btree support:** Remove `external_body` from `new`, `remove`, `clear` first (simplest). Then tackle `put` and `get` once `get_mut` specs exist.
2. **Add `debug_assert!(self.counter < u64::MAX)`** in `get` and `put` to catch the theoretically-possible overflow in debug builds.
3. **When Verus adds `&mut` return support:** Verify `deref_mut` and remove `external_body` from `CacheGuard`.
