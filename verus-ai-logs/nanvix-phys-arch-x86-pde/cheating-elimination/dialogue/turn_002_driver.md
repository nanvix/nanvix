## Turn 2: AST consistency / exec-rewrite documentation (the only open FAIL)

### Progress
- Done:
  - PASS — Zero `admit()` (`admit=0`)
  - PASS — Zero `assume()` (`assume=0`)
  - PASS — Zero trusted functions (`trusted=0`)
  - PASS — Zero `exec_allows_no_decreases_clause` (`no_decreases=0`)
  - PASS — Zero cfg-gated exec (`cfg_gate=0`; only `#[cfg(verus_keep_ghost)] include!` of spec/proof)
  - PASS — Zero unlisted `external_body` (pde.* has **0**; the 3 crate-wide are `mod.rs:80 invlpg`, `table.rs:209 read`, `table.rs:246 write`, all in `tcb-allowed.md`)
  - PASS — Surviving `external_body` all listed in tcb file (none in pde scope)
  - PASS — No specs weakened (spec-drift exit 0; see note below)
  - PASS — Cross-module regression (`make verify-arch` exit 0)
  - PASS — Verification + build (verify exit 0; `./z build -- check-kernel` → `build-finished success:true`, 0 warnings)
  - **FIXED** — AST consistency: 0 mismatches
  - **FIXED** — All exec rewrites documented (rewrite eliminated entirely)
- Current: re-verification of the turn-1 FAIL (AST/rewrite).
- Remaining: none — all 12 checklist items PASS/FIXED.

### Verification

The fixer took the reviewer's **preferred** route — collapse `frame_address`
back to the original single-expression exec body, eliminating the mismatch
rather than merely documenting it.

1. Source body now (pde.rs:420–423) — exec-identical to `dev` baseline:
   ```rust
   pub fn frame_address(&self) -> usize {
       proof! { broadcast use lemma_frame_address; }
       self.frame.into_raw_value() << crate::mem::FRAME_SHIFT
   }
   ```
   The `proof!` block is erased for both the AST checker and `cargo build`, so the
   exec-visible body is exactly `self.frame.into_raw_value() << crate::mem::FRAME_SHIFT`.

2. Lemma `lemma_frame_address` made `broadcast` with `#![trigger (raw << FRAME_SHIFT)]`
   (pde.proof.rs:16–22). Proof body unchanged.

3. **AST consistency** (I ran it myself):
   ```
   git show dev:.../pde.rs > /tmp/pde_orig.rs
   ast_consistency.py /tmp/pde_orig.rs .../pde.rs count
   → ✅ Consistent: 23 functions, 2 structs match.   (EXIT 0)
   ```
   The turn-1 `frame_address` MISMATCH is gone.

4. **`make verify-arch`** (I ran it myself) → exit 0,
   `cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
   `cheating-detail.txt` lists only the 3 TCB-allowed out-of-scope functions.
   `grep -nE 'external_body|assume|admit|trusted'` over pde.rs/spec/proof → NONE.

5. **Spec drift** (`spec_drift.py git-diff .../pde.rs --before 39e930c91`) → **exit 0**.
   The tool prints `lemma_frame_address — ensures removed/added`, but this is a
   textual artifact of the added trigger attribute, **not** a weakening:
   - Removed: `(raw << FRAME_SHIFT) as int == raw as int * (FRAME_SIZE as int)`
   - Added:   `#![trigger (raw << FRAME_SHIFT)]` + the **same** predicate.
   The predicate is byte-for-byte identical; only a `#![trigger ...]` annotation was
   prepended, and the second clause (`% (FRAME_SIZE) == 0`) is unchanged. Adding a
   trigger to a `broadcast` proof fn is a proof-strategy change (skill table:
   "Proof body changed (same spec) ✅ OK"). No guarantee is removed or relaxed;
   exit code 0 confirms no hard drift. `lemma_frame_address` is a proof helper, not a
   top-level entry spec or caller-facing contract.

6. **Build** — `./z build -- check-kernel` → arch + kernel compile,
   `{"reason":"build-finished","success":true}`, `[OK] Build complete.`, 0 warnings.
   (Confirms the `proof!`/`broadcast` additions vanish in a normal Verus-erased build.)

### Fix Request
None. The single turn-1 FAIL (AST mismatch + undocumented rewrite in
`frame_address`) is resolved by eliminating the rewrite — exec body now matches the
`dev` baseline exactly (AST count exit 0, 0 mismatches), and verification/build/spec-drift
all pass. All 12 checklist items are PASS/FIXED.

**Verdict: RESOLVED.**
