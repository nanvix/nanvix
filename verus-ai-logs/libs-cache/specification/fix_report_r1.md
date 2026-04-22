# Fix Report — Round 1

Addresses all issues from `review_r1.md`.

## Issue 1 (🔴 CRITICAL): `get` return value unspecified

**Fix:** Added `View for CacheGuard` (uninterpreted, mapping to `V`) in
`lib.spec.rs`. Strengthened `get` ensures on hit to include:
```
result->Some_0@ == old(self)@.spec_get(*key).1.unwrap()
```
This connects the exec return value to the spec transition's `Option<V>`
component — the single source of truth for the cached value. Callers can now
prove they receive the correct value through `guard@ == cached_value`.

**Files changed:** `lib.spec.rs` (View impl), `lib.rs` (get ensures).

## Issue 2 (🔴 CRITICAL): `CacheGuard::Deref`/`DerefMut` have no specs

**Fix (Deref):** Added `#[verus_verify]` on the `impl Deref for CacheGuard`
block and `#[verus_verify(external_body)]` + `#[verus_spec]` on `deref()`:
```
ensures *ret == self@
```
This completes the specification chain: `put` → `get` → `*guard`.

**Fix (DerefMut):** Attempted adding identical spec to `deref_mut`. Verus
rejects the `&mut V` return type with error: "The verifier does not yet
support &mut types, except in special cases". This is a confirmed Verus
limitation — documented in `trust.md` with the exact reproducer error.

**Coverage:** 7/8 exec functions now have contracts. Only `deref_mut` remains
unverifiable.

**Files changed:** `lib.rs` (Deref impl block), `trust.md` (DerefMut entry).

## Issue 3 (🟡 MODERATE): Pervasive subsumed clauses

**Fix:** Pruned all field-level unfoldings from every function. Retained only:
- Canonical transition: `self@ == old(self)@.spec_X(args)` (or `result@ == ...`)
- Invariant preservation: `self@.inv()`

Since all spec transitions are `pub open spec fn`, callers can unfold them
directly to derive any needed field-level properties.

**Functions pruned:**
- `new`: removed `contents == Map::empty()`, `capacity == capacity as nat`,
  `lru_order == Seq::empty()` (3 clauses)
- `get` hit: removed `contents == old.contents`, `capacity == old.capacity`
  (2 clauses)
- `put`: removed `capacity == old.capacity`, put-get round-trip block,
  zero-capacity no-op block (3 clauses)
- `remove`: removed `capacity == old.capacity`, `!contains(*key)`,
  key-absent no-op (3 clauses)
- `clear`: removed `contents == Map::empty()`, `lru_order == Seq::empty()`,
  `capacity == old.capacity` (3 clauses)

**Files changed:** `lib.rs` (all 5 functions).

## Issue 4 (🟡 MODERATE): Counter overflow unaddressed

**Fix:** Created `bugs.md` with BUG-1 documenting the `u64` counter overflow
risk. Added "Trust Assumptions" section to `trust.md` documenting the
counter-never-overflows assumption with justification (2^64 ops at 10B/sec
= ~58 years).

**Files changed:** `bugs.md` (new), `trust.md` (new section).

## Issue 5 (🟢 MINOR): `inv()` redundant conjunct

**No change.** The `lru_order.len() == contents.dom().len()` conjunct is
retained as an SMT solver hint, per review's own recommendation.

## Issue 6 (🟢 MINOR): No bugs.md file

**Fix:** Created `bugs.md` (see Issue 4 above).

## Verification Results

- `make verify-cache`: **5 verified, 0 errors**
- `make verify-bitmap`: 0 errors (cached, no regression)
- `make verify-slab`: 0 errors (cached, no regression)
- `make build`: success (dual compilation works)
- AST consistency: **18/18 MATCH** (no exec code changes)
- Coverage: **7/8** exec functions have contracts (`deref_mut` excluded)
