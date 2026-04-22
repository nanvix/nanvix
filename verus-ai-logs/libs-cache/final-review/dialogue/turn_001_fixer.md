## Response to Turn 1

### Goal
Remove `external_body` from `Cache::new` (Step 4 of the reviewer's drill-down: rewrite + add `assume_specification` for `BTreeMap::new`).

### Attempt 1: Direct `impl View for BTreeMap` + `assume_specification`
- Changes: Added `impl View for alloc::collections::BTreeMap<K, V>` in `lib.spec.rs`, plus `assume_specification` for `BTreeMap::new` with `m@ == Map::empty()`. Removed `external_body` from `Cache::new`.
- Result: **FAILED** — Rust orphan rule (`E0117`): cannot implement foreign trait `View` on foreign type `BTreeMap` in a downstream crate.

### Attempt 2: Proxy type `ExBTreeMap` with View + `assume_specification`
- Changes: Implemented `View` on the existing `ExBTreeMap` proxy type, wrote `assume_specification` returning `ExBTreeMap`.
- Result: **FAILED** — `E0308` type mismatch: Rust treats `ExBTreeMap` and `BTreeMap` as different types despite `external_type_specification`.

### Attempt 3: `assume_specification` without View (no `m@`)
- Changes: Added `assume_specification` for `BTreeMap::new` with no View-based ensures. Removed `external_body`. Cache View remained `uninterp`.
- Result: **FAILED** — Postcondition failures: `result@ == CacheView::spec_new(...)` and `result@.inv()` both unprovable because View is uninterpreted. This confirmed the reviewer's expected outcome for step 4.

### Attempt 4: Custom `btreemap_view_spec` + interpreted Cache View
- Changes: Created `pub uninterp spec fn btreemap_view_spec<K,V>(m: BTreeMap<K,V>) -> Map<K,V>` to avoid orphan rules. Rewrote `assume_specification` to use `btreemap_view_spec(m) == Map::empty()`. Created `cache_contents_of` and `cache_lru_of` helper spec functions connecting BTreeMap fields to CacheView. Changed Cache View from `uninterp` to interpreted body using these helpers.
- Sub-attempts for visibility issues:
  - **4a**: `open spec fn` helpers → error: `open` requires `pub`
  - **4b**: Made helpers `pub` → error: `btreemap_view_spec` private in public ensures
  - **4c**: Made `btreemap_view_spec` `pub` → error: `pub open cache_contents_of` accesses private `CacheEntry`
  - **4d**: Made helpers non-pub `spec fn` (closed) → error: `pub open` Cache View can't call private functions
  - **4e**: Made Cache View `closed spec fn` → **COMPILED!** 11 verified, 1 error: `result@.inv()` postcondition fails (needs reveal of closed functions in proof)

### Attempt 5: `proof!` block with inline `reveal`
- Changes: Added `proof! { reveal(<Cache<K,V> as View>::view); ... }` in `Cache::new` body.
- Result: **FAILED** — `E0401`: can't use generic parameters `K`, `V` from outer item in `proof!` (which expands to a nested function).

### Attempt 6: Proof lemma as associated function (bitmap crate pattern)
- Changes: Created `lemma_new_view` as `proof fn` inside `impl<K: Ord + Clone, V> Cache<K, V>` block in `lib.proof.rs`. Called from `Cache::new` via `proof! { Self::lemma_new_view(&result, capacity); }`.
- Sub-attempts for `reveal` syntax:
  - **6a**: `reveal(<Self as View>::view)` → error: "Self is not supported in reveal/hide"
  - **6b**: `reveal(<Cache<K, V> as View>::view)` → error: `E0401` again (K, V from outer impl block)
  - **6c**: `reveal(<Cache<_, _> as View>::view)` → **SUCCESS!** 13 verified, 0 errors.

### Final Result
- Status: **FIXED**
- `external_body` count on user functions: was 7 → now **6** (removed from `Cache::new`)
- Total `external_body` count reported by guardrails: 8 (includes `ExBTreeMap` external type spec + `assume_specification` for `BTreeMap::new`, which are infrastructure, not user functions)
- Verification: **13 verified, 0 errors**

### Changes Made

**`lib.rs`** (lines 146–156):
- Removed `#[verus_verify(external_body)]` from `Cache::new`
- Changed body from direct struct return to `let result = Self {...}; proof! { Self::lemma_new_view(&result, capacity); } result`

**`lib.spec.rs`** (lines 29–69, 216–226):
- Added `btreemap_view_spec<K,V>` — uninterpreted spec function mirroring vstd's `View::view` for BTreeMap (orphan rules prevent implementing the trait directly)
- Added `assume_specification` for `BTreeMap::new` with ensures `btreemap_view_spec(m) == Map::empty()`
- Added `cache_contents_of<K,V>` — closed spec fn projecting BTreeMap contents to `Map<K,V>` via `CacheEntry::value`
- Added `cache_lru_of<K,V>` — closed spec fn with interpreted empty case (`Seq::empty()` when domain is empty) and uninterpreted non-empty case
- Changed Cache `View::view` from `uninterp` to `closed spec fn` with interpreted body using the helpers above

**`lib.proof.rs`** (lines 320–351):
- Added `impl<K: Ord + Clone, V> Cache<K, V>` block with `lemma_new_view` proof function
- Uses `reveal(<Cache<_, _> as View>::view)`, `reveal(cache_contents_of)`, `reveal(cache_lru_of)` to expose closed function bodies
- Proves `result@.contents =~= Map::empty()` and `result@.lru_order == Seq::empty()`

### Trust Boundary
The `assume_specification` for `BTreeMap::new` is a new trust assumption — it axiomatizes that a freshly created BTreeMap is empty. This matches vstd's own spec at `vstd/std_specs/btree.rs:613-616` and is required because vstd's BTreeMap specs are gated behind `cfg(std)`, unavailable on this no_std kernel target. This must be documented in `trust.md`.

### Key Verus Insights Discovered
1. **Orphan rules block `View` for `BTreeMap`** in downstream no_std crates — use standalone spec functions instead
2. **`closed spec fn`** is the correct visibility for View when the body references private types — reveal in proofs
3. **`reveal(<T<_, _> as Trait>::method)`** with wildcards works in generic contexts where explicit type params hit `E0401`
4. **`proof!` blocks** expand to nested Rust functions, so they can't access outer generic params — use associated proof fns on the impl block instead
