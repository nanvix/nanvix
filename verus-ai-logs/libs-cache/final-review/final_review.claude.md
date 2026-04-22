# Final Review — claude-opus-4.6

## 1. Spec Quality

### External-Top Specs (API Contracts)

**Cache::new** (lib.rs:160–164)
- `ensures result@ == CacheView::spec_new(capacity as nat), result@.inv()`
- ✅ Correct, complete, readable. Spec transition `spec_new` produces empty contents, given capacity, empty LRU order. The `inv()` postcondition is explicit.

**Cache::get** (lib.rs:191–207)
- Two-branch ensures: hit returns `Some` with correct value, refreshes LRU via `spec_get`, preserves `inv()`; miss returns `None`, state unchanged.
- ✅ Correct and complete. The guard's abstract value (`result->Some_0@`) is constrained to match `spec_get(*key).1.unwrap()`.
- ✅ Frame condition on miss: `self@ == old(self)@` — full frame.
- ✅ Frame condition on hit: state transitions via `spec_get` which only modifies `lru_order`.
- Minor note: the spec does not explicitly state `self@.contents == old(self)@.contents` on hit — this is *implied* by `spec_get` returning `..self` for contents. Acceptable since `spec_get` is `pub open`.

**Cache::put** (lib.rs:231–237)
- `ensures self@ == old(self)@.spec_put(key, value), self@.inv()`
- ✅ Correct and clean unified contract. All four branches (zero-cap no-op, overwrite, evict+insert, insert) are captured by `spec_put`.
- ✅ `inv()` postcondition explicit.

**Cache::remove** (lib.rs:276–281)
- `ensures self@ == old(self)@.spec_remove(*key), self@.inv()`
- ✅ Correct. `spec_remove` handles both present (remove + filter LRU) and absent (no-op) branches.

**Cache::clear** (lib.rs:298–304)
- `ensures self@ == old(self)@.spec_clear(), self@.inv()`
- ✅ Correct. `spec_clear` resets contents and lru_order, preserves capacity.

**CacheGuard::deref** (lib.rs:94–96)
- `ensures *ret == self@`
- ✅ Correct. Relates dereferenced value to guard's abstract view.

**CacheGuard::deref_mut** — No spec. Excluded from verification due to Verus `&mut` return type limitation.
- ⚠️ Known gap, documented in trust.md.

**Cache::evict** (lib.rs:319–331) — Private, but has a spec:
- Postcondition specifies victim is `lru_order[0]`, contents/order updated, capacity preserved, inv maintained.
- ✅ Well-structured internal contract.

### Anti-pattern Check
- No tautological ensures found.
- No subsumed properties (each ensures clause adds information).
- Error paths: `get` miss is covered. `remove` absent key is covered. `put` zero-capacity is covered.
- Frame conditions: present on all methods via spec transition functions.

**Verdict: GOOD** — specs are correct, complete, readable, and free of anti-patterns.

---

## 2. Caller Coverage

| Caller Expectation | Spec Clause | Covered? |
|---|---|---|
| **new: empty cache** | `spec_new(capacity as nat)` → contents = empty, lru_order = empty | ✅ |
| **new: capacity set** | `spec_new` sets `capacity` field | ✅ |
| **get: hit returns Some with value** | hit ⟹ `result is Some`, `result->Some_0@ == spec_get.1.unwrap()` | ✅ |
| **get: miss returns None** | `!contains(*key)` ⟹ `result is None` | ✅ |
| **get: hit refreshes LRU** | `self@ == spec_get(*key).0` (which uses `move_to_mru`) | ✅ |
| **get: size unchanged** | Implied by `spec_get` not modifying `contents` | ✅ (implicit) |
| **put: new key below cap inserts** | `spec_put` below-capacity branch inserts `(key, value)` | ✅ |
| **put: at cap evicts LRU** | `spec_put` at-capacity branch evicts `lru_order[0]`, inserts new | ✅ |
| **put: overwrite replaces value** | `spec_put` existing-key branch: `contents.insert(key, value)` | ✅ |
| **put: zero-cap no-op** | `spec_put` capacity==0 branch returns `self` | ✅ |
| **remove: present key removed** | `spec_remove` present branch: `contents.remove(key)` | ✅ |
| **remove: absent key no-op** | `spec_remove` absent branch: returns `self` | ✅ |
| **clear: all removed** | `spec_clear` sets `contents = Map::empty()` | ✅ |
| **clear: capacity preserved** | `spec_clear` uses `..self` (capacity field preserved) | ✅ |
| **deref: returns reference to value** | `*ret == self@` | ✅ |
| **deref_mut: returns mutable reference** | No spec (Verus limitation) | ❌ |

- **Covered: 15 / 16**
- The only uncovered expectation is `deref_mut`, which cannot be specified due to Verus's `&mut` return type limitation. This is documented and justified.

---

## 3. Proof Completeness

- **admit() count: 0**
- Searched `lib.spec.rs`, `lib.proof.rs`, `lib.vstd_btree.rs`, `lib.rs` — no `admit()` calls found.
- All five invariant preservation lemmas (`spec_new`, `spec_get`, `spec_put`, `spec_remove`, `spec_clear`) are fully proven.
- The `lemma_new_view`, `lemma_clear_view`, and `lemma_remove_view` integration lemmas are also fully proven.

**Verdict: PASS** — no admit() blockers.

---

## 4. Trust Minimization

### External Type Specifications

| Item | File:Line | Challenge | Verdict |
|---|---|---|---|
| ExBTreeMap | lib.vstd_btree.rs:31–38 | Required: BTreeMap is a stdlib type not modeled by vstd on no_std. `external_body` needed for private fields. | ✅ Acceptable |
| ExGlobal | lib.vstd_btree.rs:40–41 | Required: default allocator type parameter for BTreeMap. | ✅ Acceptable |
| ExCacheEntry | lib.spec.rs:17–18 | Private struct used as BTreeMap value type. `external_type_specification` without `external_body` — Verus sees the fields. | ✅ Acceptable |
| ExCacheGuard | lib.spec.rs:23–25 | Contains `&'a mut V` field. Verus cannot handle `&mut` in struct fields. `external_body` required. | ✅ Acceptable (Verus limitation) |

### External Body Functions

| Function | File:Line | Can it be eliminated? | Verdict |
|---|---|---|---|
| btreemap_remove | lib.rs:114–123 | No. `BTreeMap::remove` has `Borrow<Q>` parameter that cannot be monomorphized in `assume_specification` for `alloc::collections::BTreeMap`. Thin wrapper fixing Q=K. Body is a single stdlib call. | ✅ Justified |
| CacheGuard::deref | lib.rs:93–99 | No. CacheGuard is `external_body` — the body accesses the opaque `&mut V` field. | ✅ Justified |
| Cache::get | lib.rs:190–218 | **Challenging**: Uses `BTreeMap::get_mut()` which has no vstd spec and returns `Option<&mut V>` (Verus limitation). Also constructs CacheGuard with `&mut`. Cannot be rewritten without changing exec code (source integrity). | ⚠️ Irreducible — Verus limitation |
| Cache::put | lib.rs:230–265 | **Challenging**: Same `get_mut` blockers as `get`. Rewriting to `remove+insert` would change exec code. `contains_key` also has the `Borrow<Q>` blocker. | ⚠️ Irreducible — Verus limitation |
| Cache::evict | lib.rs:318–344 | No. Uses `iter().min_by_key()` iterator chain — iterator combinators have no vstd specs. | ⚠️ Irreducible — Verus limitation |
| axiom_cache_lru_of_remove | lib.proof.rs:401–411 | **Most concerning trust item.** This is an axiom (proof fn with external_body) asserting that removing a key from the BTreeMap produces the old LRU ordering filtered by that key. Sound because `BTreeMap::remove` doesn't change `last_used` counters of remaining entries. However, it is *asserted without proof* — the soundness argument is informal. | ⚠️ Necessary trust assumption — cannot be proven without fully axiomatizing counter-based LRU ordering |

### assume_specification Items (lib.vstd_btree.rs)
- 5 items: `BTreeMap::new`, `len`, `is_empty`, `insert`, `clear`
- All are copies of upstream vstd specs adapted for `alloc::collections::BTreeMap` (vs `std::collections::BTreeMap`).
- Semantically identical to upstream — only the import path differs.
- **Verdict: ✅ Acceptable** — standard practice for no_std targets.

### Broadcast Axioms (lib.vstd_btree.rs)
- `axiom_btree_map_view_finite_dom`: BTreeMap view domain is finite.
- `axiom_spec_btree_map_len`: Connects `spec_btree_map_len` to `btreemap_view_spec.len()`.
- Both mirror upstream vstd axioms.
- **Verdict: ✅ Acceptable.**

### Counter Overflow Assumption
- `self.counter` (u64) incremented without overflow check.
- At 10 billion ops/sec, overflow requires ~58 years.
- No `requires self.counter < u64::MAX` precondition.
- The spec uses abstract `Seq` ordering, so the *spec* is correct regardless — but the external_body trust gap means the *implementation's* correctness depends on no overflow.
- **Verdict: ✅ Acceptable** — documented in trust.md and bugs.md.

**Overall Trust Assessment:** 6 external_body functions, 1 custom axiom, 5 assume_specifications. All are justified by Verus limitations (no `&mut` return types, no iterator specs, no `get_mut` spec, `Borrow<Q>` generics) or no_std platform constraints. No items can be eliminated with current Verus capabilities.

---

## 5. AST Consistency

**Result: 2 MISMATCHES + 1 EXTRA — all acceptable**

### Cache::new — MISMATCH
```diff
-        Self {
+        let result = Self {
             entries: BTreeMap::new(),
             counter: 0,
             capacity,
+        };
+        proof! {
+            Self::lemma_new_view(&result, capacity);
         }
+        result
```
**Analysis:** The original directly returns the struct literal. The Verus version binds it to `result` and calls a `proof!{}` block. The `proof!{}` block is ghost code (erased at compilation). The `let result = ...; result` transformation is semantically equivalent to directly returning the expression — the struct construction and its field values are identical. **Acceptable.**

### Cache::remove — MISMATCH
```diff
-        self.entries.remove(key);
+        btreemap_remove(&mut self.entries, key);
+        proof! {
+            Self::lemma_remove_view(self, *key, old(self).entries, old(self).capacity);
+        }
```
**Analysis:** The original calls `self.entries.remove(key)` directly. The Verus version uses the `btreemap_remove` wrapper (which calls `m.remove(k)` inside — identical semantics) plus a `proof!{}` block. The VERUS REWRITE comment at line 284 documents this. **Acceptable** — same runtime behavior, wrapper needed for verification.

### btreemap_remove — EXTRA_IN_VERUS
This is a new helper function in the Verus version that doesn't exist in the original source. It wraps `BTreeMap::remove` to provide a verifiable signature. **Acceptable** — it's a pure wrapper that adds verification capability without changing semantics.

**Verdict: PASS** — all mismatches are semantically equivalent or documented rewrites.

---

## 6. Verification

**Result: PASS**

```
verification results:: 18 verified, 0 errors
```

- 18 verification conditions verified successfully.
- 0 errors.
- Exit code: 0.
- 8/9 exec functions have contracts (only `deref_mut` excluded — Verus limitation).

---

## 7. Guardrails Compliance

### Counts

| Dimension | Count | Details |
|---|---|---|
| `admit()` | **0** | None found in any file. |
| `assume()` | **0** | None found in any file (excluding `assume_specification`). |
| `external_body` | **8** | See breakdown below. |
| `trusted` | **0** | None found in any file. |
| `no_decreases` | **0** | None found in any file. |
| `cfg(not(verus_keep_ghost))` | **0** | None found — no cfg-gated exec code. |
| `assume_specification` | **5** | All in lib.vstd_btree.rs (stdlib specs). |

### external_body Breakdown

| # | Function | File:Line | Classification |
|---|---|---|---|
| 1 | ExBTreeMap (type) | lib.vstd_btree.rs:32 | EXTERNAL_TYPE — stdlib type |
| 2 | ExCacheGuard (type) | lib.spec.rs:24 | EXTERNAL_TYPE — Verus &mut limitation |
| 3 | CacheGuard::deref | lib.rs:93 | VERUS_LIMITATION — opaque guard field |
| 4 | btreemap_remove | lib.rs:114 | STDLIB_WRAPPER — Borrow\<Q\> workaround |
| 5 | Cache::get | lib.rs:190 | VERUS_LIMITATION — get_mut has no spec, &mut return |
| 6 | Cache::put | lib.rs:230 | VERUS_LIMITATION — same as get |
| 7 | Cache::evict | lib.rs:318 | VERUS_LIMITATION — no iterator specs |
| 8 | axiom_cache_lru_of_remove | lib.proof.rs:401 | VERUS_LIMITATION — uninterpreted LRU ordering |

Items 1–2 are `external_type_specification` (acceptable for types with private/unsupported fields).
Items 3–7 are exec functions on user code — each is a concern requiring justification. All 5 are justified by documented Verus limitations.
Item 8 is a proof axiom — concerning but necessary for the uninterpreted LRU ordering model.

### BLOCKER Assessment
- **admit: 0** — ✅ No blockers.
- **assume: 0** — ✅ No blockers.
- **trusted: 0** — ✅ No blockers.
- **external_body on user functions: 5** (+ 1 axiom) — ⚠️ All justified but represent trust gaps.

---

## 8. Bug Reconciliation

### BUG-1: Counter Overflow (u64)

- **Status in bugs.md:** UNCONFIRMED
- **Still valid?** Yes — `self.counter += 1` in `get` (line 210) and `put` (lines 246, 257) has no overflow guard.
- **Was it fixed?** No — intentionally left as a documented trust assumption.
- **Classification:** **Context-Dependent** — physically unreachable (58+ years at 10B ops/sec). The spec is correct regardless (uses abstract `Seq` ordering), but the implementation's correctness under the `external_body` trust gap depends on no overflow.
- **Impact:** The overflow would only matter if `get`/`put` were not `external_body`. Since they are, the spec-level model (abstract LRU sequence) is immune. However, if Verus adds `get_mut` support and these functions become body-verified, the overflow would need explicit handling.

### Undiscovered Bugs
- No additional bugs discovered during this review.
- The `clear` function resets `self.counter = 0`, which is correct but means counter values are not globally unique across clear boundaries — this is fine since all entries are also cleared.

---

## Issues Found (highest priority first)

1. **[CONCERN]** `Cache::get`, `Cache::put`, and `Cache::evict` are `external_body` — three of the most important API functions have their bodies trusted rather than verified. This is due to Verus limitations (no `get_mut` spec, no `&mut` return types, no iterator specs). The specs are well-written and the trust boundary is clearly documented, but ~60% of the exec code by complexity is unverified.

2. **[CONCERN]** `axiom_cache_lru_of_remove` is a custom axiom with `external_body` on a proof function. It asserts a relationship between `cache_lru_of` (which is partly uninterpreted) and BTreeMap removal. The soundness argument is informal (BTreeMap::remove preserves other entries' counters). This is the most opaque trust item.

3. **[CONCERN]** `CacheGuard::deref_mut` has no specification at all — Verus cannot handle `&mut` return types. Mutation-through-guard semantics are entirely unmodeled. While Rust's borrow checker provides safety guarantees, there is no formal connection between guard mutations and cache state.

4. **[NOTE]** Counter overflow (BUG-1) remains as a documented trust assumption. Acceptable given the 58-year overflow horizon.

5. **[NOTE]** AST mismatches in `Cache::new` and `Cache::remove` are semantically equivalent rewrites documented with VERUS REWRITE comments. The `btreemap_remove` wrapper is an additional function not in the original source.

---

## Overall Assessment

**PASS (Conditional)**

**Rationale:**

The verification effort for the `cache` crate is well-executed within the constraints of current Verus capabilities:

- **Specs are high quality:** All 7 public functions (except `deref_mut`) have correct, complete, readable specifications with proper frame conditions and invariant preservation. The spec transition functions in `CacheView` are cleanly designed.
- **Proofs are complete:** 0 `admit()`, 0 `assume()`, 0 `trusted`. All 18 verification conditions pass. Five invariant preservation lemmas are fully proven with clean mathematical proofs.
- **Trust boundary is well-documented:** Every `external_body` and `assume_specification` is documented in trust.md with classification, justification, and Verus error reproducers.
- **Caller expectations are met:** 15/16 expectations from caller_analysis.md have corresponding spec clauses.

**Caveats preventing unconditional PASS:**

1. Three core API functions (`get`, `put`, `evict`) are `external_body` — their implementations are trusted, not verified. This is the dominant limitation.
2. One custom axiom (`axiom_cache_lru_of_remove`) is informally justified.
3. `deref_mut` is entirely unverified.

All caveats are due to documented Verus limitations (no `&mut` return types, no `get_mut` spec, no iterator specs) and cannot be resolved without upstream Verus changes. The verification achieves the maximum possible assurance level given current tool capabilities.
