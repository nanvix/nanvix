# Independent Review: cache (GPT-5.3-Codex)

## 1. Cheating Detection

I ran the requested checks and `make verify-cache`.

- `admit()`: **0**
- `assume()`: **0**
- `external_body` attributes: **9**
  - 7 function-level (`lib.rs`)
  - 2 external type specs (`lib.spec.rs`)
- `trusted`: **0**
- `exec_allows_no_decreases_clause`: **0**
- `#[cfg(not(verus_keep_ghost))]` in exec code (`lib.rs`): **0**

Notes:
- Raw text search for `external_body` returns 11 due to 2 comment mentions; attribute count is 9.
- Counts match `fix_report.md` for cheating categories.

## 2. Trust Item Challenges

### ExBTreeMap
- **Classification (`EXTERNAL_TYPE`) is acceptable**, but rationale in `lib.spec.rs` comment is outdated.
- vstd **does** have BTreeMap specs in `vstd/std_specs/btree.rs`.
- However, no_std claim is valid: module is gated by `cfg(all(feature = "alloc", feature = "std"))` in `vstd/std_specs/mod.rs`, and `btree.rs` imports `std::collections::*`.
- Could `assume_specification` be used locally for `alloc::collections::BTreeMap`? **Yes, in principle**, but that introduces substantial new trusted axioms and still requires a concrete `Cache::view` model to prove bodies.

### ExCacheGuard
- **Classification (`VERUS_LIMITATION`) is correct.**
- Minimal reproducer confirms Verus rejects struct fields of type `&mut` with:  
  `The verifier does not yet support ... &mut types, except in special cases`.

### Cache::new/get/put/remove/clear/evict
- `new/remove/clear`: potentially body-verifiable only if custom BTreeMap assume-specs + concrete `view()` are introduced.
- `get`: blocker is real. `get_mut` is unsupported (isolated repro fails with `&mut` limitation) and `CacheGuard` requires `&mut` internals.
- `put`: current `get_mut` use is avoidable (rewrite with remove+insert), but verification still depends on broader BTreeMap spec/view work and `evict`.
- `evict`: current iterator chain could be rewritten as explicit loop; this specific blocker is not fundamental. But proving still needs iterator/remove specs for the no_std BTreeMap path and concrete view relation.
- `deref`: with current `CacheGuard` shape, external_body is justified. Restructuring to avoid `&mut` would require major API/representation changes (likely unsafe/raw-pointer based), not a surgical verification change.

### CacheGuard::deref_mut
- Correctly excluded from verification.
- Exact issue: signature/body use `&mut V`; Verus reports unsupported `&mut` types (except special cases).

### Counter overflow assumption
- Correctly documented as trust assumption (BUG-1).
- Real semantic risk in theory (wrap breaks LRU ordering), but practically remote.

## 3. AST Consistency

I ran AST consistency summary and report.

- Functions matched: **18/18**
- Structs matched: **3/3**
- Mismatches: **0**
- Missing/extra: **0**

No per-item diff needed (no mismatches).

## 4. Verification Status

`make verify-cache` output shows:
- Verus verification exit code: **0** (`=== Results === Exit code : 0`)
- Make target exits non-zero because cheating checker flags `external_body` (`status: CHEATING_DETECTED`).

So: **verification passes; policy gate fails due surviving trust items**.

## 5. Bug vs Limitation

Surviving `external_body` items:

- `CacheGuard::deref`: **Limitation** (opaque external type due `&mut` field)
- `Cache::new`: **Limitation** (no_std BTreeMap spec path + uninterpreted view)
- `Cache::get`: **Limitation** (`get_mut` + `&mut` limitations + CacheGuard)
- `Cache::put`: **Mixed, primarily Limitation**
  - Limitation: same ecosystem blockers as above
  - Potential latent bug sensitivity: counter overflow (BUG-1)
- `Cache::remove`: **Limitation**
- `Cache::clear`: **Limitation**
- `Cache::evict`: **Limitation**, and logic appears correct under no-overflow assumption

Special checks:
- `put` threshold `entries.len() >= capacity` is **correct**.
- `evict` chooses minimum `last_used`; with monotonic non-overflow counter this is correct LRU victim.
- BUG-1 remains a real (low-practicality) defect risk masked by trust assumptions.

## 6. vstd Search Results

Direct search confirms:
- `vstd/std_specs/mod.rs` gates btree behind `cfg(all(feature = "alloc", feature = "std"))`.
- `vstd/std_specs/btree.rs` exists and includes extensive `assume_specification` for BTreeMap (`new`, `insert`, `get`, `remove`, `clear`, `len`, `iter`, etc.).
- `get_mut` is **not** specified there.
- btree specs rely on `std::collections::*`, confirming incompatibility with this crate’s no_std kernel build mode.

## 7. Challengeable Items (should have been eliminated)

1. **The claim “BTreeMap has no vstd specs” should have been eliminated/corrected everywhere** (it is corrected in trust doc/fix report, but stale comment remains in `lib.spec.rs`).
2. **`evict`-via-iterator-chain as a blocker is challengeable**: it could be rewritten as a loop; this does not alone remove trust, but the stated reason is too narrow.
3. **`put` `get_mut` blocker is partially challengeable**: body can be rewritten to avoid `get_mut` (remove+insert path), though this still does not eliminate major trust without additional spec infrastructure.

## 8. Verdict

**FAIL (strict integrity audit perspective).**

Reason: while cheating counts are correctly reported and limitations are mostly real, at least two rationale points are overstated/narrow (iterator-chain blocker and “no BTreeMap specs” messaging in source comments). Trust minimization could likely improve (at least partially) with targeted rewrites plus local assume-spec infrastructure, though not to zero under current Verus `&mut` limitations.
