# Specification Review: cache

> Consolidated from independent reviews by claude-opus-4.6 and gpt-5.3-codex.
> Raw outputs: `review_r1.claude.md`, `review_r1.gpt.md`.

## Checklist
### Specification
- [x] Every in-scope exec function has requires/ensures
- [x] No tautological ensures (e.g., `Err(_) => true`)
- [ ] No subsumed ensures (derivable from inv() + other ensures)
- [ ] Error paths have meaningful ensures (match style: Ok => ..., Err => ...)
- [x] No assume_specification for workspace-internal code
- [x] vstd searched before any assume_specification
- [ ] Specs written for the caller (usable directly in caller proofs)

**Unchecked items:**
- **Subsumed ensures**: Every function (`new`, `get`, `put`, `remove`, `clear`)
  carries 3–5 ensures clauses that are strict unfoldings of the `== spec_*(…)`
  canonical clause. Since all spec transition functions are `pub open spec fn`,
  callers can unfold them directly. These clauses are redundant bloat.
- **Error paths**: `get` on miss is well-specified (`self@ == old(self)@`), but
  the hit path does not meaningfully constrain the return value — `result is Some`
  without any link to the cached value is effectively a one-sided spec.
- **Caller usability**: `CacheGuard::deref` and `CacheGuard::deref_mut` have
  **no specs at all**. Since these are the only way callers extract values from
  `get`, the specification chain is broken: callers cannot prove they receive
  the correct value.

## Determinism Check
| Function | Result | Under-constrained aspect |
|---|---|---|
| `Cache::get` (return value) | INCOMPLETE | On hit: only `result is Some`. The value inside `CacheGuard` is entirely unconstrained — no View impl, no Deref spec. A conforming implementation could return arbitrary data. |
| `CacheGuard::deref` | INCOMPLETE | No spec at all. |
| `CacheGuard::deref_mut` | INCOMPLETE | No spec at all. |

All other functions (`new`, `put`, `remove`, `clear`, `evict`) are COMPLETE —
the `== spec_*(…)` clause fully determines the post-state and return value.

## Caller Coverage Issues
| Function | Status | Notes |
|---|---|---|
| `Cache::get` | MISSING_SPEC | Return value unconstrained on hit. Caller expectation "guard dereferences to stored value" (caller analysis §CacheGuard::deref) is unverifiable. |
| `CacheGuard::deref` | MISSING_SPEC | No contract. This is the primary value-extraction interface. |
| `CacheGuard::deref_mut` | MISSING_SPEC | No contract. Mutation-through-guard behavior unmodeled. |
| `Cache::new` | SUBSUMED | 4 ensures clauses are unfoldings of `spec_new` (which is `open`). |
| `Cache::put` | SUBSUMED | Round-trip and zero-cap clauses derivable from `spec_put`. |
| `Cache::remove` | SUBSUMED | Key-absent no-op and key-not-present clauses derivable from `spec_remove`. |
| `Cache::clear` | SUBSUMED | Emptiness/order/capacity clauses derivable from `spec_clear`. |

## Missing Properties
1. **Get-value correctness (CRITICAL)** — The fundamental cache contract ("you
   get back what you put in") is unverifiable. `get` on hit says `result is Some`
   but never connects the `CacheGuard` to `old(self)@.contents[*key]`. The
   `spec_get` transition function returns `(CacheView, Option<V>)` and the
   `Option<V>` component is never referenced in the exec ensures.
   **Fix**: Either (a) add a View impl for `CacheGuard` mapping to `V` plus a
   spec on `Deref`, or (b) add an ensures clause directly on `get` such as
   `result matches Some(g) ==> *g == old(self)@.contents[*key]` (if Verus can
   express this through the external-body guard).

2. **Guard mutation effect** — If a caller mutates through `DerefMut`, the
   cache entry should reflect the change. Currently unspecified. This is a
   known Verus limitation (`&mut` in struct fields) but should be documented
   in trust.md.

3. **Counter overflow** — `self.counter` is `u64`, incremented on every `get`
   hit and `put`. The spec has no precondition guarding overflow. While 2⁶⁴
   operations is physically unreachable, a rigorous spec would either add
   `requires self.counter < u64::MAX` or document the trust assumption. The
   property analysis (BUG-1) identifies this; it should be recorded in
   trust.md.

## Specs to Remove
- **`new`**: Remove `result@.inv()`, `result@.contents == Map::empty()`,
  `result@.capacity == capacity as nat`, `result@.lru_order == Seq::empty()`.
  All derivable from `result@ == spec_new(…)` (open spec).
- **`get` (hit)**: Remove `self@.contents == old(self)@.contents`,
  `self@.capacity == old(self)@.capacity`, `self@.inv()`.
  All derivable from `self@ == old(self)@.spec_get(*key).0`.
- **`put`**: Remove `self@.capacity == old(self)@.capacity`, put-get round-trip
  block, zero-capacity no-op block. All derivable from `self@ == spec_put(…)`.
- **`remove`**: Remove `self@.capacity == old(self)@.capacity`,
  `!self@.contents.dom().contains(*key)`,
  `!old(self)@.contents.dom().contains(*key) ==> self@ == old(self)@`.
  All derivable from `self@ == spec_remove(…)`.
- **`clear`**: Remove `self@.contents == Map::empty()`,
  `self@.lru_order == Seq::empty()`, `self@.capacity == old(self)@.capacity`.
  All derivable from `self@ == spec_clear()`.

**Caveat**: Keeping `self@.inv()` on each function is a reasonable ergonomic
choice — it saves callers from invoking invariant-preservation lemmas. The
field-level unfoldings, however, add pure maintenance burden.

## Issues (highest priority first)

1. **🔴 CRITICAL — `get` return value unspecified.** The ensures says
   `result is Some` on hit but never constrains what the `CacheGuard` provides.
   A conforming implementation could return any `V`. This defeats the fundamental
   purpose of the cache. The `spec_get` spec function computes the correct
   `Option<V>` but it is never connected to the exec return value.

2. **🔴 CRITICAL — `CacheGuard::Deref`/`DerefMut` have no specs.** These are
   the only way callers access values from `get`. Without contracts, the
   specification chain from `put` → `get` → `*guard` is broken.

3. **🟡 MODERATE — Pervasive subsumed clauses.** Every function carries 3–5
   ensures that merely unfold the canonical `== spec_*(…)` clause. This bloats
   the spec, increases maintenance burden, and risks divergence if spec functions
   are updated. Prune to `== spec_*(…)` + `inv()`.

4. **🟡 MODERATE — Counter overflow unaddressed.** `u64` counter incremented
   without bounds checking. The property analysis identifies this (BUG-1) but
   it is not recorded in bugs.md or trust.md as a verification assumption.

5. **🟢 MINOR — `inv()` redundant conjunct.** `lru_order.len() == contents.dom().len()`
   follows from `no_duplicates()` + `to_set() == dom()`. Acceptable as an SMT
   trigger / solver hint.

6. **🟢 MINOR — No bugs.md file.** The property analysis identifies BUG-1
   (counter overflow) but no bugs.md exists to track it.

## Result: FAIL

**Reason**: Two checklist items are unchecked:
1. Subsumed ensures throughout all functions.
2. `get` hit path does not meaningfully constrain the return value, and
   `CacheGuard::deref`/`deref_mut` lack specs entirely — callers cannot
   use the specification to verify correct data retrieval.

Both CRITICAL issues (items 1–2) must be resolved before the specification
can pass review.
