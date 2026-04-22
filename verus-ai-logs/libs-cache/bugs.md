# Bugs — libs/cache

## BUG-1: Counter overflow (u64)

- **Status:** UNCONFIRMED
- **Severity:** LOW (physically unreachable)
- **Location:** `lib.rs` — `Cache::get` (line ~192) and `Cache::put` (line ~224)
- **Description:** The `counter: u64` field is incremented on every `get` hit and
  `put` call without overflow checking (`self.counter += 1`). After 2^64
  operations the counter wraps to 0, which would corrupt LRU ordering —
  freshly accessed entries would have a lower counter than old ones, causing
  incorrect eviction.
- **Impact:** At 10 billion ops/sec, overflow requires ~58 years of continuous
  operation. Physically unreachable in practice, but the spec does not model
  this assumption.
- **Spec gap:** No `requires self.counter < u64::MAX` precondition exists.
  The spec transition functions use abstract `Seq` ordering (not counters),
  so the spec is correct even if the implementation overflows — but the
  `external_body` trust gap means the implementation's correctness depends
  on no overflow occurring.
- **Recommendation:** Document as a trust assumption. Optionally add
  `debug_assert!(self.counter < u64::MAX)` or use `checked_add`.
