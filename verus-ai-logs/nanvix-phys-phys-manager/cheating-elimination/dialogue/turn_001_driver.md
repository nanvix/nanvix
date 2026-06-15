## Turn 1: Full checklist walkthrough — phys-manager cheating-elimination

### Progress
- Done (PASS): all 12 checklist items verified PASS with tool evidence (below).
- Current: none — review complete.
- Remaining: none.

### Verification

Scope: `src/kernel/src/mm/phys/manager.rs` + included `manager.spec.rs` /
`manager.proof.rs`. Commands run from repo root.

**Whole-suite verification — `make verify-kernel`** → Exit code 0.
Cheating scan (global): `assume=0 external_body=18 admit=3 trusted=0
no_decreases=0 cfg_gate=9`. The non-zero items are resolved per-module below
using `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`.

1. **Zero admit()** — PASS.
   `grep "admit(" manager.{rs,spec.rs,proof.rs}` → only a prose mention in a
   `manager.proof.rs:17` comment; no `admit()` call site. `cheating-detail.txt`
   shows the 3 global admits are all in `mm/virt/identity_map.rs`
   (533/627/718) — **out of manager scope**.

2. **Zero assume()** — PASS. `grep "assume("` → none. `assume=0` globally.

3. **Zero trusted functions** — PASS. `trusted=0`; no `#[verifier::external]`
   or trusted markers in the three files.

4. **Zero exec_allows_no_decreases_clause** — PASS. `no_decreases=0`; grep finds
   none in scope.

5. **Zero cfg-gated exec (only imports/derives/debug_assert/logging allowed)**
   — PASS. The only `#[cfg(not(verus_keep_ghost))]` sites (manager.rs lines
   207,213,347,353,390,393,460,466,508) each gate an `error!`/`warn!` **logging**
   macro. The `#[cfg(verus_keep_ghost)]` sites (lines 8,10) gate `include!` of the
   spec/proof files (**imports**). The 3 prior `#[cfg_attr(verus_keep_ghost,
   verus_spec(invariant …))]` loop wrappers were rewritten to direct
   `#[verus_spec(invariant …)]` this phase (legal in both builds:
   `proc_macro_hygiene` is enabled unconditionally at `kmain.rs:20`).

6. **Zero external_body unless tcb-allowed** — PASS. The 6 manager external_body
   functions (cheating-detail.txt 8–13) are each enumerated in
   `verus-ai-logs/tcb-allowed.md`:
   - `manager.rs::init` (tcb-allowed line 129)
   - `manager.rs::kernel_watermark` (line 188)
   - `manager.proof.rs::lemma_manager_attached` (line 211)
   - `manager.proof.rs::lemma_kernel_alloc_one` (line 214)
   - `manager.proof.rs::lemma_kernel_alloc_contiguous` (line 219)
   - `manager.proof.rs::lemma_user_bulk_err_restored` (line 222)
   No unlisted external_body in scope. (The 3 `assume_specification`s —
   `Result::and_then`, `Result::inspect_err`, `Vec::capacity` — are std/alloc
   external-top specs, not flagged by the gate; `assume=0`.)

7. **AST consistency: zero mismatches** — PASS.
   `ast_consistency.py --base-ref verus-ai-prove-bottom-up manager.rs count`
   → "✅ Consistent: 8 functions, 1 structs match." Exec is byte-identical
   (after ghost stripping) to the phase baseline.

8. **All exec rewrites documented (VERUS comment + reproducer)** — PASS.
   `git diff <cheating-elimination start c60b1dd> HEAD` over the three files
   shows the phase touched **only** ghost annotations (three
   `cfg_attr(verus_keep_ghost, verus_spec(…))` → `verus_spec(…)`); **zero exec
   change**. The single verus-required exec deviation present
   (`check_user_watermark`: hoisting `frame::free_count()` + `kernel_watermark()`
   accessor) carries an explanatory `VERUS DEVIATION` comment (manager.rs:336).

9. **Each surviving external_body listed** — PASS (same evidence as item 6).

10. **No specs weakened (spec-drift)** — PASS.
    `spec_drift.py git-diff <file> --before c60b1dd` for manager.rs /
    manager.spec.rs / manager.proof.rs → all "✅ No contract drift detected",
    exit 0 (ensures removed: 0, requires added: 0, proof drift: 0).

11. **Cross-module regression** — PASS. `make verify-kernel` (full kernel) →
    Exit code 0; all modules pass (cached, committed state already verified).

12. **Verification + build (0 errors, 0 warnings)** — PASS.
    - `make verify-kernel` → Exit code 0; manager external_body all allowed,
      0 admit/assume/trusted/no_decreases in scope.
    - `make all-kernel` (non-verus build) → `Finished dev profile … in 10.82s`,
      Exit 0, no warnings — confirms the cfg_attr→verus_spec rewrite compiles in
      the normal build.

### Fix Request
None. All 12 checklist items PASS with concrete tool evidence. No code changes
required.
