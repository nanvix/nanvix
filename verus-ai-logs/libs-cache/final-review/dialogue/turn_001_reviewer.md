## Turn 1: external_body on Cache::new — Step 1+4 (assume_specification for BTreeMap::new)

### Progress Tracker
- external_body on user functions: **7 remaining** (each is a defect)
  1. `Cache::new` — lib.rs:141
  2. `Cache::get` — lib.rs:168
  3. `Cache::put` — lib.rs:208
  4. `Cache::remove` — lib.rs:254
  5. `Cache::clear` — lib.rs:271
  6. `Cache::evict` — lib.rs:289
  7. `CacheGuard::deref` — lib.rs:91
- external_type_specification: 4 (expected, not defects)
- Spec quality issues: 0 (specs are high quality — not the concern here)
- Current drill-down: function `Cache::new`, step 4/6

### Verification of Previous Fix
First turn, no previous fix.

### Foundational Challenge

The final review classified all 7 `external_body` items as `VERUS_LIMITATION`
with the rationale that `assume_specification` would "merely shift trust to
unverified axioms, not eliminate it." **This rationale is wrong.** Here's why:

- `external_body` on `Cache::put` trusts **all user logic**: zero-capacity
  handling, overwrite behavior, counter bumping, eviction logic, map mutation —
  every line of user code is unverified.
- `assume_specification` on `BTreeMap::insert` trusts **only the stdlib
  primitive**. All user logic (the four branches of `put`, the counter, the
  eviction decision) would be **verified**.

This is not "merely shifting trust" — it is a **strictly smaller trust
boundary**. It is also exactly the standard Verus pattern: vstd itself uses
`assume_specification` for all BTreeMap methods (`vstd/std_specs/btree.rs`).

### Issue

**Step 1 (completed by reviewer):** I removed `external_body` from `Cache::new`
and ran `make verify-cache`. The exact error is:

```
error: `alloc::collections::btree::map::impl&%18::new` is not supported
  (note: you may be able to add a Verus specification to this function
   with `assume_specification`)
  = help: The following declaration may resolve this error:
          pub assume_specification<K, V> [alloc::collections::BTreeMap::<K, V>::new] ()
              -> alloc::collections::BTreeMap<K, V>;
```

**Verus itself tells us the fix: `assume_specification`.**

**Step 3 (completed by reviewer):** vstd already provides the exact spec at
`vstd/std_specs/btree.rs:613-616`:

```rust
pub assume_specification<Key, Value>[ BTreeMap::<Key, Value>::new ]() -> (m: BTreeMap<Key, Value>)
    ensures
        m@ == Map::<Key, Value>::empty(),
;
```

This is gated behind `cfg(all(feature = "alloc", feature = "std"))` only because
vstd uses `use std::collections::BTreeMap` — but `std::collections::BTreeMap` is
a re-export of `alloc::collections::BTreeMap`, which this crate already uses.

### Specific Question

**Step 4 — Fix attempt for `Cache::new`:**

Add a local `assume_specification` for `alloc::collections::BTreeMap::new` in
`lib.spec.rs` (following the vstd pattern above), remove `external_body` from
`Cache::new`, and run `make verify-cache`.

Report the EXACT result — either it verifies successfully, or show the **full
new error message**. If a new error appears (e.g., about the uninterpreted
`view()`), that's expected — report it and we'll address it in the next turn.

Note: You will also need to add `View` impl for `BTreeMap` via
`assume_specification` or equivalent, since the vstd BTreeMap View
(`impl<Key, Value, A: Allocator + Clone> View for BTreeMap<Key, Value, A>`)
is also gated behind `cfg(std)`. The cache crate's `lib.spec.rs` already
declares `ExBTreeMap` as `external_type_specification` + `external_body`, and
already has an `uninterp` `View` for `Cache` — you may need to modify these.

### Evidence

**Commands run by reviewer:**

1. Baseline verification:
```
$ make verify-cache
verification: cached (no recompilation), — (exit 0)
cheating: assume=0 external_body=9 admit=0 trusted=0 no_decreases=0 cfg_gate=0
```

2. Removed `external_body` from `Cache::new`, ran verification:
```
$ make verify-cache
error: `alloc::collections::btree::map::impl&%18::new` is not supported
  = help: pub assume_specification<K, V> [alloc::collections::BTreeMap::<K, V>::new] ()
              -> alloc::collections::BTreeMap<K, V>;
Exit code: 101
```

3. Verified vstd btree module gating:
```
$ grep -B5 'pub mod btree' ~/verus/vstd/std_specs/mod.rs
#[cfg(all(feature = "alloc", feature = "std"))]
pub mod btree;
```

4. Confirmed vstd BTreeMap::new spec exists at `vstd/std_specs/btree.rs:613-616`.

5. Confirmed vstd BTreeMap View impl at `vstd/std_specs/btree.rs:457-461`:
```rust
impl<Key, Value, A: Allocator + Clone> View for BTreeMap<Key, Value, A> {
    type V = Map<Key, Value>;
    uninterp spec fn view(&self) -> Map<Key, Value>;
}
```
