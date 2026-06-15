## Turn 2: Verification of fixes (#5 cfg-gate, #6/#9 external_body)

### Progress
- Done: ALL 12 checklist items PASS or FIXED (see below).
- Current: confirming the two FAILs from Turn 1 are resolved.
- Remaining: none.

### Fixes applied
1. `src/kernel/src/hal/mem/types/address/phys.spec.rs` — removed the redundant
   `#[cfg(verus_keep_ghost)]` on the line preceding `verus! {` (the file is already
   `include!`d only under that cfg from `phys.rs:9`). Now opens with a bare `verus! {`,
   matching `phys.proof.rs` and `mm/phys/mod.spec.rs`.
2. `verus-ai-logs/tcb-allowed.md` — added an "Allowed `external_type_specification`"
   entry for `ExFrameNumber` (foreign `arch::FrameNumber` opaque registration, mirroring
   `ExLinkedList`) and an "Allowed `assume_specification`" section enumerating the six
   `phys.spec.rs` library-edge boundaries (`FRAME_SIZE`, `FRAME_SHIFT`,
   `VirtualAddress::new`, `<VirtualAddress as Address>::into_raw_value`,
   `FrameNumber::into_raw_value`, `FrameNumber::from_raw_value`).

### Verification (tool output)

**#5 cfg-gated exec — FIXED**
- `make verify-kernel`: `cfg_gate=8` (was 9; the phys redundant gate is gone).
- Per-module classification: phys files now contribute **0** counted cfg-gates — only
  `include!`/`use` gates remain, all ALLOWED.

**#6 / #9 external_body — FIXED**
- `detect_cheating(phys.spec.rs)` → `eb 1  eb_fn 0` (was `eb_fn 1`). `ExFrameNumber` is now
  correctly recognized as a type-spec, not a blocking external_body.
- `cheating-detail.txt`: the only phys entry is
  `hal/mem/types/address/phys.spec.rs:40 ExFrameNumber (struct): external_type_spec` —
  no blocking external_body from the phys module.
- `ExFrameNumber` + the six `assume_specification`s are now listed in `tcb-allowed.md`.

**Regression — all green**
- `make verify-kernel`: **37 verified, 0 errors, exit 0**;
  `assume=0 admit=0 trusted=0 no_decreases=0`.
- `./z build -- all`: `[OK] Build complete.` — 0 errors, 0 code warnings.
- `spec_drift.py` (vs cheating-elimination start `27abdb10`): **0 contract drift**.
- `ast_consistency.py count`: unchanged — still only the 2 pre-approved
  `f(complex_expr)` rewrites (exec code untouched by the fix).

### Final checklist status
| Item | Status |
|------|--------|
| Zero `admit()` | PASS (admit=0) |
| Zero `assume()` | PASS (assume=0) |
| Zero trusted functions | PASS (trusted=0) |
| Zero `exec_allows_no_decreases_clause` | PASS (no_decreases=0) |
| Zero cfg-gated exec code | **FIXED** (phys cfg_gate=0) |
| Zero external_body unless listed | **FIXED** (eb_fn=0; ExFrameNumber=type_spec, listed) |
| AST consistency: zero mismatches | PASS (2 pre-approved equivalent rewrites, commented) |
| All exec rewrites commented | PASS (VERUS DEVIATION comments) |
| Each surviving external_body listed | **FIXED** (ExFrameNumber in tcb-allowed.md) |
| No specs weakened | PASS (0 drift) |
| Cross-module regression | PASS (37 verified, 0 errors) |
| Verification + build, 0 err/0 warn | PASS (verify exit 0; build OK) |

All items PASS/FIXED with tool evidence. → RESOLVED.
