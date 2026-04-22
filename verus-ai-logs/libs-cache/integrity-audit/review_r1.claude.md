# Independent Review: cache (Claude Opus 4.6)

**Reviewer:** Claude Opus 4.6 (independent audit)
**Date:** 2026-04-22
**Crate:** `src/libs/cache`
**Files reviewed:** `lib.rs`, `lib.spec.rs`, `lib.proof.rs`, `trust.md`, `fix_report.md`, `bugs.md`

---

## 1. Cheating Detection

### Direct grep results

| Pattern | Count | Details |
|---------|-------|---------|
| `admit()` | 0 | None found in any source file |
| `assume()` | 0 | None found |
| `external_body` | 9 | 7 on functions (`lib.rs`), 2 on type specs (`lib.spec.rs`) |
| `external_type_specification` | 4 | ExBTreeMap, ExGlobal, ExCacheEntry, ExCacheGuard |
| `trusted` | 0 | None found |
| `exec_allows_no_decreases` | 0 | None found |
| `cfg(not(verus_keep_ghost))` | 0 | No cfg-gated exec code |
| `assume_specification` | 0 | None found |

### external_body breakdown

**Type-level (2):**
1. `ExBTreeMap` — `lib.spec.rs:21` (paired with `external_type_specification` at line 20)
2. `ExCacheGuard` — `lib.spec.rs:37` (paired with `external_type_specification` at line 36)

**Function-level (7):**
1. `CacheGuard::deref` — `lib.rs:91`
2. `Cache::new` — `lib.rs:141`
3. `Cache::get` — `lib.rs:168`
4. `Cache::put` — `lib.rs:208`
5. `Cache::remove` — `lib.rs:254`
6. `Cache::clear` — `lib.rs:271`
7. `Cache::evict` — `lib.rs:289`

### Cross-check with fix_report.md

The fix_report claims: `external_body: 9 → 9` (0 eliminated). **Confirmed correct.**
All other counts (admit=0, assume=0, trusted=0, no_decreases=0, cfg_gate=0) match. **✓**

### Unverified function

`CacheGuard::deref_mut` (line 101) has no Verus annotations at all — it is excluded from
verification scope entirely. The `make verify-cache` output confirms `7/8 exec functions have
contracts` with `deref_mut` listed as unverified. **Correctly documented in trust.md.**

---

## 2. Trust Item Challenges

### 2a. ExBTreeMap — `EXTERNAL_TYPE`

**Classification correctness: ✓ CORRECT**

I verified:
- vstd provides BTreeMap specs at `/home/ruize/verus/vstd/std_specs/btree.rs`
- The module is gated by `#[cfg(all(feature = "alloc", feature = "std"))]` in
  `vstd/std_specs/mod.rs:17-18`
- The btree module imports `use std::collections::{BTreeMap, BTreeSet}` — hard dependency on `std`
- The cache crate targets `i686-unknown-none` (Nanvix kernel), built with
  `-Z build-std=core,alloc,compiler_builtins` — no `std` crate available
- `external_body` is required because BTreeMap has private internal fields

**Could we use `assume_specification` instead?** In principle, custom `assume_specification`
directives for `alloc::collections::BTreeMap` methods could be written, bypassing the vstd `std`
gating. However:
1. This only shifts trust from "function body is correct" to "BTreeMap spec is correct"
2. It does not eliminate trust — it redistributes it
3. Even with all BTreeMap method specs, `get` and `put` remain `external_body` due to `get_mut`
   (see §2c)

**Verdict: Legitimate. Cannot be eliminated.**

### 2b. ExCacheGuard — `VERUS_LIMITATION`

**Classification correctness: ✓ CORRECT**

`CacheGuard<'a, V>` contains field `value: &'a mut V`. Verus error message: "The verifier does
not yet support &mut types, except in special cases."

I confirmed:
- vstd btree specs do use `&mut` in function parameters (e.g., `insert(m: &mut BTreeMap...)`) —
  so `&mut` in function parameters IS supported
- But `&mut` in **struct fields** is a different, unsupported case
- `external_body` is required to hide the struct's internal fields from Verus
- No `get_mut` spec exists in vstd (0 occurrences confirmed by grep)

**Could CacheGuard be redesigned?** Only by removing `&mut V` from the struct, which would break
the `DerefMut` API contract. This is a fundamental design choice — callers mutate cached values
through `*guard = new_val`. Changing this would require API-level redesign beyond verification
scope.

**Verdict: Genuine Verus limitation. Cannot be eliminated without API redesign.**

### 2c. Function-level external_body — detailed challenge

#### Cache::new

**Root blocker:** No vstd btree specs on `no_std` target.

**Could it be verified with custom `assume_specification`?** Partially. `BTreeMap::new()` has a
straightforward spec (`m@ == Map::empty()`). But `Cache::view()` is `uninterp` — to verify the
body, we'd need a concrete view function mapping `(entries, counter, capacity)` →
`CacheView{contents, capacity, lru_order}`. Defining `lru_order` requires sorting BTreeMap entries
by `last_used` counter at the spec level — non-trivial.

**Verdict: Could theoretically be unblocked (with substantial effort), but trust just shifts.**

#### Cache::get

**Root blockers (multiple, independent):**
1. `BTreeMap::get_mut()` — **no vstd spec at all** (confirmed: 0 occurrences of `get_mut` in
   vstd/std_specs/btree.rs)
2. `get_mut` returns `Option<&mut V>` — Verus `&mut` return type limitation
3. Creates `CacheGuard` with `&mut entry.value` — depends on ExCacheGuard limitation

**Could the code use `get` (immutable) + `remove` + `insert` instead?** This avoids `get_mut` but:
- Still constructs `CacheGuard` with `&mut` (blocker #3 persists)
- Would be less efficient (O(log n) remove + O(log n) insert vs O(log n) get_mut)
- The `CacheGuard` must reference `&mut V` in the BTreeMap's storage, so removing the entry
  first would invalidate the reference

**Verdict: Cannot be eliminated. Three independent blockers.**

#### Cache::put

**Root blockers:** Same as `get` (uses `get_mut` for update-in-place) plus calls `self.evict()`.

**Could the code avoid `get_mut`?** Same analysis as `get`. The update-in-place path (`Some(entry)
= self.entries.get_mut(&key)`) could theoretically use `remove` + `insert`, but:
- `put` does not return a `CacheGuard`, so blocker #3 from `get` doesn't apply here
- However, blocker #1 (`get_mut` has no spec) and the `no_std` target issue remain
- Even rewriting to avoid `get_mut`, the `no_std` BTreeMap spec problem persists

**Verdict: Cannot be eliminated without `assume_specification` for BTreeMap AND code rewrite.**

#### Cache::remove

**Root blocker:** vstd btree specs unavailable on `no_std` target.

**Could be unblocked with custom `assume_specification`?** Yes, in principle. `BTreeMap::remove`
has a well-defined spec in vstd (`borrowed_key_removed`). Custom `assume_specification` for
`alloc::collections::BTreeMap::remove` would allow verifying the body. But it still requires a
concrete `Cache::view()`.

**Verdict: Could theoretically be unblocked. Trust shifts, not eliminated.**

#### Cache::clear

**Root blocker:** Same `no_std` issue.

**Could be unblocked?** Same analysis as `remove`. `BTreeMap::clear` has a trivial spec
(`m@ == Map::empty()`). Custom `assume_specification` + concrete view would work in principle.

**Verdict: Could theoretically be unblocked. Trust shifts, not eliminated.**

#### Cache::evict

**Root blockers (multiple):**
1. `self.entries.iter().min_by_key(|(_, e)| e.last_used)` — iterator combinator with closure.
   vstd has `iter` and `Iter::next` specs but no `min_by_key` spec.
2. `BTreeMap::remove()` — `no_std` btree spec issue.

**Could the iterator chain be rewritten as a for loop?** Yes, in principle:
```rust
let mut min_key = None;
let mut min_used = u64::MAX;
for (k, e) in self.entries.iter() {
    if e.last_used < min_used {
        min_used = e.last_used;
        min_key = Some(k.clone());
    }
}
```
vstd has `BTreeMap::iter` specs and `ForLoopGhostIterator` support. But:
- This still requires the `no_std` target to have btree specs (blocked)
- Plus the `remove` at the end still needs btree specs
- The loop invariant would be non-trivial (track minimum across iteration)

**Verdict: Could be partially unblocked with for-loop rewrite + `assume_specification`, but
still depends on `no_std` btree access.**

#### CacheGuard::deref

**Root blocker:** CacheGuard is `external_body` (due to `&mut V` field). The body `self.value`
accesses an opaque field.

**Verdict: Cannot be eliminated without resolving ExCacheGuard limitation.**

### 2d. CacheGuard::deref_mut — excluded from verification

**Correctly excluded.** The function signature `fn deref_mut(&mut self) -> &mut V` triggers
Verus error "The verifier does not yet support &mut types, except in special cases" — even
`external_body` cannot be applied. The function cannot carry any Verus annotations.

**Impact:** Mutation through `*guard = new_value` is completely unmodeled. This is a genuine
Verus limitation on `&mut` return types, correctly documented.

### 2e. Counter overflow assumption

**Correctly documented.** `self.counter: u64` is incremented in `get` and `put` without overflow
checking. The spec uses abstract `Seq` ordering, so the spec is correct regardless of overflow —
but the implementation depends on no overflow for LRU ordering to be correct.

At 10 billion ops/sec, overflow requires ~58 years. This is physically unreachable.
Adding `requires self.counter < u64::MAX` would burden callers with an unprovable obligation.
**Classification as trust assumption is appropriate.**

---

## 3. AST Consistency

### AST checker results

```
Functions: 18 matched, 0 mismatched, 0 missing
Structs: 3 matched, 0 mismatched
Consistent: true
```

All 18 functions (including 10 test functions) have identical AST hashes between the `dev` base
and the verified branch. All 3 structs (`Cache`, `CacheEntry`, `CacheGuard`) match exactly.

**No exec code was modified.** Only Verus annotations (`#[verus_verify]`, `#[verus_spec]`,
cfg-gated imports/includes, feature flags) were added. **✓ PASS**

---

## 4. Verification Status

```
make verify-cache
```

**Verus exit code: 0** (verification successful)

Make returns exit code 2 due to `CHEATING_DETECTED` (9 `external_body` items). This is expected —
the cheating detector correctly flags all `external_body` usage. The Verus verification itself
(all proofs, specs, lemmas) passes clean.

All 5 invariant preservation lemmas are fully proven:
- `lemma_spec_new_inv` ✅
- `lemma_spec_get_inv` ✅
- `lemma_spec_put_inv` ✅
- `lemma_spec_remove_inv` ✅
- `lemma_spec_clear_inv` ✅

Plus 5 helper lemmas (filter, subrange, push properties) — all proven without admit/assume.

---

## 5. Bug vs Limitation

### Per external_body analysis

| Function | Bug masked? | Analysis |
|----------|-------------|----------|
| `Cache::new` | No | Constructor is trivially correct. BTreeMap::new() + counter=0 + capacity=arg. |
| `Cache::get` | **Partial (BUG-1)** | Counter overflow (`self.counter += 1`) could corrupt LRU ordering if u64 wraps. Physically unreachable but formally unmodeled. Otherwise correct. |
| `CacheGuard::deref` | No | `self.value` returns the contained reference. Trivially correct. |
| `Cache::put` | **Partial (BUG-1)** | Same counter overflow issue. The eviction threshold `entries.len() >= capacity` is correct — at capacity, evict before insert. |
| `Cache::remove` | No | Single `BTreeMap::remove()` call. Correct by BTreeMap semantics. |
| `Cache::clear` | No | `BTreeMap::clear()` + counter reset. Correct. |
| `Cache::evict` | No | `min_by_key(|(_, e)| e.last_used)` correctly finds the entry with the smallest counter (LRU victim). This matches `lru_order[0]` in the spec. Correct assuming no counter overflow. |

### BUG-1: Counter overflow (u64)

**Status:** UNCONFIRMED — physically unreachable

**Analysis:** `self.counter += 1` in `get` (line 188) and `put` (lines 224, 235) can overflow
after 2^64 operations. In Rust debug mode this panics; in release mode it wraps to 0. Wrapping
would corrupt LRU ordering — a fresh entry could have counter 0, appearing as the oldest entry
and being evicted immediately.

**Severity:** LOW. 2^64 at 10B ops/sec = ~58.5 years continuous operation.

**Spec impact:** The spec uses abstract `Seq<K>` ordering (not counters), so the spec transitions
are correct by construction. The external_body gap means the implementation's correctness depends
on the assumption that `counter < u64::MAX` holds, which is documented but not formally enforced.

### Eviction correctness

I independently verified that `evict`'s `min_by_key(|(_, e)| e.last_used)` produces the correct
LRU victim:

- `lru_order` is defined as a `Seq<K>` ordered LRU→MRU (index 0 = LRU)
- Entries with lower `last_used` counter = less recently used = closer to index 0
- `min_by_key` on `last_used` finds the minimum counter = the LRU entry = `lru_order[0]` ✓

### Put eviction threshold

The code checks `self.entries.len() >= self.capacity` before evicting. The spec checks
`self.contents.dom().len() >= self.capacity`. Both correctly trigger eviction when the cache is
at capacity (len == capacity), ensuring post-insert the cache has exactly capacity entries. ✓

---

## 6. vstd Search Results

### BTreeMap specs exist in vstd

File: `/home/ruize/verus/vstd/std_specs/btree.rs`

**assume_specification entries found (BTreeMap methods):**
- `BTreeMap::new` (line 613)
- `BTreeMap::insert` (line 624)
- `BTreeMap::get` (line 718) — immutable only
- `BTreeMap::remove` (line 776)
- `BTreeMap::clear` (line 793)
- `BTreeMap::len` (line 590)
- `BTreeMap::is_empty` (line 597)
- `BTreeMap::contains_key` (line 670)
- `BTreeMap::keys` (line 800)
- `BTreeMap::values` (line 813)
- `BTreeMap::iter` (line 411)
- `btree_map::Iter::next` (line 289)

**NOT specified:**
- `BTreeMap::get_mut` — **0 occurrences in entire vstd** (confirmed)
- `min_by_key`, `map` and other iterator combinators — not specified

### cfg gating confirms no_std incompatibility

```rust
// vstd/std_specs/mod.rs:17-18
#[cfg(all(feature = "alloc", feature = "std"))]
pub mod btree;
```

The btree module uses `use std::collections::{BTreeMap, BTreeSet}` (imports from `std`). The
cache crate's target (`i686-unknown-none`, OS: nanvix) builds with
`-Z build-std=core,alloc,compiler_builtins` — the `std` crate does not exist. Enabling the `std`
feature would cause compilation failure in vstd's btree module.

**Conclusion:** The trust.md's `no_std` incompatibility claim is **factually correct** and
well-documented. The fix_report correctly notes that vstd *does* have BTreeMap specs but they are
structurally inaccessible.

---

## 7. Challengeable Items (should have been eliminated)

### Potentially eliminable with significant effort

Three functions (`new`, `remove`, `clear`) could theoretically be verified with:
1. Custom `assume_specification` for `alloc::collections::BTreeMap` methods
2. A concrete `Cache::view()` implementation (requiring spec-level sort of entries by counter)

**However**, the fix_report correctly evaluated and rejected this approach because:
- It eliminates only 3 of 7 function-level `external_body` items
- The 4 most important methods (`get`, `put`, `evict`, `deref`) remain `external_body` regardless
- Custom `assume_specification` introduces new trust assumptions (BTreeMap method specs), so
  trust is redistributed, not eliminated
- The engineering cost (concrete View, spec-level sort) is disproportionate to the gain

**My assessment: The fix_report's rejection is reasonable.** Eliminating 3/7 external_body items
while the remaining 4 (covering the most complex logic) stay trusted does not meaningfully
improve the trust boundary. The current approach of keeping all 7 as external_body with clear
documentation is pragmatically sound.

### Items that genuinely cannot be eliminated

| Item | Root blocker | Eliminable? |
|------|-------------|-------------|
| ExBTreeMap | no_std target, BTreeMap private fields | No |
| ExCacheGuard | `&mut V` in struct field | No (Verus limitation) |
| `deref` | ExCacheGuard is opaque | No |
| `get` | `get_mut` no spec + `&mut` return + CacheGuard | No (3 independent blockers) |
| `put` | `get_mut` no spec + calls evict | No (with current code) |
| `evict` | `min_by_key` combinator + no_std btree | No |
| `deref_mut` | `&mut` return type | No (Verus limitation) |
| `new` | no_std btree + uninterp view | Theoretically yes |
| `remove` | no_std btree + uninterp view | Theoretically yes |
| `clear` | no_std btree + uninterp view | Theoretically yes |

---

## 8. Verdict

### **PASS**

**Justification:**

1. **Cheating counts are accurate.** 9 `external_body`, 0 everything else. Fix_report matches. ✓
2. **All trust items are correctly classified.** ExBTreeMap as `EXTERNAL_TYPE`, ExCacheGuard as
   `VERUS_LIMITATION`, all function-level as `VERUS_LIMITATION`. Each classification is supported
   by verified evidence (vstd cfg gating, `get_mut` absence, `&mut` struct field limitation). ✓
3. **AST consistency is perfect.** 18/18 functions, 3/3 structs match. No exec code modified. ✓
4. **Verification passes.** Verus exit code 0. All 5 invariant preservation lemmas + 5 helper
   lemmas proven clean (no admit/assume). ✓
5. **Bug documentation is appropriate.** BUG-1 (counter overflow) is correctly identified as
   unconfirmed/low-severity and documented as a trust assumption. ✓
6. **No eliminable items were left unaddressed.** The 3 theoretically eliminable functions
   (`new`, `remove`, `clear`) were evaluated and rejected for sound engineering reasons. ✓
7. **Spec quality is good.** The `CacheView` abstraction correctly models LRU semantics with
   `contents`, `capacity`, and `lru_order`. Spec transitions cover all edge cases (zero capacity,
   key present/absent, at/below capacity). ✓
8. **Proof quality is high.** 10 proof lemmas, all fully proven. Helper lemmas for filter, push,
   subrange properties are well-structured and independently useful. ✓

**Minor observation:** The trust boundary documentation (trust.md, fix_report.md) is unusually
thorough, including evaluated-and-rejected alternatives with clear reasoning. This exceeds the
typical standard.
