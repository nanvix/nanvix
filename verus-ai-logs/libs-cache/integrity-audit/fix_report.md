# Integrity Audit Report: cache

## Cheating Counts (before → after)

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 9 | 9 | 0 |
| trusted | 0 | 0 | 0 |
| no_decreases | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |

## Items Eliminated

None. All 9 `external_body` items were challenged and found to be genuinely
unverifiable due to hard constraints of the `no_std` kernel target.

## Items Remaining in trust.md

### External Type Specifications (2 with external_body)

1. **ExBTreeMap** — `lib.spec.rs:20-23`
   - Classification: `EXTERNAL_TYPE`
   - Reason: `alloc::collections::BTreeMap` requires type declaration for Verus.
     vstd provides BTreeMap specs (`vstd::std_specs::btree`), but the module is
     gated behind `cfg(all(feature = "alloc", feature = "std"))` and internally
     imports from `std::collections`, making it structurally incompatible with this
     crate's `no_std` kernel target (`-Z build-std=core,alloc,compiler_builtins`).
   - Reproducer: Enabling `std` feature for vstd would require the `std` crate,
     which is not built for the i686-nanvix kernel target.

2. **ExCacheGuard** — `lib.spec.rs:35-38`
   - Classification: `VERUS_LIMITATION`
   - Reason: `CacheGuard<'a, V>` has field `value: &'a mut V`. Verus does not
     support `&mut` in struct field types.
   - Reproducer: Adding `#[verus_verify]` to CacheGuard produces "The verifier
     does not yet support &mut types, except in special cases".

### External Type Specifications (2 without external_body)

3. **ExGlobal** — `lib.spec.rs:25-26`
   - Classification: `EXTERNAL_TYPE`
   - Reason: Default allocator type parameter for BTreeMap.

4. **ExCacheEntry** — `lib.spec.rs:29-31`
   - Classification: `EXTERNAL_TYPE`
   - Reason: Internal struct used as BTreeMap value type.

### Function-level external_body (7 functions)

5. **CacheGuard::deref** — `lib.rs:95`
   - Classification: `VERUS_LIMITATION`
   - Reason: CacheGuard is `external_body` (due to `&mut V` field), so Verus
     cannot see the `self.value` field access.

6. **Cache::new** — `lib.rs:147`
   - Classification: `VERUS_LIMITATION`
   - Reason: Body calls `BTreeMap::new()`. BTreeMap vstd specs unavailable on
     `no_std` target. Cache::view() is uninterp, so body cannot be verified
     against abstract state even with specs.

7. **Cache::get** — `lib.rs:186`
   - Classification: `VERUS_LIMITATION`
   - Reason: Body calls `BTreeMap::get_mut()` which (a) has no vstd spec even in
     the `std` btree module, and (b) returns `Option<&mut V>` — a Verus &mut
     return type limitation. Also constructs CacheGuard with &mut.

8. **Cache::put** — `lib.rs:216`
   - Classification: `VERUS_LIMITATION`
   - Reason: Same `get_mut` blockers as Cache::get, plus calls `self.evict()`.

9. **Cache::remove** — `lib.rs:262`
   - Classification: `VERUS_LIMITATION`
   - Reason: Calls `BTreeMap::remove()`. vstd btree specs unavailable on
     `no_std` target.

10. **Cache::clear** — `lib.rs:279`
    - Classification: `VERUS_LIMITATION`
    - Reason: Calls `BTreeMap::clear()`. vstd btree specs unavailable on
      `no_std` target.

11. **Cache::evict** — `lib.rs:303`
    - Classification: `VERUS_LIMITATION`
    - Reason: Uses `self.entries.iter().min_by_key(|(_, e)| e.last_used)`
      iterator chain with closure, plus `BTreeMap::remove()`. Iterator
      combinators lack vstd specs.

### Unverifiable Function (no spec possible)

12. **CacheGuard::deref_mut** — `lib.rs:101`
    - Classification: `VERUS_LIMITATION`
    - Reason: Returns `&mut V`. Verus error: "The verifier does not yet support
      &mut types, except in special cases".

## AST Consistency

- **Matched: 18** (all functions and structs)
- **Mismatched: 0**
- **Missing: 0**

All exec code is unchanged from the `dev` branch. Only Verus annotations
(`#[verus_verify]`, `#[verus_spec]`, cfg-gated imports/includes, feature flags)
were added.

## Evaluated and Rejected Alternatives

### Custom assume_specification for BTreeMap methods

**Evaluated:** Writing project-specific `assume_specification` for
`alloc::collections::BTreeMap` methods (new, insert, remove, clear, len)
to bypass the vstd `std` feature gating.

**What it would unblock:** At most `Cache::new`, `Cache::remove`, and
`Cache::clear` — these only call BTreeMap methods with straightforward
specs. This also requires making `Cache::view()` concrete, which means
defining a spec-level sort function to produce `lru_order: Seq<K>` from
counter-based ordering.

**Why rejected:**
1. `Cache::get` and `Cache::put` remain `external_body` regardless because
   they call `BTreeMap::get_mut`, which has no vstd spec and returns
   `Option<&mut V>` (Verus limitation).
2. `Cache::evict` remains `external_body` due to iterator chain with closure.
3. `CacheGuard::deref` remains `external_body` because CacheGuard's struct
   fields are opaque.
4. Net result: eliminating 3 out of 7 function `external_body` items at
   significant cost (concrete View, spec-level sort, new assume_specification
   trust assumptions), while the 4 most important methods (`get`, `put`,
   `evict`, `deref`) remain trusted.
5. Adding `assume_specification` for BTreeMap merely shifts trust from
   "Cache method body is correct" to "BTreeMap method spec is correct" —
   it does not eliminate trust, only redistributes it.

### Enabling vstd `std` feature

**Evaluated:** Enabling `std` feature for vstd to access built-in btree specs.

**Why rejected:** Hard constraint. The btree module requires
`cfg(all(feature = "alloc", feature = "std"))` and internally imports from
`std::collections`. The cache crate targets the i686-nanvix kernel target
which builds only `core,alloc,compiler_builtins` — the `std` crate does
not exist on this target. Enabling the feature would cause compilation
failure in vstd's btree module.

## trust.md Corrections

The previous trust.md stated: *"BTreeMap (from alloc::collections) has no
vstd specifications."* This is factually imprecise. Corrected to reflect
that vstd provides BTreeMap specs in `vstd::std_specs::btree`, but they are
structurally incompatible with the `no_std` kernel target due to `std`
feature gating and internal `std::collections` imports.

## Result: PASS

All 9 `external_body` items are genuinely unverifiable under current
constraints. No `admit`, `assume`, `trusted`, `no_decreases`, or
`cfg-gated exec` patterns found. AST consistency is perfect (18/18 match).
Proof structure is clean — all 5 invariant preservation lemmas are fully
proven with no cheating. The trust boundary is minimal and well-documented.
