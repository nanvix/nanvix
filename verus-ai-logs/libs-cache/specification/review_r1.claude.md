# Spec Review (Claude Opus 4.6): `cache` crate

## 1. Caller Coverage Issues

| Function | Status | Notes |
|---|---|---|
| `new` | **SUBSUMED** | `result@ == spec_new(…)` already determines everything; 4 additional clauses are redundant since `spec_new` is `open spec` |
| `get` | **MISSING_SPEC** | Return value is unconstrained on hit — only `result is Some`, but no connection to the stored value. `CacheGuard` has no `View` and `Deref`/`DerefMut` have no specs. Caller cannot prove anything about the data received. |
| `put` | **SUBSUMED** | `self@ == spec_put(…)` fully determines post-state; round-trip and zero-cap clauses are derivable from the `open spec` definition |
| `remove` | **SUBSUMED** | `self@ == spec_remove(…)` suffices; remaining 4 clauses are derivable |
| `clear` | **SUBSUMED** | `self@ == spec_clear()` suffices; remaining 4 clauses are derivable |
| `evict` | OK | Internal function, well-specified |
| `Deref for CacheGuard` | **MISSING_SPEC** | No `#[verus_spec]` at all. This is the only way callers extract values from `get`. |
| `DerefMut for CacheGuard` | **MISSING_SPEC** | Same as above; callers cannot prove mutation effects on the cached value |

## 2. Spec Quality Assessment

**`new`**
- Requires: none (correct)
- Ensures: strong enough but bloated. `== spec_new(…)` is canonical; the other 4 unfold it.
- Recommendation: Keep only `result@ == spec_new(…)` and `result@.inv()`.

**`get`**
- Requires: `inv()` — appropriate.
- Ensures (hit): state transition via `spec_get` is good, but **return value is opaque**. `result is Some` tells nothing about the value inside `CacheGuard`. `spec_get` returns `(CacheView, Option<V>)` but the `Option<V>` is never connected to the result.
- Ensures (miss): `self@ == old(self)@` is a clean frame condition.
- Hit-path subsumed: `contents ==`, `capacity ==`, `inv()` — all derivable from `== spec_get(…).0`.
- **Critical gap**: No `CacheGuard::view()` or `Deref` spec.

**`put`**
- `== spec_put(…)` is canonical. Round-trip and zero-cap are conveniences derivable from the open definition.
- No return value to constrain. Good.

**`remove`**
- Clean. No issues beyond subsumption.

**`clear`**
- Clean. No issues beyond subsumption.

**`evict`**
- Private, well-specified. `requires contents.dom().len() > 0` is appropriate.

## 3. Determinism Analysis

| Function | Result | Detail |
|---|---|---|
| `new` | COMPLETE | `spec_new` fully determines `result@` |
| `get` (state) | COMPLETE | `spec_get` fully determines `self@` |
| `get` (return) | **INCOMPLETE** | On hit: only `result is Some`. Value inside `CacheGuard` is unconstrained. |
| `put` | COMPLETE | `spec_put` fully determines `self@`; returns `()` |
| `remove` | COMPLETE | `spec_remove` fully determines `self@`; returns `()` |
| `clear` | COMPLETE | `spec_clear` fully determines `self@`; returns `()` |
| `evict` | COMPLETE | Fully determined |

## 4. Missing Properties

1. **Get-value correctness** — `CacheGuard` is opaque. Need View or Deref spec.
2. **Guard mutation effect** — `DerefMut` behavior on cache state unspecified.
3. **Counter overflow** — No precondition guarding against `u64` wrapping.
4. **Size observability** — No `len()` or `is_empty()` function.

## 5. Specs to Remove

All field-level unfoldings in `new`, `get`, `put`, `remove`, `clear` are subsumed by the `== spec_*(…)` clause. Keep `== spec_*(…)` + `inv()` only.

## 6. Issues (highest priority first)

1. 🔴 `get` return value unspecified — defeats cache purpose
2. 🔴 `CacheGuard::Deref`/`DerefMut` have no specs
3. 🟡 Pervasive subsumed clauses (bloat)
4. 🟡 Counter overflow unaddressed
5. 🟢 `inv()` has a redundant conjunct (acceptable as SMT trigger)

## 7. Suspected Bugs

No functional bugs identified. Counter overflow is theoretical, not practical.
