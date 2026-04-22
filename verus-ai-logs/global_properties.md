# Global Properties — Nanvix Verification

> Cross-module invariants discovered during formal verification of individual modules.
> Each property is referenced by its ID and linked to the module analysis that identified it.

---

## GLOBAL-1: Bounded Resource Guarantee (`cache`)

**Source:** `libs-cache/property_analysis.md`

**Statement:** The `cache` crate provides a bounded key-value store whose entry count never exceeds the capacity configured at construction. Any system component using this cache can rely on bounded memory usage proportional to capacity.

**Modules involved:** `cache` (provider), any future consumer crate

---

## GLOBAL-2: No-Panic Guarantee (`cache`)

**Source:** `libs-cache/property_analysis.md`

**Statement:** All public methods of `Cache<K, V>` are panic-free under the well-formedness invariant (`inv()` holds) and the counter-no-overflow precondition (`counter < u64::MAX`). In a microkernel context, library panics can crash the system; this guarantee is load-bearing.

**Modules involved:** `cache`

---

## GLOBAL-3: Guard Borrow Safety (`cache`)

**Source:** `libs-cache/property_analysis.md`

**Statement:** `CacheGuard` borrows the cache exclusively. Mutations through the guard affect only the value at the guarded key; structural metadata (`lru_order`, `capacity`) is unaffected. Exclusivity is enforced by Rust's borrow checker.

**Modules involved:** `cache`

---

## GLOBAL-4: Clear/Reset Enables Recovery (`cache`)

**Source:** `libs-cache/property_analysis.md`

**Statement:** The cache's `clear` method produces a state abstractly equivalent to a freshly constructed cache with the same capacity. This supports system-level recovery and reinitialization workflows.

**Modules involved:** `cache`
