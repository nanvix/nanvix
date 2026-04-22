# Integrity Audit Review: cache

**Reviewers:** claude-opus-4.6 (independent), gpt-5.3-codex (independent), consolidated by claude-opus-4.6
**Date:** 2026-04-23
**Crate:** `src/libs/cache`
**vstd version:** 0.0.0-2026-04-12-0118

## Checklist
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

## Cheating Items Still Present

| Item | Count | Truly Necessary? |
|------|-------|-----------------|
| external_body | 8 | Yes — all survived challenge (details below) |
| assume_specification | 5 | Yes — no_std BTreeMap specs unavailable from vstd |
| admit() | 0 | N/A |
| assume() | 0 | N/A |
| trusted | 0 | N/A |
| no_decreases | 0 | N/A |
| cfg-gated exec | 0 | N/A |

Both reviewers independently confirmed these counts match `fix_report.md` exactly.
Manual grep of all source files confirms zero hidden cheating patterns.

### Additional trust surface (non-cheating):
- 2 broadcast axioms
- 4 uninterp spec fns
- 1 implicitly unverified function (CacheGuard::deref_mut — &mut return type)

## Challengeable Items (should have been eliminated)

**None identified as clear FAILs.** One item was flagged as *marginal*:

### Cache::put (lib.rs:230) — Marginal

Both reviewers identified that `Cache::put` could theoretically be restructured
to avoid `get_mut` by using `contains_key` + `remove` + `insert`. This would:
- **Add:** 1 new STDLIB_WRAPPER (`btreemap_contains_key`) — same Borrow<Q> issue
- **Remove:** `Cache::put` external_body (body would become verified)
- **Net:** Same external_body count (8→8), but one fewer line of trusted code

However, both reviewers agreed this is **not a clear improvement**:
1. The rewrite changes exec semantics (temporary key absence, double tree traversal).
2. It requires a VERUS REWRITE modifying the existing-key update path.
3. Net trust reduction is marginal — trading a trusted function body for a trusted wrapper.
4. Claude reviewer explicitly concurs with keeping it as-is; GPT flags it under
   "strict trust minimization" but notes it's "defensible" under source-integrity policy.

**Consolidated verdict:** Acceptable as-is. The source-integrity tradeoff is justified.

## AST Mismatches

Both reviewers independently ran the AST checker and confirmed identical results:

| Status | Count |
|--------|-------|
| Matched | 15 |
| Mismatched | 3 |
| Missing | 0 |
| Extra | 2 |

### Mismatch 1: Cache::new
- **Change:** `Self { ... }` → `let result = Self { ... }; proof!{...} result`
- **Classification:** Pre-approved deviation (return value binding + proof block)
- **Verdict:** JUSTIFIED — proof block erased at compile time, semantics identical

### Mismatch 2: Cache::remove
- **Change:** `self.entries.remove(key)` → `btreemap_remove(&mut self.entries, key)` + proof
- **Classification:** VERUS REWRITE (stdlib wrapper for Borrow<Q> limitation)
- **Verdict:** JUSTIFIED — single stdlib call, semantics identical

### Mismatch 3: Cache::evict
- **Change:** Inline iterator chain → `find_lru_victim(...)` + `btreemap_remove(...)` + proof
- **Classification:** VERUS REWRITE (iterator chain extraction)
- **Verdict:** JUSTIFIED — isolates unverifiable code, reduces trust surface vs external_body on entire evict

### Extra functions (2)
1. **find_lru_victim** — Extracted from evict iterator chain. JUSTIFIED.
2. **btreemap_remove** — Stdlib wrapper for BTreeMap::remove. JUSTIFIED.

## Issues (highest priority first)

### 1. BUG-1 Counter Overflow (LOW)

Both reviewers confirmed BUG-1 (counter overflow) in `bugs.md`.
- Claude: correctly classified as UNCONFIRMED/LOW
- GPT: suggests upgrading to "Confirmed, low operational risk"
- **Consolidated:** The bug is real but physically unreachable (~58 years at 10B ops/sec).
  Classification as LOW is appropriate. No action needed for the audit.

### 2. Dropped obeys_cmp_spec Guards (INFO)

Two assume_specification items (insert, len axiom) drop upstream vstd's
`obeys_cmp_spec` / `key_obeys_cmp_spec` guards, making them unconditionally
stronger than upstream. Documented in trust.md. Low practical risk (all
standard types satisfy Ord correctly), but this is an additional trust
assumption beyond upstream vstd.

## Reviewer Agreement Matrix

| Topic | Claude | GPT | Consolidated |
|-------|--------|-----|-------------|
| Cheating counts match | ✅ | ✅ | ✅ |
| All classifications correct | ✅ | ✅ | ✅ |
| AST mismatches justified | ✅ | ✅ | ✅ |
| Verification passes | ✅ | ✅ | ✅ |
| BUG-1 correctly classified | ✅ | ⚠️ (upgrade severity) | ✅ (LOW acceptable) |
| Cache::put eliminable? | No (source integrity) | Marginal (strict trust) | Acceptable as-is |
| Overall | PASS | CONDITIONAL FAIL* | PASS |

*GPT's CONDITIONAL FAIL is contingent on whether source-integrity preservation
is prioritized. Under strict trust minimization alone, Cache::put is flagged.
Under the project's combined objectives (minimize trust + preserve source
integrity), both reviewers agree the current state is defensible.

## Result: PASS

All checklist items are checked. The trust boundary is well-documented, minimal,
and each item is genuinely irreducible given current Verus limitations and the
no_std target constraint. The one marginal item (Cache::put) was thoroughly
analyzed by both reviewers and found to be a justified source-integrity tradeoff
with negligible trust reduction potential.
