# Independent Review: cache (gpt-5.3-codex)

## 1. Cheating Detection
`make verify-cache` result:
- Verus verification exit code: **0** (`cached (no recompilation)`)
- Make target exit: **non-zero** due cheating detector (`status: CHEATING_DETECTED`)
- Detector summary: `assume=0 external_body=8 admit=0 trusted=0 no_decreases=0 cfg_gate=0`

Manual grep under `src/libs/cache/src/*.rs`:
- `admit(`: **0**
- `assume(`: **0**
- `trusted`: **0**
- `external_body`: **8 real annotations** (extra grep hits were comments)
- `assume_specification`: **5 declarations** (extra grep hits were comments)
- `no_decreases` / `exec_allows_no_decreases_clause`: **0**
- `cfg(...)`: present only for ghost includes (`verus_keep_ghost`) and tests; **no forbidden cfg-gated exec logic**

Cross-check vs `fix_report.md` counts: **match** (8 external_body, 5 assume_specification, others 0).

## 2. Trust Item Challenge

### External type/spec trust
- **ExBTreeMap / ExGlobal / ExCacheEntry**: classification as EXTERNAL_TYPE is correct.
  - Could eliminate? Not realistically, unless replacing std/alloc BTreeMap or exposing private internals (not acceptable).
- **ExCacheGuard** (`&'a mut V` field): VERUS_LIMITATION classification is correct.
  - Evidence from Verus source: `unsupported_err!(..., "&mut types, except in special cases")` and `"mut parameters of &mut types"`.
  - Could eliminate? Not with current Verus support.

### `find_lru_victim`
- Claim challenged: “manual loop could avoid iterator chain.”
- Verdict: **not eliminable in this no_std setup without adding broad new trust**.
  - `vstd::std_specs::btree` is cfg-gated behind `all(feature="alloc", feature="std")`.
  - Manual loop still needs iterator specs (`iter`/`next`) unavailable to this crate unless copying substantial vstd assumptions.
  - `min_by_key` itself has no vstd spec.

### `Cache::get` / `Cache::put`
- **Cache::get**: classification as VERUS_LIMITATION is correct; returning `Option<CacheGuard<'_, V>>` fundamentally depends on `&mut` flow.
- **Cache::put**: **partially challengeable**.
  - A `remove`+`insert` rewrite could likely avoid `get_mut` and be verifiable with current wrappers.
  - This was rejected on “source integrity” grounds.
  - Audit judgment: if objective is strict trust minimization, this is a **potentially eliminable** trust item; if objective is byte-level exec fidelity, keeping it external is defensible.

### `btreemap_remove`
- vstd check:
  - `std_specs::btree` does include `remove::<Q>` specs (for `std::collections::BTreeMap`).
  - local no_std adaptation does not include remove/get/get_mut specs.
- Classification as STDLIB_WRAPPER is reasonable.
- Could eliminate wrapper? Possibly only by importing/copying more upstream remove machinery (`borrowed_key_removed`, etc.), which increases trust surface; net trust reduction is unclear.

### `axiom_cache_lru_of_remove`
- Classification as VERUS_LIMITATION is mostly correct given `cache_lru_of_nonempty` is uninterpreted.
- Could be proven from basic axioms today? **No**, not from current abstractions.
- This remains the most delicate trust point: correctness relies on a semantic argument (remove preserves remaining `last_used` ordering), not a mechanized derivation.

### assume_specification (5 items)
- Presence and classification (EXTERNAL_BOTTOM) are correct.
- Additional concern: local specs intentionally drop upstream `obeys_cmp_spec`/`key_obeys_cmp_spec` guards for some cases, creating stronger assumptions than upstream vstd.

## 3. AST Consistency
AST run (using project-local output dir due environment restriction on `/tmp`):
- Matched: **15**
- Mismatched: **3** (`Cache::new`, `Cache::remove`, `Cache::evict`)
- Missing: **0**
- Extra: **2** (`find_lru_victim`, `btreemap_remove`)

Mismatch analysis:
- `Cache::new`: proof plumbing (`let result`, proof block) only; semantic-preserving.
- `Cache::remove`: direct remove replaced by wrapper + proof; semantically equivalent stdlib call.
- `Cache::evict`: victim-selection chain extracted + remove wrapper + proof.

Justifications are generally sound; no hidden behavior change found beyond declared Verus rewrites.

## 4. Verification
`make verify-cache` confirms:
- Verus crate verification succeeds (exit code 0).
- Overall make target returns non-zero only because cheating/trust policy flags remaining items.

## 5. Bug vs Limitation Analysis
Per surviving `external_body`:
- **btreemap_remove**: body is a single `m.remove(k)`; spec/body alignment looks sound; limitation/wrapper, not bug.
- **CacheGuard::deref**: direct field return; consistent with spec; limitation cascade from external guard type.
- **Cache::get**: behavior matches spec shape, but relies on unchecked `u64` counter increment.
- **Cache::put**: behavior matches spec shape, same counter overflow risk.
- **find_lru_victim**: body appears correct for min-by-counter selection; unverifiable due iterator/spec gaps.
- **axiom_cache_lru_of_remove**: no body to inspect; trust axiom, not proved.

BUG-1 (counter overflow):
- Classification as “UNCONFIRMED/LOW” is too soft.
- This is a **real latent bug** (wraparound can break LRU order), with very low practical likelihood.
- Better classification: **Confirmed, low operational risk, high-time-horizon correctness risk**.

## 6. Conclusion
- Cheating counts are accurate and cross-check clean.
- Most trust items appear genuine Verus/no_std limitations.
- **One item likely could have been reduced**: `Cache::put` externalization (via a verifiable rewrite using existing wrapper strategy), but this conflicts with strict source-integrity policy.

**Overall verdict: CONDITIONAL FAIL for strict trust minimization.**
If source-integrity preservation is the top priority, verdict can be downgraded to PASS-with-justified-trust.
