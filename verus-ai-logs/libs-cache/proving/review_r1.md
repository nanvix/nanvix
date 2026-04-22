# Proving Review: cache

Consolidated from independent reviews by Claude Opus 4.6 and GPT-5.3-Codex.
Both reviewers agree on all findings — no conflicts.

## Checklist
### Proving
- [x] No specs weakened from specification phase
- [x] Zero remaining admit()
- [x] No new assume/assume_specification/external_body added without reproducer
- [x] No cfg-gated exec code (branches, expressions, match arms)
- [x] All trust items in trust.md classified per spec-design skill
- [x] Any claimed limitation has an **isolated** reproducer for the specific construct — not just the full failing expression. Conclusion names exact unsupported API/pattern, not a feature class
- [x] If many functions are external_body, this is a red flag — verify each claim independently
- [x] make verify passes (0 errors, 0 warnings)
- [x] Exec rewrites are minimal and semantically equivalent

## Weakened Specs
None. `git diff 1cd84e654 -- src/libs/cache/src/lib.rs src/libs/cache/src/lib.spec.rs`
produces empty output. Both files are byte-identical to the specification-phase baseline.
The proving phase touched only `lib.proof.rs`.

## Remaining Admits
None. All 5 admit() stubs from the specification phase have been replaced with genuine
proofs in `lib.proof.rs`.

## Issues (highest priority first)

### 1. All Cache methods are external_body (structural — not a blocker)
**Severity:** Informational (cannot be resolved in current Verus/vstd)

9 `external_body` attributes total (7 functions + 2 type specs). Root cause:
`alloc::collections::BTreeMap` has zero vstd coverage — no View trait, no
`assume_specification` for any method. Additionally, `CacheGuard` contains
`&'a mut V` which Verus cannot handle in struct fields. All 9 items are
documented in `trust.md` with classifications and reproducers.

Both reviewers independently verified that the escalation ladder was exhausted:
no vstd specs exist for BTreeMap, and replacing BTreeMap with a vstd-compatible
structure would be a major exec rewrite outside the scope of verification.

### 2. deref_mut excluded from verification (structural — not a blocker)
**Severity:** Informational

`CacheGuard::deref_mut` (line 101) has no `#[verus_verify]` annotation because
Verus does not support `&mut` return types. Documented in `trust.md`. Mutation
semantics through the guard are unmodeled.

### 3. Counter overflow assumption (documented — not a blocker)
**Severity:** Low (documented in bugs.md BUG-1 and trust.md)

`Cache::get` and `Cache::put` increment `self.counter: u64` without overflow
checks. At 10B ops/sec, overflow requires ~58 years. The spec uses abstract
`Seq` ordering so the spec is correct regardless, but the `external_body`
trust gap means correctness depends on no overflow occurring.

## Proof Quality Assessment

Both reviewers confirm the proofs are **genuine mathematical reasoning**, not
brute-force assertion bombing:

- **6 helper lemmas** — well-scoped, each proving one reusable property about
  sequences/sets (push preserves no-dup, filter preserves no-dup, filter→set
  equivalence, filter length reduction, subrange no-dup, drop-first set
  equivalence). All are called at least once.
- **5 main lemmas** — each covers all branches of its spec transition and proves
  all 4 invariant conjuncts (size ≤ capacity, no_duplicates, to_set == dom,
  len match). The hardest case (eviction in `spec_put`) is appropriately the
  most detailed.
- Proof strategy is consistent: invoke relevant helper, use Verus extensionality
  (`=~=`) to close gaps. No unnecessary complexity.

## Result: PASS
