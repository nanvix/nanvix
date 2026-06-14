## Turn 1: Full checklist walk-through — cheating-elimination, hal-memory-region

### Progress
- Done (all PASS): every checklist item below.
- Current: full sweep completed in one turn — no FAIL items.
- Remaining: none.

### Verification

Target module: `src/kernel/src/hal/mem/types/region.rs` (+ `region.spec.rs`, `region.proof.rs`).
Phase-start baseline SHA: `a3ff01f85366` (from `pipeline_state.json`).

**`make verify-kernel`** → exit code 0. `note: verifying module hal::mem::types::region`,
`Finished dev profile`, no warnings. Global cheating counter line reports
`assume=0 external_body=11 admit=27 trusted=0 no_decreases=0 cfg_gate=14`, but
`cheating-detail.txt` confirms **every** flagged entry lives in other modules
(`mm/phys/*`, `mm/virt/*`, `libs/arch/*`). Grep of the detail file for
`types/region` returns nothing for admit / external_body / cfg.

Per-pattern grep over `region.rs`, `region.spec.rs`, `region.proof.rs`:
- `admit(` → 0 matches.
- `assume(` → 0 matches.
- `external_body` → 0 matches.
- `trusted` / `assume_specification` / `no_decreases` → 0 matches.
- `cfg(` → only lines 9 & 11: `#[cfg(verus_keep_ghost)] include!("region.spec.rs"|"region.proof.rs")`.
  These are spec/proof imports, an allowed cfg use (not cfg-gated exec logic).
- `external_derive` → lines 65, 143, 298: `#[verus_verify(external_derive)]` on
  struct derives. Allowed (derive handling, not `external_body`).

**AST consistency** (`ast_consistency.py --base-ref a3ff01f… region.rs summary`):
`Consistent: ✅ YES (matched=28 mismatched=0 missing=0 extra=0)`. All 28
functions + 3 structs MATCH. No exec rewrites exist, so the
VERUS-REWRITE-comment requirement is vacuously satisfied.

**Spec drift** (`spec_drift.py check hal-memory-region`, baseline `a3ff01f85366`):
exit 0. Functions with changes: 0; contract drift: 0; ensures removed: 0;
requires added: 0. No guarantees weakened.

**Cross-module regression**: `make verify-kernel` verifies `<all>` modules →
exit 0 (all verified modules pass).

**Build**: `./z build` compiled `kernel v0.16.17` →
`Finished dev profile` with 0 errors and 0 warnings.

**TCB**: `verus-ai-logs/tcb-allowed.md` lists no `region` functions — correct,
since the module has zero `external_body` requiring an allow-list entry.

### Checklist verdicts
- [PASS] Zero admit() — grep empty in region files.
- [PASS] Zero assume() — grep empty.
- [PASS] Zero trusted functions — grep empty.
- [PASS] Zero exec_allows_no_decreases_clause — `no_decreases=0`, grep empty.
- [PASS] Zero cfg-gated exec — only spec/proof `include!` imports under `verus_keep_ghost`.
- [PASS] Zero external_body (none present; nothing to list in TCB).
- [PASS] AST consistency — 28 matched, 0 mismatched.
- [PASS] All exec rewrites carry comment — N/A (zero rewrites; all MATCH).
- [PASS] Surviving external_body in TCB — N/A (none).
- [PASS] No specs weakened — spec-drift exit 0.
- [PASS] Cross-module regression — verify-kernel exit 0.
- [PASS] Verification + build — verify exit 0; kernel build 0 errors / 0 warnings.

### Fix Request
None. All checklist items PASS with tool evidence. Writing STOP = RESOLVED.
