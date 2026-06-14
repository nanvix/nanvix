## Turn 1: Full Checklist Walkthrough — Cheating Elimination (phys-manager)

### Progress
- Done (PASS, evidence below): ALL checklist items.
- Current: Final verification sweep — no FAIL items found.
- Remaining: none.

### Scope
- Source: `src/kernel/src/mm/phys/manager.rs`
- Spec:   `src/kernel/src/mm/phys/manager.spec.rs`
- Proof:  `src/kernel/src/mm/phys/manager.proof.rs`
- TCB list: `verus-ai-logs/tcb-allowed.md`
- Baseline (cheating-elimination phase start): `c64feda70cdd31643e996f4a552120eb33e51476`

### Verification

Commands run and results:

1. `make verify-kernel` → Exit code 0. All five `mm::phys` modules verified
   (`mm::phys`, `frame`, `kframe`, `manager`, `upool`). Cheating-pattern summary:
   `assume=0 external_body=22 admit=0 trusted=0 no_decreases=0 cfg_gate=9`
   (crate-wide counts).

2. Per-file grep over the three `manager.*` files
   (`grep -nE 'admit\(|assume\(|trusted|external_body|no_decreases'`):
   - `admit(`  → 0 (only the word in a comment in `manager.proof.rs:8`).
   - `assume(` → 0.
   - `trusted` → 0.
   - `no_decreases` → 0.
   - `external_body` → 6 (lines 98, 177, 249, 292, 336, 388).

3. AST consistency (`scripts/ast_consistency.py … summary`, auto-detected baseline):
   `Consistent: ✅ YES (matched=7 mismatched=0 missing=0 extra=0)`.
   All 7 functions (`init`, `get_mut`, `alloc_user_frame`, `check_user_watermark`,
   `alloc_many_user_frames`, `alloc_kernel_frame`, `alloc_many_kernel_frames`) and
   the `PhysMemoryManager` struct = MATCH. Exec code unchanged.

4. Spec drift (`scripts/spec_drift.py git-diff src/kernel/src/mm/phys/manager.rs
   --before c64feda70…`): Exit 0 — "✅ No contract drift detected." 0 ensures
   removed, 0 requires added, 0 functions removed.

5. Build (`./z build -- check-kernel`): Exit 0, `build-finished success:true`,
   no `compiler-message`/`warning:` entries. Re-touched `manager.rs` and re-checked
   to force recompile — still 0 warnings.

### Checklist Results

| # | Item | Verdict | Evidence |
|---|------|---------|----------|
| 1 | Zero `admit()` | PASS | grep + `admit=0` |
| 2 | Zero `assume()` | PASS | grep + `assume=0` |
| 3 | Zero trusted fns | PASS | grep + `trusted=0` |
| 4 | Zero `exec_allows_no_decreases_clause` | PASS | `no_decreases=0` |
| 5 | Zero cfg-gated exec code | PASS | Only `#[cfg(verus_keep_ghost)] include!()` (ghost spec/proof imports, lines 9,11) and `#[cfg_attr(verus_keep_ghost, allow(verus_impl_method_marker))]` (lint-allow attrs, lines 97,291). No exec-behavior gating. |
| 6 | Zero `external_body` unless in TCB list | PASS | All 6 (`init`, `alloc_many_user_frames`, `alloc_user_frame`, `check_user_watermark`, `alloc_kernel_frame`, `alloc_many_kernel_frames`) explicitly listed in `tcb-allowed.md` §"PhysMemoryManager (manager.rs)" lines 54–73. |
| 7 | AST consistency: zero mismatches | PASS | matched=7 mismatched=0 |
| 8 | All exec rewrites have VERUS REWRITE comment + reproducer | PASS (vacuous) | AST shows 0 exec changes, so there are no rewrites to document. |
| 9 | Each surviving `external_body` listed in TCB | PASS | Per-function confirmation against `tcb-allowed.md` (item 6). |
| 10 | No specs weakened | PASS | spec_drift exit 0, 0 contract drift |
| 11 | Cross-module regression (`make verify-kernel`) | PASS | All modules verified, exit 0 |
| 12 | Verification + build, 0 errors / 0 warnings | PASS | verify exit 0; check-kernel exit 0, no warnings |

### Per-`external_body` TCB confirmation (item 6 / 9, individually)

- `manager.rs:107 init` → listed (tcb-allowed.md:54). PASS.
- `manager.rs:198 alloc_many_user_frames` → listed (tcb-allowed.md:65). PASS.
- `manager.rs:267 alloc_user_frame` → listed (tcb-allowed.md:57). PASS.
- `manager.rs:306 check_user_watermark` → listed (tcb-allowed.md:61). PASS.
- `manager.rs:352 alloc_kernel_frame` → listed (tcb-allowed.md:69). PASS.
- `manager.rs:409 alloc_many_kernel_frames` → listed (tcb-allowed.md:71). PASS.

(`get_mut` is `external_body`-free in the verified sense — it is on the TCB
"Skip / exclude" list, tcb-allowed.md:37, and carries no `#[verus_verify(external_body)]`.)

### Fix Request

None. Every checklist item is PASS with concrete tool evidence. No code changes
requested. Marking the cheating-elimination review RESOLVED.
