# Spec Review (GPT-5.3-Codex): `cache` crate

## 1. Caller Coverage Issues

| Function | Status | Notes |
|---|---|---|
| `Cache::new` | OK | Covers empty state + capacity. |
| `Cache::get` | MISSING_SPEC | Hit/miss split is good, but returned `CacheGuard` is under-specified (no link to key/value). |
| `Cache::put` | OK | Transition via `spec_put` captures insert/overwrite/evict semantics. |
| `Cache::remove` | OK | Transition + idempotent no-op covered. |
| `Cache::clear` | OK | Clear-to-empty + capacity preserved covered. |
| `CacheGuard::deref` | MISSING_SPEC | No Verus contract; guard consistency expectation not formalized. |
| `CacheGuard::deref_mut` | MISSING_SPEC | No Verus contract; mutation-through-guard behavior unmodeled. |

## 2. Spec Quality Assessment

- **`new`**: strong enough; several ensures are **subsumed** by `result@ == spec_new(...)`.
- **`get`**: requires are reasonable; frame on miss is good. Hit path lacks semantic guarantee for returned guard target/value.
- **`put`**: strong transition spec; useful caller-facing facts included. `capacity preserved` and `capacity==0 ==> self unchanged` are derivable from `spec_put` (possible bloat).
- **`remove`**: strong transition spec; "key absent => no-op" and "key not present after" derivable from `spec_remove`.
- **`clear`**: strong but redundant clauses derivable from `spec_clear`.
- No tautological clauses found.
- Error/None path handling exists where applicable.

## 3. Determinism Analysis

| Function | Result |
|---|---|
| `Cache::new` | COMPLETE |
| `Cache::get` | INCOMPLETE (`result` on hit under-constrained: only `Some`, not what it points to) |
| `Cache::put` | COMPLETE (`()` + exact post-state) |
| `Cache::remove` | COMPLETE |
| `Cache::clear` | COMPLETE |
| `CacheGuard::deref` | INCOMPLETE (no spec) |
| `CacheGuard::deref_mut` | INCOMPLETE (no spec) |

## 4. Missing Properties

- Guard/value correspondence: `get(k)` hit should specify guard refers to `contents[k]`.
- Post-`deref_mut` effect on cache abstract state is unmodeled.
- No explicit spec-level handling/assumption for `counter: u64` overflow and LRU ordering impact.

## 5. Specs to Remove

Remove/trim **subsumed** ensures to reduce noise:
- `new`: `inv`, `contents empty`, `capacity`, `lru_order empty` (all implied by `spec_new` equality).
- `put`: `capacity preserved`, `capacity==0 ==> self==old` (implied by `spec_put`); keep round-trip only if caller convenience is desired.
- `remove`: no-present/no-op clauses implied by `spec_remove`.
- `clear`: emptiness/order/capacity clauses implied by `spec_clear`.

## 6. Issues (highest priority first)

1. Add contracts for `CacheGuard::deref` and `deref_mut` (caller-visible API gap).
2. Strengthen `get` hit ensures to relate returned guard to queried key/value.
3. Prune subsumed ensures to avoid bloated specs and maintenance burden.
4. Decide/document overflow policy for `counter` (trusted assumption vs explicit behavior).

## 7. Suspected Bugs

- **Potential**: `counter` overflow can break strict recency ordering (or panic in checked builds), which may violate intended LRU semantics over very long runs.
- No immediate functional bug otherwise stands out in the shown implementation.
