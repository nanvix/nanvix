## Turn 2: VERUS REWRITE comment for `MemoryRegion::start` (final two FAIL items)

### Progress
- Done (PASS/FIXED):
  - Zero `admit()` — global `admit=0`; none in region files.
  - Zero `assume()` — global `assume=0`; none in region files.
  - Zero trusted functions — global `trusted=0`.
  - Zero `exec_allows_no_decreases_clause` — global `no_decreases=0`.
  - Zero cfg-gated exec code — only two `#[cfg(verus_keep_ghost)]` lines in region.rs, both gating `include!` of ghost spec/proof (excluded by detector).
  - Zero `external_body` in region — none in any region file; all 25 global ones listed in `tcb-allowed.md`.
  - Per-`external_body` listing — N/A for region; global set governed by tcb-allowed.md.
  - **AST consistency — FIXED** (see below).
  - **All exec rewrites have VERUS REWRITE comment + reproducer — FIXED** (see below).
  - No specs weakened — `spec_drift.py git-diff` re-run: 0 contract drift (ensures removed 0, requires added 0).
  - Cross-module regression — `make verify-kernel`: cached green, exit 0.
  - Verification + build — `make verify-kernel` 41 verified / 0 errors / 0 warnings; fixer's `./z build -- all` succeeded.
- Current: verifying the two coupled FAIL items from turn 1.
- Remaining: none.

### Verification
The fixer added the `VERUS REWRITE` comment to `MemoryRegion::start` (region.rs:220–233) while keeping the verifying `self.start` body. I verified independently:

1. `make verify-kernel` → **cached (no recompilation), exit 0**; `verification: 41 verified, 0 errors`; `assume=0 external_body=25 admit=0 trusted=0 no_decreases=0 cfg_gate=7` (all external_body/cfg out-of-scope, TCB-allowed). No warnings.

2. `ast_consistency.py ... summary` → `MemoryRegion::start  MISMATCH` (matched=27, mismatched=1, missing=0, extra=0). This single mismatch is the **expected, intended** semantically-equivalent rewrite. It is now properly documented:
   - `grep "VERUS REWRITE" region.rs` → present at line 221.
   - Read of region.rs:220–233 confirms the comment (a) is tagged `VERUS REWRITE`, (b) states the original `self.start.clone()` body, (c) names the Verus limitation (`Clone::clone` on generic `T: Address` has no spec tying `spec_addr(&result)` to `spec_addr(&self.start)`), (d) embeds the minimal reproducer (`postcondition not satisfied … 40 verified, 1 errors`), and (e) gives the `Address: Copy` semantic-equivalence justification.
   - I independently proved this in turn 1: restoring `.clone()` yields `40 verified, 1 errors`; `self.start` yields `41 verified, 0 errors`. So the rewrite is genuinely forced by a verified Verus limitation and is semantically equivalent (clone of a `Copy` type is a bitwise copy).

3. `spec_drift.py git-diff src/.../region.rs --before 3b7e25cc23 --after HEAD` → **0 contract drift** (no ensures removed, no requires added). The only change since turn 1 is a comment.

4. `git diff src/.../region.rs` is empty (the comment change is committed); working tree matches the verified state.

Both turn-1 FAIL items (AST consistency; VERUS REWRITE comment + reproducer) are now satisfied. The lone AST mismatch is an acceptable, documented, semantically-equivalent rewrite for a verified Verus limitation — exactly what the checklist permits.

### Fix Request
None. All twelve checklist items are PASS or FIXED with tool-backed evidence. Marking the cheating-elimination phase RESOLVED.
