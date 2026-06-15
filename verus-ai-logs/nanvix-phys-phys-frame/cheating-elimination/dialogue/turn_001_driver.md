## Turn 1: Full Checklist Walkthrough — phys-frame cheating-elimination

### Progress
- Done (PASS): all 12 checklist items (evidence below).
- Current: final sign-off.
- Remaining: none.

### Verification

All evidence gathered by reading the actual source and running tools myself
(not trusting any verbal claim).

Scope files:
- `src/kernel/src/mm/phys/frame.rs` (1507 lines)
- `src/kernel/src/mm/phys/frame.spec.rs` (45 lines)
- `src/kernel/src/mm/phys/frame.proof.rs` (1080 lines)

Commands run:
- `make verify-kernel MODULE=mm::phys` → exit 0.
- `make verify-kernel` (full, cross-module) → exit 0.
- `./z build -- all` → exit 0, `[OK] Build complete.`
- `python3 scripts/ast_consistency.py --base-ref <cheating-elim-start> frame.rs summary`
- `python3 scripts/spec_drift.py check phys-frame` → exit 0.

Cheating-detail (frame scope only), from
`verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`:
```
mm/phys/frame.rs:1242 instance: external_body
mm/phys/frame.rs:1269 init: external_body
mm/phys/frame.rs:1325 alloc: external_body
mm/phys/frame.rs:1355 alloc_contiguous: external_body
mm/phys/frame.rs:1403 free: external_body
mm/phys/frame.rs:1445 book: external_body
mm/phys/frame.rs:1466 alloc_range: external_body
```
No admit/assume/trusted/no_decreases entries for any frame file.

Per-item findings:

1. **Zero admit()** — PASS. `grep -nE 'admit\('` over frame.rs/spec/proof → none.
   Global gate: `admit=13` are all in `manager.proof.rs`, `mod.proof.rs`,
   `identity_map.rs` (out of scope), zero in frame files.

2. **Zero assume()** — PASS. `grep` → none in frame files. Gate `assume=0`.

3. **Zero trusted functions** — PASS. Gate `trusted=0`; none in frame files.

4. **Zero exec_allows_no_decreases_clause** — PASS. Gate `no_decreases=0`.

5. **Zero cfg-gated exec code (only imports/derives/debug_assert/logging)** —
   PASS. Read every `cfg` block in frame.rs:
   - L49/L52 `#[cfg(verus_keep_ghost)] include!("frame.spec.rs"|"frame.proof.rs")`
     → ghost include (import). Allowed.
   - 21× `#[cfg(not(verus_keep_ghost))] error!(...)` → logging. Allowed.
   - L315/L820/L1170 `#[cfg(not(verus_keep_ghost))] debug_assert_eq!(...)` →
     debug_assert. Allowed.
   No cfg gates a divergent exec body.

6. **external_body only if TCB-listed** — PASS. Exactly 7 in frame.rs, each
   checked individually against `verus-ai-logs/tcb-allowed.md`:
   - `instance` (L1242) — listed ("Allowed external_body", first entry). ✓
   - `init` (L1269) — listed ("Skip/exclude" + "Cross-module"). ✓
   - `alloc` (L1325) — listed ("Cross-module dependencies"). ✓
   - `alloc_contiguous` (L1355) — listed. ✓
   - `free` (L1403) — listed. ✓
   - `book` (L1445) — listed. ✓
   - `alloc_range` (L1466) — listed. ✓
   Each retains a full `#[verus_spec]` contract (verified by reading the
   attribute blocks). Note: `free_count`, `share`, `refcount` are NOT
   external_body in the code (they are verified wrappers calling `instance()`),
   i.e. stricter than the TCB list permits — acceptable.

7. **AST consistency: zero mismatches** — PASS. Against the cheating-elimination
   phase-start baseline `89f37ecaadb8`:
   `ast_consistency.py ... count` → `1 mismatched (18 functions match)`. The single
   mismatch is `free_count`. `diff` shows it is the pre-approved "intermediate
   value" deviation:
   `inner.bitmap.number_of_bits() - inner.bitmap.usage()` →
   `let nbits = ...; let used = ...; nbits - used` so the proof can observe
   `number_of_bits() > 0` before `lemma_free_count_eq`. Semantically identical
   (same calls, order, result). This is exactly the
   `f(complex_expr)` → `let x = complex_expr; f(x)` row in the ast-consistency
   pre-approved-deviation table — i.e. a "semantically-equivalent rewrite for a
   verified Verus limitation", which the checklist parenthetical explicitly
   permits. PASS.

8. **All exec rewrites have deviation comment + reproducer** — PASS. The one exec
   rewrite (`free_count`) carries a documenting comment:
   `// VERUS DEVIATION (pre-approved: intermediate value). ...`. The repo's
   established convention is `VERUS DEVIATION` (also used in the already-verified
   `hal/.../address/phys.rs`); `VERUS REWRITE` is used nowhere in `src/`. Per the
   ast-consistency skill, pre-approved deviations (intermediate value) require
   only a documenting comment — "no full deviation process needed" — so a minimal
   reproducer is not required for this class. Comment present and accurate. PASS.

9. **Each surviving external_body TCB-listed** — PASS. Same evidence as #6; all 7
   addressed individually, all listed.

10. **No specs weakened (spec drift)** — PASS. `spec_drift.py check phys-frame`
    (baseline `89f37ecaad8` → HEAD) → exit 0:
    `Functions with changes: 0 / Contract drift: 0 / Ensures removed: 0 /
    Requires added: 0`. No `requires`/`ensures` removed or relaxed.

11. **Cross-module regression** — PASS. `make verify-kernel` (full) → exit 0,
    `cached (no recompilation)`. All verified modules pass. The
    `CHEATING_DETECTED` status reflects out-of-scope modules' own counters
    (manager/mod/upool/identity_map admits + external_body), not a regression in
    any verified module.

12. **Verification + build: 0 errors, 0 warnings** — PASS. `make verify-kernel`
    exit 0; log has 0 `error:` and 0 `warning:` lines (10 informational
    low-confidence-trigger `note:` lines only — not warnings). `./z build -- all`
    exit 0, `[OK] Build complete.` (the lone "Sysroot directory not found" line
    is a build-script info message, not a compiler warning).

### Fix Request
None. Every checklist item is PASS with concrete tool evidence. No code changes
required. Proceeding to STOP = RESOLVED.
