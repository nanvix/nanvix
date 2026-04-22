# Independent Review: cache (GPT-5.3 Codex)

## 1. Spec Preservation
Compared `lib.rs` at `a48beb884` vs current.

- `Cache::new` removed explicit field ensures (`contents/capacity/lru_order`) and removed `external_body`.
  - **Implied**: `result@ == CacheView::spec_new(capacity as nat)` plus `spec_new` definition gives exactly empty contents, preserved capacity, empty LRU.
  - No weakening.
- `Cache::get` removed hit-case `self@.contents == old(self)@.contents` and `self@.capacity == old(self)@.capacity`.
  - **Implied**: `self@ == old(self)@.spec_get(*key).0`; in `spec_get` hit branch uses `..self`, so contents/capacity unchanged.
  - Added `result->Some_0@ == old(self)@.spec_get(*key).1.unwrap()` (strictly stronger result linkage).
- `Cache::put` removed capacity-preservation, put-get round-trip, and zero-capacity no-op clauses.
  - **Implied**: `self@ == old(self)@.spec_put(key, value)`; all `spec_put` branches preserve `capacity` via `..self`; `capacity==0` branch returns `self`; `capacity>0` branches all produce map containing `key -> value`.
- `Cache::remove` removed capacity-preservation, key-absent-after-remove, and absent-key no-op clauses.
  - **Implied**: `self@ == old(self)@.spec_remove(*key)`; `spec_remove` uses `..self` (capacity preserved), present branch removes key, absent branch returns `self`.
- `Cache::clear` removed explicit empty-contents, empty-LRU, and capacity-preservation clauses.
  - **Implied**: `self@ == old(self)@.spec_clear()`; `spec_clear` sets empty contents/LRU and preserves capacity via `..self`.

**Conclusion:** No removed clause is a genuine caller-visible weakening; all are derivable from retained transition-equality ensures and open transition definitions.

## 2. Cheating Audit
- `admit()`: **0** (checked by search and `make verify-cache` cheating report).
- `assume(...)`: **0**.
- cfg-gated exec code: **0 suspicious** (`#[cfg(verus_keep_ghost)]` includes and test cfg only; no behavior-hiding cfg on exec logic).
- `assume_specification` in `lib.vstd_btree.rs`: **5** (`new`, `len`, `is_empty`, `insert`, `clear`).
  - Assessment: expected trust bridge for `alloc::collections::BTreeMap` on `no_std`; still a substantial trusted base.

`external_body`: **8 total**
1. `lib.rs:97` `CacheGuard::deref` — in `trust.md` ✅, explicit reproducer ❌ (only indirect rationale).
2. `lib.rs:121` `btreemap_remove` — in `trust.md` ✅, explicit reproducer ❌.
3. `lib.rs:208` `Cache::get` — in `trust.md` ✅, explicit reproducer partial (limitation explained, no minimal script).
4. `lib.rs:238` `Cache::put` — in `trust.md` ✅, explicit reproducer partial.
5. `lib.rs:335` `Cache::evict` — in `trust.md` ✅, explicit reproducer ❌.
6. `lib.spec.rs:24` `ExCacheGuard` type body — in `trust.md` ✅, reproducer ✅.
7. `lib.vstd_btree.rs:32` `ExBTreeMap` type body — in `trust.md` ✅, reproducer ✅.
8. `lib.proof.rs:409` `axiom_cache_lru_of_remove` — in `trust.md` ✅, explicit reproducer N/A (axiom; needs stronger soundness argument than currently provided).

Escalation-ladder check:
- `btreemap_remove` is already a stdlib wrapper + thin external body (reasonable endpoint).
- `get`/`put` still blocked by `get_mut` + `&mut` modeling limitations.
- `evict` likely removable only with non-trivial redesign (e.g., model/maintain ghost LRU order explicitly rather than iterator/min_by_key chain).
- `axiom_cache_lru_of_remove` is the most concerning trust point; currently asserts key-order relation over an uninterpreted ordering function.

## 3. Proof Quality
- Invariant lemmas are structured and reusable (filter/no-dup/set/len decomposition), not brute-force SMT spam.
- `new/clear/remove` bridge lemmas are coherent.
- However, `axiom_cache_lru_of_remove` is an unproven proof-level trust leap over uninterpreted `cache_lru_of_nonempty`; this is a significant proof-quality/soundness gap.

## 4. Rewrite Audit
`Cache::remove` rewrite (`self.entries.remove(key)` -> `btreemap_remove(...)` + proof block):
- Minimality: **mostly minimal** for enabling body verification.
- Semantic equivalence: **yes**, wrapper body is exactly `m.remove(k)` with matching map-view ensures.
- Avoidability: likely avoidable only with broader spec infrastructure changes for alloc BTreeMap `remove` (non-minimal).

## 5. Verification Status
Command run: `make verify-cache 2>&1 | tail -30`
- Verifier phase: `Exit code : 0` (cached; no verification errors reported).
- Overall target result: **FAIL** due cheating check (`external_body: 8`), and `make` exits with error.

## Issues (highest priority first)
1. **Unproven axiom trust gap:** `axiom_cache_lru_of_remove` is external-body proof axiom over uninterpreted LRU function; strongest unsoundness risk.
2. **High trusted surface:** 8 external_body + 5 assume_specification leaves substantial behavior trusted, not proven.
3. **Trust docs lack reproducibility quality** for several external bodies (`deref`, `btreemap_remove`, `evict`, and partial for `get`/`put`).
4. **Pipeline status not clean:** `make verify-cache` fails policy gate (CHEATING_DETECTED), so verification is not “green” at target level.

## Verdict: FAIL
Specs were not weakened, but the trust boundary remains too large (especially axiom-level external proof), and the repository’s own verify target currently fails its cheating gate.
