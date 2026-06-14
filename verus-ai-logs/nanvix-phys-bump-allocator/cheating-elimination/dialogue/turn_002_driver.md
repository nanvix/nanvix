## Turn 2: All exec rewrites have VERUS REWRITE comment and minimal reproducer — FIXED

### Progress
- Done (PASS / FIXED, all independently verified):
  - Zero admit() — `admit=0`
  - Zero assume() — `assume=0`
  - Zero trusted — `trusted=0`
  - Zero exec_allows_no_decreases_clause — `no_decreases=0`
  - Zero cfg-gated exec code — `cfg_gate=0` (only `#[cfg(verus_keep_ghost)]` ghost
    includes and `#[cfg(test)]` test module)
  - external_body only if tcb-allowed — `external_body=2` (`alloc`, `alloc_as`),
    both in `verus-ai-logs/tcb-allowed.md`
  - Each surviving external_body confirmed in tcb-allowed — PASS
  - AST consistency — only `align_up` MISMATCH, a documented semantically-equivalent
    rewrite for a *verified* Verus limitation (allowed by the checklist rule)
  - **All exec rewrites have VERUS REWRITE comment + minimal reproducer — FIXED (this turn)**
  - No specs weakened — `spec_drift.py check bump-allocator` → 0 drift, exit 0
  - Cross-module regression — `make verify` all crates exit 0 (turn 1)
  - Verification + build — `make verify-bump-allocator` exit 0; `./z build -- all` exit 0; 0 warnings
- Current: final item resolved.
- Remaining: none.

### Verification

The turn-1 FAIL was: `align_up` rewrite carried a `VERUS DEVIATION` tag (not the
required `VERUS REWRITE`) and had no minimal reproducer.

I independently verified the fixer's changes — did not trust the writeup:

1. **Tag renamed.** `grep -n "VERUS REWRITE\|VERUS DEVIATION"
   src/libs/bump_allocator/src/lib.rs` →
   `137: // VERUS REWRITE: the original body was`. No `VERUS DEVIATION` remains.

2. **Reproducer file exists and is genuine.**
   `verus-ai-logs/nanvix-phys-bump-allocator/cheating-elimination/repro/div_ceil_no_spec.rs`
   contains the ORIGINAL body `value.div_ceil(alignment).checked_mul(alignment)`.
   I ran it through the real verifier myself:
   `PATH="/home/ruize/toolchain/verus:$PATH" verus div_ceil_no_spec.rs` →

   ```
   error: `core::num::impl&%11::div_ceil` is not supported (note: you may be
   able to add a Verus specification ... with `assume_specification`) ...
     --> div_ceil_no_spec.rs:44:5
   error: aborting due to 1 previous error
   ```

   This confirms the limitation is real and reproducible — not a prose assertion.
   The comment at `lib.rs:137-144` references this reproducer by path.

3. **No exec semantics changed by the edit.**
   `ast_consistency.py --base-ref exp ... summary` → `matched=11 mismatched=1`,
   the lone mismatch still `align_up` (the documented, allowed rewrite). No new
   mismatches introduced.

4. **Verification still clean.** `make verify-bump-allocator` →
   `Exit code: 0`, `assume=0 external_body=2 admit=0 trusted=0 no_decreases=0
   cfg_gate=0`. (The script's `CHEATING_DETECTED` status reflects only the two
   TCB-allowed `external_body` functions, which the checklist explicitly permits.)

### Fix Request

None. This was the last outstanding item, and it is FIXED with tool-verified
evidence. All checklist items are PASS/FIXED.

**Verdict: RESOLVED.**
