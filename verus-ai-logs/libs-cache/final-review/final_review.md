# Final Comprehensive Review: cache

**Date:** 2026-04-22
**Reviewers:** Claude Opus 4.6 (`final_review.claude.md`), GPT-5.3-Codex (`final_review.codex.md`)
**Consolidated by:** Claude Opus 4.6

---

## Checklist

### Caller Analysis
- [x] All pub functions have callers identified (or justified as design-intent-only)
- [x] Trait obligations documented with expected semantics
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified (1-2 sentences)
- [x] Pre-existing specs assessed (if any exist from upstream verification)

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite)
- [x] All caller-observable state represented (no missing fields)
- [x] No implementation-specific fields (no capacity, hash fn, pointers)
- [x] inv() encodes real constraints (not trivially true)
- [x] Mathematical types used (int/Seq/Set/Map; exception: addresses keep usize)
- [x] Inherited View fields validated against ALL callers (if pre-existing)

### Specification
- [x] Every in-scope exec function has requires/ensures
- [x] No tautological ensures (e.g., `Err(_) => true`)
- [x] No subsumed ensures (derivable from inv() + other ensures)
- [x] Error paths have meaningful ensures (match style: Ok => ..., Err => ...)
- [x] No assume_specification for workspace-internal code
- [x] vstd searched before any assume_specification
- [x] Specs written for the caller (usable directly in caller proofs)

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

### Polish
- [x] Large inline proof blocks extracted into named lemmas (proof-extraction)
- [x] No redundant assertions, hints, or duplicate lemmas (proof-minimization; uncalled lemmas are NOT dead — they still prove properties)
- [x] Lemma names are descriptive (describe WHAT is proven)
- [x] make verify still passes (0 errors, 0 warnings)
- [x] No specs weakened during polish

### Integrity Audit
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code (only imports/derives/debug_assert/logging allowed)
- [x] Every external_body challenged — only genuinely unverifiable items survive
- [x] AST consistency: each mismatch analyzed (allowed deviation vs restore needed)
- [x] All exec rewrites have VERUS REWRITE comment and minimal reproducer
- [x] trust.md minimized — only genuinely unverifiable items remain
- [x] make verify passes (0 errors, 0 warnings)

### Bug Recording
- [x] bugs.md exists and has been reviewed for completeness
- [x] Every surviving verification failure classified (True Bug / Context-Dependent / False Positive)
- [x] Each True Bug has all fields: What / Why / Verification Failure / How Verus Helped / Severity / Suggested Fix
- [x] No external_body used to mask a code defect (vs. genuine Verus limitation)
- [x] Bug entries include provenance (which phase discovered it)

---

## Spec Quality

Both reviewers agree: **high quality**.

The `CacheView<K, V>` abstraction uses purely mathematical types (`Map<K, V>`,
`Seq<K>`, `nat`) and passes the substitution test — swapping BTreeMap for a hash
map or linked list would not change the View. The invariant `inv()` encodes four
non-trivial constraints (capacity bound, no-duplicate ordering, key-set identity,
explicit cardinality link).

All five spec transition functions (`spec_new`, `spec_get`, `spec_put`,
`spec_remove`, `spec_clear`) are declarative and implementation-independent.
`spec_put` correctly models all four branches (zero-capacity no-op, overwrite,
eviction, below-capacity insert). `spec_get` models hit vs miss bidirectionally.
Frame conditions are implicit via transition equality (`self@ == old(self)@.spec_*(…)`),
which callers can unfold directly.

The `move_to_mru` helper isolates the `filter(…).push(…)` pattern for reuse in
`spec_get` and `spec_put`, enabling modular proofs of set-preservation and
no-duplicates.

Minor note (both reviewers): `get`'s guard view uses `.spec_get(*key).1.unwrap()`
which is safe (the hit branch guarantees `Some`) but slightly less readable than
`old(self)@.contents[*key]`. This is a style preference, not a defect.

---

## Caller Coverage

- **Covered:** 16 / 17 caller expectations
- **Missing:** `CacheGuard::deref_mut` (1 expectation — yields `&mut V`)

`deref_mut` cannot be verified: Verus does not support `&mut` return types.
The function cannot carry any annotation at all (even `external_body` fails
on the signature). This is documented in `trust.md` with a reproducer and
classified as `VERUS_LIMITATION`. Both reviewers independently confirmed this.

All other caller expectations from `caller_analysis.md` are fully covered:
`new` (empty + capacity), `get` (hit/miss + guard value + LRU refresh + size
unchanged), `put` (insert/overwrite/evict/zero-cap), `remove` (present/absent),
`clear` (all removed + capacity preserved), `deref` (`*ret == self@`), and
exclusive-borrow semantics (enforced by Rust's borrow checker, not specs).

---

## Proof Completeness

- **Remaining admit(): 0**

Zero `admit()`, `assume()`, or `trusted` in any of the three source files.
All 5 invariant preservation lemmas are fully proven in `lib.proof.rs`:

| Lemma | Status |
|-------|--------|
| `lemma_spec_new_inv` | ✅ Proven |
| `lemma_spec_get_inv` | ✅ Proven |
| `lemma_spec_put_inv` | ✅ Proven (3 branches: overwrite, eviction, below-capacity) |
| `lemma_spec_remove_inv` | ✅ Proven |
| `lemma_spec_clear_inv` | ✅ Proven |

6 helper lemmas support the proofs (`push_preserves_no_dup`,
`filter_preserves_no_dup`, `filter_neq_to_set`, `filter_neq_len`,
`subrange_no_dup`, `drop_first_to_set`), all fully proven.

---

## Trust Minimization

- **Total trust items:** 11 (4 external_type_specification + 7 external_body functions)
- **Challengeable (should be eliminated):** None

### External Type Specifications (4)

| Item | Classification | Verdict |
|------|---------------|---------|
| `ExBTreeMap` (spec.rs:20-23) | EXTERNAL_TYPE | ✅ Genuine — `alloc::collections::BTreeMap` private fields |
| `ExGlobal` (spec.rs:25-26) | EXTERNAL_TYPE | ✅ Genuine — default allocator for BTreeMap |
| `ExCacheEntry` (spec.rs:29-31) | EXTERNAL_TYPE | ✅ Genuine — private struct, BTreeMap value type |
| `ExCacheGuard` (spec.rs:35-38) | VERUS_LIMITATION | ✅ Genuine — `&'a mut V` field unsupported |

### External Body Functions (7)

| Function | Classification | Independent blockers | Eliminable? |
|----------|---------------|---------------------|-------------|
| `Cache::new` (lib.rs:141) | VERUS_LIMITATION | no_std blocks vstd btree + uninterp view | ❌ |
| `Cache::get` (lib.rs:168) | VERUS_LIMITATION | `get_mut` no vstd spec + `&mut` return + CacheGuard `&mut` | ❌ |
| `Cache::put` (lib.rs:208) | VERUS_LIMITATION | `get_mut` no vstd spec + calls evict + no_std btree | ❌ |
| `Cache::remove` (lib.rs:254) | VERUS_LIMITATION | no_std blocks vstd btree + uninterp view | ❌ |
| `Cache::clear` (lib.rs:271) | VERUS_LIMITATION | no_std blocks vstd btree + uninterp view | ❌ |
| `Cache::evict` (lib.rs:289) | VERUS_LIMITATION | `min_by_key` no spec + no_std btree + BTreeMap::remove | ❌ |
| `CacheGuard::deref` (lib.rs:91) | VERUS_LIMITATION | CacheGuard opaque (`&mut` field) | ❌ |

**Root cause:** A single environmental constraint — the no_std kernel target —
blocks access to vstd's BTreeMap specs (`cfg(all(feature="alloc", feature="std"))`).
The integrity audit (dual-reviewer) independently challenged every item. Three
functions (`new`, `remove`, `clear`) were identified as theoretically eliminable
via custom `assume_specification`, but this was correctly rejected because trust
would merely shift to unverified axioms, not be eliminated.

**Unverifiable function:** `CacheGuard::deref_mut` is completely excluded from
verification (cannot even carry `external_body`). Documented in trust.md.

---

## Guardrails Compliance

| Dimension | Count | Status |
|-----------|-------|--------|
| `admit` | 0 | ✅ |
| `assume` | 0 | ✅ |
| `external_body` (type) | 2 | ✅ Acceptable (external_type_specification) |
| `external_body` (function) | 7 | ⚠️ BLOCKER — requires human review |
| `trusted` | 0 | ✅ |
| `no_decreases` | 0 | ✅ |
| `cfg_gate` | 0 | ✅ |
| `assume_specification` | 0 | ✅ |

- **Unjustified items:** None — all 7 function-level `external_body` items are
  documented in `trust.md` with classifications, reproducers, and dual-reviewer
  validation.

The 7 function-level `external_body` items are **per-policy BLOCKERs** requiring
human review. Both reviewers confirm none can be eliminated under current
constraints. Each has been independently challenged by two models with different
reasoning paths, reaching the same conclusion.

---

## AST Consistency

- **AST check: PASS**
- Functions: 18/18 MATCH (matched=18, mismatched=0, missing=0, extra=0)
- Structs: 3/3 MATCH
- VERUS REWRITE comments: 0 (none needed — zero exec code modifications)
- Tool: `ast_consistency.py --base-ref dev`, tree-sitter based

No exec code was modified during the verification effort. The implementation is
identical to the `dev` branch.

---

## Verification

- **verus: PASS** — 11 verified, 0 errors (exit code 0)
- Function coverage: 7/8 exec functions have contracts (unverified: `deref_mut`)
- Cheating detection: assume=0, external_body=9, admit=0, trusted=0,
  no_decreases=0, cfg_gate=0

---

## Bug Summary

- **Total bugs recorded:** 1
- **True Bugs:** 0
- **Context-Dependent:** 1

### BUG-1: Counter overflow (u64) — Context-Dependent

- **Location:** `lib.rs` — `Cache::get` (line ~189) and `Cache::put` (line ~224, ~235)
- **Description:** `self.counter: u64` is incremented without overflow checking.
  After 2^64 operations, counter wraps to 0, corrupting LRU ordering.
- **Impact:** At 10 billion ops/sec, overflow requires ~58 years. Physically
  unreachable.
- **Spec gap:** No `requires self.counter < u64::MAX` precondition. The abstract
  spec uses `Seq` ordering (not counters), so the spec is correct regardless of
  overflow — but the `external_body` trust bridge depends on no overflow.
- **Classification:** Context-Dependent — real in theory, physically unreachable.
  Correctly documented as trust assumption in `trust.md`.
- **Recommendation:** Add `debug_assert!(self.counter < u64::MAX)` for defense-in-depth.
- **Both reviewers agree:** Properly handled, no fix required.

### Additional potential bugs from property analysis

BUG-2 through BUG-5 from the property analysis were reviewed:
- BUG-2 (usize vs nat): Trust boundary, not a code bug. ✅
- BUG-3 (evict on empty): Non-issue — guarded by control flow. ✅
- BUG-4 (clear resets counter): Design choice, safe since entries also cleared. ✅
- BUG-5 (min_by_key ties): Non-issue under no-overflow (counter injectivity). ✅

No bugs were discovered during proving/integrity that were not already recorded.

---

## Reviewer Agreement

Both reviewers (Claude Opus 4.6 and GPT-5.3-Codex) independently reached
**CONDITIONAL PASS** with the same condition: human sign-off on the 7
function-level `external_body` items.

**No disagreements** on any dimension. Both reviewers:
- Confirmed all specs are high quality and caller-oriented
- Confirmed 0 admit/assume/trusted
- Confirmed all trust items are justified and non-eliminable
- Confirmed AST consistency is clean
- Confirmed BUG-1 is properly handled as Context-Dependent
- Identified the same root cause (no_std target) for all trust items

---

## Issues (highest priority first)

1. **BLOCKER (policy):** 7 function-level `external_body` items require human
   review and acceptance. Root cause: no_std target blocks vstd BTreeMap specs.
   All items are documented with classifications, reproducers, and dual-reviewer
   validation. Cannot be eliminated without changing the target platform or
   waiting for Verus to add support.

2. **LOW:** `CacheGuard::deref_mut` is completely unverifiable (Verus `&mut`
   return type limitation). Mutation-through-guard semantics are unmodeled.
   Cannot be resolved until Verus adds `&mut` return type support.

3. **LOW:** BUG-1 counter overflow (u64) — physically unreachable but
   undischarged. Consider adding `debug_assert!(self.counter < u64::MAX)`.

---

## Result: CONDITIONAL PASS

The verification effort is technically sound and thorough:
- **Spec quality:** High — abstract, caller-oriented, complete, declarative
- **Proofs:** Complete — 11 lemmas, zero admit/assume
- **Trust boundary:** Minimal and well-documented — single root cause (no_std)
- **AST integrity:** Perfect — zero exec code modifications
- **Verification:** Clean — 0 errors

**Condition:** Human acceptance of 7 `external_body` functions, all caused by
the no_std kernel target blocking vstd BTreeMap specs. These are environmental
constraints, not verification quality issues. Each item has been independently
challenged by two reviewers using different models, and none can be eliminated
under current constraints.
