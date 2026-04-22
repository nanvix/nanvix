# Independent Review: cache (Claude Opus 4.6)

## 1. Spec Preservation

Comparing current specs against spec-phase commit `a48beb884`. For each removed
ensures clause, I assess whether it is **implied** by the remaining ensures or
represents a genuine **weakening**.

### Cache::new

Removed clauses:
- `result@.contents == Map::<K, V>::empty()` — **Implied.** `spec_new(capacity as nat)` is defined as `CacheView { contents: Map::empty(), ... }`, so `result@ == spec_new(...)` already gives `result@.contents == Map::empty()`.
- `result@.capacity == capacity as nat` — **Implied.** Same reasoning: `spec_new` sets `capacity: capacity`, so the equality implies `result@.capacity == capacity as nat`.
- `result@.lru_order == Seq::<K>::empty()` — **Implied.** `spec_new` sets `lru_order: Seq::empty()`.

Also removed: `external_body` attribute. This is a **strengthening** — the body is now verified, increasing assurance.

**Verdict: No weakening. All removed clauses are logically implied by `result@ == CacheView::spec_new(capacity as nat)`.**

### Cache::get

Removed clauses:
- `self@.contents == old(self)@.contents` — **Implied.** `spec_get` returns `(CacheView { lru_order: ..., ..self }, ...)` on hit (using `..self` which preserves `contents`) and `(self, None)` on miss. So `self@ == old(self)@.spec_get(*key).0` implies `self@.contents == old(self)@.contents` in both cases.
- `self@.capacity == old(self)@.capacity` — **Implied.** Same reasoning: `spec_get` preserves `capacity` via `..self`.

Added clause:
- `result->Some_0@ == old(self)@.spec_get(*key).1.unwrap()` — This is a **new strengthening** that was missing in the spec phase. It gives the caller information about the returned guard's value.

**Verdict: No weakening. One strengthening added (guard value spec).**

### Cache::put

Removed clauses:
- `self@.capacity == old(self)@.capacity` — **Implied.** In all four branches of `spec_put`, capacity is preserved via `..self` or the zero-capacity self-return. So `self@ == old(self)@.spec_put(key, value)` implies capacity preservation.
- `old(self)@.capacity > 0 ==> { self@.contents.dom().contains(key) && self@.contents[key] == value }` (put-get round-trip) — **Implied.** In all three non-zero-capacity branches of `spec_put`: (a) overwrite existing: `contents.insert(key, value)` means `contents[key] == value`; (b) evict+insert: `contents.remove(victim).insert(key, value)` means `contents[key] == value`; (c) below-capacity insert: `contents.insert(key, value)` means `contents[key] == value`. Since `self@ == spec_put(...)`, the caller can derive this from the definition.
- `old(self)@.capacity == 0 ==> self@ == old(self)@` (zero-capacity no-op) — **Implied.** `spec_put` returns `self` when `capacity == 0`, so `self@ == spec_put(...)` trivially implies `self@ == old(self)@` when capacity is zero.

**Verdict: No weakening. All removed clauses are implied by `self@ == old(self)@.spec_put(key, value)` plus `spec_put`'s open definition.**

### Cache::remove

Removed clauses:
- `self@.capacity == old(self)@.capacity` — **Implied.** `spec_remove` preserves capacity in both branches (via `..self` or identity).
- `!self@.contents.dom().contains(*key)` — **Implied.** On the key-present branch, `spec_remove` does `contents.remove(key)`, so `!contents.remove(key).dom().contains(key)` follows from Map axioms. On the key-absent branch, the key was already absent.
- `!old(self)@.contents.dom().contains(*key) ==> self@ == old(self)@` — **Implied.** `spec_remove` returns `self` when the key is absent.

Also removed: `external_body` attribute — **strengthening** (body now verified).

**Verdict: No weakening. All removed clauses are implied by `self@ == old(self)@.spec_remove(*key)`.**

### Cache::clear

Removed clauses:
- `self@.contents == Map::<K, V>::empty()` — **Implied.** `spec_clear` sets `contents: Map::empty()`.
- `self@.lru_order == Seq::<K>::empty()` — **Implied.** `spec_clear` sets `lru_order: Seq::empty()`.
- `self@.capacity == old(self)@.capacity` — **Implied.** `spec_clear` uses `..self` preserving capacity.

Also removed: `external_body` attribute — **strengthening**.

**Verdict: No weakening. All removed clauses are implied by `self@ == old(self)@.spec_clear()`.**

### Summary

All 14 removed ensures clauses are logically implied by the retained `self@ == old(self)@.spec_<op>(...)` ensures combined with the `pub open` spec transition function definitions. The proving phase simplified the specs to their canonical form without losing caller-visible information. Three functions (`new`, `remove`, `clear`) had `external_body` removed — a net strengthening.

## 2. Cheating Audit

### admit() — **0 instances** ✅

### assume — **0 instances** ✅

### cfg-gated exec code — **0 instances** ✅
All three `cfg(verus_keep_ghost)` guards are on `include!()` statements (importing spec/proof/vstd_btree files). No exec code is gated.

### external_body on exec functions — **5 instances**

| # | Function | File:Line | Classification | trust.md | Eliminable? |
|---|----------|-----------|---------------|----------|-------------|
| 1 | `CacheGuard::deref` | lib.rs:93 | VERUS_LIMITATION | ✅ | No — CacheGuard is external_body (Verus &mut limitation) |
| 2 | `btreemap_remove` | lib.rs:114 | STDLIB_WRAPPER | ✅ | No — BTreeMap::remove has Borrow\<Q\> generic; cannot use assume_specification directly |
| 3 | `Cache::get` | lib.rs:190 | VERUS_LIMITATION | ✅ | No — BTreeMap::get_mut has no vstd spec, returns Option\<&mut V\> |
| 4 | `Cache::put` | lib.rs:230 | VERUS_LIMITATION | ✅ | No — Same get_mut blocker as Cache::get |
| 5 | `Cache::evict` | lib.rs:321 | VERUS_LIMITATION | ✅ | No — Uses iter().min_by_key() chain with no vstd specs |

All 5 have trust.md entries with classification and reproducer/justification.

### external_body on proof functions — **1 instance**

| # | Function | File:Line | Classification | trust.md |
|---|----------|-----------|---------------|----------|
| 6 | `axiom_cache_lru_of_remove` | lib.proof.rs:408 | VERUS_LIMITATION | ✅ |

This is an axiom linking the uninterpreted `cache_lru_of` function to BTreeMap removal.
The axiom states: `cache_lru_of(new) == cache_lru_of(old).filter(|k| k != key)`.

**Soundness assessment:** The axiom is sound under the invariant that BTreeMap::remove
does not alter `last_used` counters of remaining entries (which is true — BTreeMap::remove
only removes the target key). The `cache_lru_of` function sorts entries by `last_used`,
and removing one entry from a sorted sequence yields the original sequence filtered to
exclude that entry. This is a reasonable axiom given that the LRU ordering cannot be
expressed as a closed-form spec function over the opaque BTreeMap type.

### external_body on type specifications — **2 instances**

| # | Type | File:Line | Classification | trust.md |
|---|------|-----------|---------------|----------|
| 7 | `ExBTreeMap` | lib.vstd_btree.rs:32 | EXTERNAL_TYPE | ✅ |
| 8 | `ExCacheGuard` | lib.spec.rs:24 | VERUS_LIMITATION | ✅ |

### assume_specification — **5 instances** (all in lib.vstd_btree.rs)

| # | Method | Line | Matches vstd? |
|---|--------|------|---------------|
| 1 | `BTreeMap::new` | 69 | ✅ Semantically identical to vstd std_specs/btree.rs |
| 2 | `BTreeMap::len` | 88 | ✅ Semantically identical |
| 3 | `BTreeMap::is_empty` | 98 | ✅ Semantically identical |
| 4 | `BTreeMap::insert` | 108 | ✅ Semantically identical |
| 5 | `BTreeMap::clear` | 130 | ✅ Semantically identical |

These are necessary because vstd's BTreeMap specs are gated behind `cfg(std)`, unavailable
on the no_std kernel target. The specs are documented as being copied from vstd with only
the import path changed (`alloc::collections` vs `std::collections`).

### broadcast axiom — **2 instances** (lib.vstd_btree.rs)

| # | Axiom | Line | Assessment |
|---|-------|------|------------|
| 1 | `axiom_btree_map_view_finite_dom` | 56 | ✅ Mirrors vstd axiom |
| 2 | `axiom_spec_btree_map_len` | 80 | ✅ Mirrors vstd axiom |

### Unverified exec functions — **1 instance**

| # | Function | Reason |
|---|----------|--------|
| 1 | `deref_mut` | Verus limitation: cannot even annotate functions returning `&mut V` |

Documented in trust.md with reproducer.

### Trust assumption (implicit)

Counter overflow (`u64`): `self.counter` is incremented without overflow checking in `get`
and `put`. Documented in trust.md and bugs.md (BUG-1). Physically unreachable at ~58 years
at 10B ops/sec.

### Total trust items: 8 external_body + 5 assume_specification + 2 broadcast axioms + 1 unverified function = 16

All documented in trust.md. No undocumented trust items found.

## 3. Proof Quality

### Structure

The proofs in lib.proof.rs are well-structured mathematical reasoning, not brute-force:

1. **Helper lemmas (lines 16–168):** Seven reusable sequence lemmas:
   - `lemma_push_preserves_no_dup` — push + no_dup preservation
   - `lemma_filter_preserves_no_dup` — filter + no_dup (inductive, with decreases)
   - `lemma_filter_neq_to_set` — filter-then-to_set equals to_set-then-remove
   - `lemma_filter_neq_len` — filter of present key reduces len by 1
   - `lemma_filter_neq_absent` — filter is identity for absent key (inductive)
   - `lemma_subrange_no_dup` — subrange preserves no_dup
   - `lemma_drop_first_to_set` — dropping first element = remove from set

   These are well-factored, with clear pre/postconditions and appropriate decreases clauses.

2. **Invariant preservation lemmas (lines 175–332):** Five lemmas proving `inv()` is preserved
   by each spec transition. Each follows the same pattern:
   - Case-split on the spec transition branches
   - Prove `no_duplicates`, `to_set == contents.dom()`, and `len` invariants
   - Use helper lemmas rather than asserting low-level facts

3. **Verification bridge lemmas (lines 338–511):** Three lemmas connecting concrete Cache
   state to abstract CacheView, used in the bodies of `new`, `clear`, and `remove`.

### Assessment

The proof technique is sound and modular:
- No `admit()` or `assume` anywhere
- Inductive proofs use proper `decreases` annotations
- Helper lemmas are genuinely reusable (e.g., `lemma_filter_preserves_no_dup` is used in
  `spec_get_inv`, `spec_put_inv`, `spec_remove_inv`, and `lemma_remove_view`)
- The `reveal()` pattern for closed spec functions is used correctly

**Quality: HIGH.** The proofs follow a principled decomposition.

## 4. Rewrite Audit

### Location: Cache::remove, line 284 of lib.rs

**Original code (spec phase):**
```rust
self.entries.remove(key);
```

**Rewritten code (proving phase):**
```rust
btreemap_remove(&mut self.entries, key);
```

### Assessment

- **Minimal?** Yes — single line changed.
- **Semantically equivalent?** Yes — `btreemap_remove(m, k)` calls `m.remove(k)`, identical behavior.
- **Could it have been avoided?** No. `BTreeMap::remove(&mut self, key: &Q)` where `Q: Borrow<K>` has a complex generic signature that prevents writing an `assume_specification` for the `alloc::collections` variant (the `Borrow<Q>` + `Allocator` type parameters cannot be monomorphized). The wrapper fixes `Q = K` and provides specs via `external_body`. This is the standard escalation approach (stdlib wrapper) documented in trust.md.
- **Return value:** The original code discarded the return value (`remove` returns `Option<V>`). The wrapper's spec is richer (provides ensures about the return value), but the return value is still discarded in `Cache::remove`. This is fine — the spec on the wrapper is complete, and the caller simply doesn't use the result.

**Verdict: Rewrite is justified, minimal, and semantically equivalent.**

## 5. Verification Status

```
$ touch src/libs/cache/src/lib.rs && make verify-cache
verification results:: 18 verified, 0 errors
Exit code : 0

Cheating Pattern Check:
  external_body: 8 (6 exec/proof functions + 2 type specs, all documented)
  assume=0, admit=0, trusted=0, no_decreases=0, cfg_gate=0

Function Coverage:
  8/9 exec functions have contracts
  Unverified: deref_mut (Verus &mut limitation)
```

**Verification passes cleanly: 18 verified, 0 errors.**

The `make` target exits with error code 1 due to `CHEATING_DETECTED` status (triggered by
external_body count), but this is expected and documented — all external_body uses are
justified Verus limitations or stdlib wrappers.

## Issues (highest priority first)

### Issue 1: Counter overflow is unmodeled (LOW)

**Location:** `Cache::get` (line 210), `Cache::put` (line 246)

`self.counter += 1` can overflow `u64`. The spec transition functions use abstract `Seq`
ordering, so the specs are correct regardless. However, the `external_body` functions
(`get`, `put`) rely on `counter` for correct LRU behavior. If `counter` wraps, the
implementation's LRU ordering would be corrupted while the spec claims otherwise.

This is documented in trust.md and bugs.md (BUG-1). Severity is LOW (58+ years at
10B ops/sec), but it represents a gap between the trusted implementation and the spec.

**Recommendation:** Consider `debug_assert!(self.counter < u64::MAX)` as a runtime
safety net, or document the assumption more prominently in the public API docs.

### Issue 2: deref_mut is unverified (LOW)

**Location:** `CacheGuard::deref_mut` (line 102)

Mutation through the guard (`*guard = new_value`) has no formal spec. This is a known
Verus limitation with no workaround. Documented in trust.md.

**Impact:** Callers who mutate cached values through the guard have no formal guarantee
the mutation persists. In practice, `CacheGuard` wraps `&mut V`, so Rust's ownership
guarantees the mutation is reflected in the BTreeMap entry.

### Issue 3: No issues with spec drift or undocumented trust (NONE)

All trust items are documented. No spec weakening occurred. No hidden `admit()` or
`assume` found.

## Verdict: PASS

**Justification:**

1. **Spec preservation:** All 14 removed ensures clauses are logically implied by the
   retained canonical `self@ == spec_<op>(...)` form. Three functions had `external_body`
   removed (net strengthening). One new strengthening added (`get` guard value spec).
2. **Cheating:** Zero `admit()`, zero `assume`, zero cfg-gated exec code. All 8
   `external_body` uses are justified by Verus limitations or stdlib wrapper needs,
   with complete trust.md documentation including reproducers.
3. **Proofs:** 18 verified, 0 errors. Proof structure is modular with well-factored
   helper lemmas. The single `external_body` proof axiom (`axiom_cache_lru_of_remove`)
   is sound and documented.
4. **Rewrite:** Minimal, semantically equivalent, justified by BTreeMap generic signature
   limitations.
5. **Coverage:** 8/9 exec functions have contracts (the missing one is a known Verus
   limitation).
