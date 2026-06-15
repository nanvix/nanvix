## Turn 1: Full checklist sweep — arch-frame-number cheating elimination

### Progress
- Done (all PASS): every checklist item below.
- Current: final sweep complete.
- Remaining: none.

### Verification

Source files reviewed:
- `src/libs/arch/src/x86/mem/paging/frame/number.rs` (exec)
- `src/libs/arch/src/x86/mem/paging/frame/number.spec.rs` (spec/view/invariant)
- `src/libs/arch/src/x86/mem/paging/frame/number.proof.rs` (empty `verus! { }`)

Commands run and results:

1. **`make verify-arch`** → exit 0, status **CLEAN**.
   - `cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0`
   - Cheating Pattern Check: "✅ No cheating detected."

2. **Pattern grep** over `number*.rs` for
   `external_body|admit|assume|trusted|no_decreases`: no matches in any exec/spec/proof
   file. The only `cfg(verus_keep_ghost)` occurrences are lines 9 & 11 of `number.rs`,
   gating `include!("number.spec.rs")` and `include!("number.proof.rs")` — ghost
   imports, which are explicitly allowed.

3. **AST consistency** (`ast_consistency.py number.rs count`):
   `✅ Consistent: 4 functions, 1 structs match.` Zero mismatches → no exec rewrites
   exist, so no VERUS REWRITE comments are required.

4. **Spec drift** (`spec_drift.py check arch-frame-number`, baseline
   `6df2ff9a` → HEAD): "✅ No contract drift detected." 0 functions changed,
   0 ensures removed, 0 requires added.

5. **Cross-module regression** (`make verify`, all VERUS_CRATES + kernel): every
   crate verifies with **exit 0**. `arch` is CLEAN. `kernel` reports
   `external_body=23 cfg_gate=6` — these are **pre-existing, TCB-allowed** entries
   (e.g. `book_physical_memory_regions`, `book_mmio_regions`, `instance`,
   `KernelFrame::deref*`) documented in `verus-ai-logs/tcb-allowed.md`, entirely
   outside the `arch-frame-number` scope, and verification still passes (exit 0).

6. **Build** (`./z build -- all`): `[OK] Build complete.` exit 0. No compiler
   warnings (only a benign "Sysroot directory not found; skipping symlink update"
   notice — not a warning/error).

### Checklist results

- [PASS] Zero admit() — `admit=0`, no grep matches.
- [PASS] Zero assume() — `assume=0`, no grep matches.
- [PASS] Zero trusted functions — `trusted=0`.
- [PASS] Zero exec_allows_no_decreases_clause — `no_decreases=0`.
- [PASS] Zero cfg-gated exec code — only `cfg(verus_keep_ghost)` ghost `include!`
  imports of the spec/proof files; `arch` `cfg_gate=0`.
- [PASS] Zero external_body — module has none; tcb-allowed list not engaged.
- [PASS] AST consistency — 4 fns / 1 struct match, zero mismatches.
- [PASS] All exec rewrites have VERUS REWRITE comment — N/A (no rewrites; AST consistent).
- [PASS] Each surviving external_body listed in tcb-allowed — N/A (none surviving in module).
- [PASS] No specs weakened — spec-drift exit 0, zero contract drift vs phase baseline.
- [PASS] Cross-module regression — `make verify` all crates exit 0.
- [PASS] Verification + build — `make verify-arch` exit 0 CLEAN; `./z build -- all` exit 0, no warnings.

### Fix Request

None. All checklist items PASS with concrete tool evidence. No code changes required.
