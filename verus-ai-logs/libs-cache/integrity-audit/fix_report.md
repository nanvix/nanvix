# Integrity Audit Report: cache

## Cheating Counts (before → after)

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 8 | 8 | 0 |
| trusted | 0 | 0 | 0 |
| no_decreases | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |
| assume_specification | 5 | 5 | 0 |

Note: The 8 `external_body` items break down as: 6 functions (`axiom_cache_lru_of_remove`,
`deref`, `btreemap_remove`, `get`, `put`, `evict`) + 2 external type specifications
(`ExBTreeMap`, `ExCacheGuard`). The 5 `assume_specification` items are in `lib.vstd_btree.rs`
for `BTreeMap::{new, len, is_empty, insert, clear}` — copies of upstream vstd specs adapted
for `alloc::collections::BTreeMap` on no\_std targets. Additionally, 2 `broadcast axiom`
declarations exist for `btreemap_view_spec` domain finiteness and `len` equivalence.

## Items Eliminated

None. Every `external_body` item was challenged against the verus-constraints escalation
ladder. No items could be eliminated.

## Detailed Challenge Analysis

### 1. btreemap_remove (lib.rs:114-123) — KEEP

**Classification:** STDLIB_WRAPPER

**Challenge:** Can we use `assume_specification` for `BTreeMap::remove` directly instead
of a wrapper?

**Result:** No. `BTreeMap::remove` has signature `fn remove<Q>(&mut self, key: &Q) -> Option<V>`
where `K: Borrow<Q>` and `Q: Ord`. The `Borrow<Q>` generic parameter cannot be monomorphized
in `assume_specification` for `alloc::collections::BTreeMap` (documented in
`lib.vstd_btree.rs:124-127`). The wrapper fixes `Q=K` and is a single stdlib call — the
thinnest possible trust layer.

### 2. CacheGuard::deref (lib.rs:93-99) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can we verify the body?

**Result:** No. `CacheGuard` itself is `external_body` because it contains `&'a mut V` in a
struct field, which Verus does not support ("The verifier does not yet support &mut types,
except in special cases"). Since the struct is opaque, field access `self.value` cannot be
verified. This is a fundamental limitation.

### 3. Cache::get (lib.rs:190-218) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can we rewrite to avoid `get_mut`?

**Result:** No. Two independent blockers: (a) `BTreeMap::get_mut` has no vstd spec even in
the `std` btree module, and (b) it returns `Option<&mut V>` — a Verus `&mut` return type
limitation. Additionally, the function constructs `CacheGuard` with `&mut entry.value`,
which requires `&mut` access. No rewrite avoids all three blockers.

### 4. Cache::put (lib.rs:230-265) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can we rewrite to avoid `get_mut` and body-verify?

**Result:** No, due to source integrity. A rewrite would change the existing-key path from
in-place `get_mut` mutation to a `remove` + `insert` sequence — a structural exec code
modification. The `contains_key` alternative also has the `Borrow<Q>` blocker (same as
`remove`). Per verus-constraints, exec code modifications beyond the escalation ladder
are not permitted.

### 5. Cache::evict (lib.rs:321-344) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can we rewrite with a manual loop to avoid iterator chains?

**Result:** No. The function uses `self.entries.iter().min_by_key(|(_, e)| e.last_used)`.
Even a manual loop would require `BTreeMap::iter()` which has no vstd spec. There is no
vstd-supported way to iterate over `BTreeMap` entries. A from-scratch rewrite tracking the
minimum via a separate data structure would be a substantial exec code modification.

### 6. axiom_cache_lru_of_remove (lib.proof.rs:408-418) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can this axiom be proven instead of assumed?

**Result:** No, under the current design. `cache_lru_of` delegates to the uninterpreted
`cache_lru_of_nonempty` for non-empty maps, so there is no definitional body to reason about.
Even with a concrete definition (e.g., spec-level sort-by-`last_used` over Map keys), proving
this would require: (1) a recursive sort-by-value function over `Map` (vstd `Map` has no
ordering primitives), (2) a stability-under-removal lemma for that sort function, and
(3) significant proof effort. The axiom statement is small and obviously sound (BTreeMap::remove
does not change `last_used` counters of remaining entries).

### 7. ExBTreeMap (lib.vstd_btree.rs:31-38) — KEEP

**Classification:** EXTERNAL_TYPE

**Challenge:** Can we use vstd's BTreeMap support directly?

**Result:** No. vstd's btree specs are gated behind `cfg(all(feature = "alloc", feature = "std"))`
and import from `std::collections`. This crate is a no\_std kernel target — the `std` crate
does not exist on `i686-nanvix`.

### 8. ExCacheGuard (lib.spec.rs:23-25) — KEEP

**Classification:** VERUS_LIMITATION

**Challenge:** Can we avoid external_body on the type?

**Result:** No. `CacheGuard` has field `value: &'a mut V`. Verus error: "The verifier does not
yet support &mut types, except in special cases" on the struct definition.

### assume_specification items (lib.vstd_btree.rs) — KEEP

**Classification:** EXTERNAL_BOTTOM (stdlib specs)

All 5 are copies of upstream vstd specs adapted for `alloc::collections::BTreeMap`. They
specify `BTreeMap::{new, len, is_empty, insert, clear}` and are semantically identical to
the upstream specs — only the import path (`alloc::` vs `std::`) differs. These exist because
vstd gates its btree specs behind `cfg(std)` which is unavailable on this no\_std target.

### Trust Assumption: Counter Overflow

Within the `external_body` functions `Cache::get` and `Cache::put`, `self.counter += 1`
(`u64`) has no overflow guard. The spec transitions use abstract `Seq` ordering that doesn't
model counters, so the spec is correct, but the implementation's correctness depends on no
overflow occurring. At 10 billion ops/sec, overflow requires ~58 years — physically
unreachable. See `bugs.md` BUG-1.

## AST Consistency

- **Matched:** 16
- **Mismatched:** 2
- **Missing:** 0
- **Extra:** 1 (btreemap_remove — stdlib wrapper)

### MISMATCH: Cache::new

```diff
--- source
+++ verus
     pub const fn new(capacity: usize) -> Self {
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
     }
```

**Cause:** Pre-approved deviation — `Ok(Self { .. })` → `let result = Self { .. }; result`.
The `ensures` clause references the return value, requiring it to be named. The proof block
is ghost code erased at compile time. Semantics are identical.

**Action:** ACCEPT (pre-approved deviation).

### MISMATCH: Cache::remove

```diff
--- source
+++ verus
     pub fn remove(&mut self, key: &K) {
-        self.entries.remove(key);
+        btreemap_remove(&mut self.entries, key);
+        proof! { ... }
     }
```

**Cause:** Escalation ladder step 4 (stdlib wrapper). `BTreeMap::remove`'s `Borrow<Q>` generic
parameter prevents `assume_specification`. The wrapper `btreemap_remove` fixes `Q=K` and
is a single stdlib call (`m.remove(k)`). The proof block is ghost code.

**Action:** ACCEPT (stdlib wrapper deviation, documented with `// VERUS REWRITE` comment).

### EXTRA: btreemap_remove

**Cause:** New function added as a stdlib wrapper for `BTreeMap::remove`. Required by the
`Cache::remove` deviation above. Body is a single stdlib call.

**Action:** ACCEPT (stdlib wrapper, classified STDLIB_WRAPPER in trust.md).

## Result: PASS

All `external_body` items are justified and documented. No items could be eliminated.
No `admit()`, `assume()`, `trusted`, or `cfg-gated exec` cheating detected. AST mismatches
are pre-approved deviations or stdlib wrapper patterns. Verification passes with 0 errors.
