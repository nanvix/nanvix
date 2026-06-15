## Turn 1: Full cheating-elimination checklist walkthrough — `mm::virt::identity_map`

### Progress
- Done (all PASS): every checklist item below.
- Current: full sweep of the entire checklist (first turn).
- Remaining: none.

### Verification

Commands run and evidence:

1. **`make verify-kernel`** → exit 0. Cheating summary (whole kernel crate):
   `assume=0 external_body=29 admit=0 trusted=0 no_decreases=0 cfg_gate=6`.
   Detail: `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`.

2. **Per-marker grep** on the three module files
   (`identity_map.rs`, `.spec.rs`, `.proof.rs`):
   - `admit(` → 0 in code (only a stale prose mention in `proof.rs:16`).
   - `assume(` / `assume_specification` → 0.
   - `verifier::trusted` / no_decreases → 0.

3. **Module-scoped cfg-gate count** (reusing `verify.sh::count_cfg_gates`
   logic on the three files) → **0**. The only two `#[cfg(verus_keep_ghost)]`
   gates (`identity_map.rs:24,26`) target `include!(...)`, which the counter
   excludes (imports/include allowed).

4. **external_body inventory** (from `cheating-detail.txt`):
   - `identity_map.rs:521 ensure_pt` — external_body
   - `identity_map.rs:610 ensure_pte` — external_body
   - `identity_map.rs:698 identity_map_page` — external_body
   - `.spec.rs:48/52/56/66` ExTableIndex / ExPageDirectoryEntry /
     ExPageTableEntry / ExTable — classified `external_type_spec` (opaque
     foreign-type registration, not exec bodies).
   Cross-checked `verus-ai-logs/tcb-allowed.md` lines 182–217 ("Allowed
   `external_body` — `mm::virt::identity_map`"): all three exec shims are
   listed individually with rationale (global statics, raw page-table memory,
   interior-mutable allocator token, arch newtype/asm below the boundary).

5. **AST consistency** (`scripts/ast_consistency.py`):
   - vs phase-start `0e63adac6`: ✅ Consistent (14 fns, 1 struct).
   - vs base branch `verus-ai/arch-frame-number` (`1bba3cac8`): ✅ all 14
     functions + `Cr3Guard` struct **MATCH** (matched=14 mismatched=0).
     Exec code is byte-for-byte equivalent to the original (modulo annotations).

6. **Exec rewrites**: grep `VERUS REWRITE` / `REWRITE` → none. Consistent
   with the AST result (no exec body changed), so the "rewrite comment +
   reproducer" requirement is vacuously satisfied.

7. **Spec drift** (`scripts/spec_drift.py check virt-identity-map`,
   baseline `0e63adac6` → HEAD): exit 0, **0 contract drift** (ensures
   removed=0, requires added=0, fns removed=0). No guarantee weakened.

8. **Cross-module regression**: `make verify-kernel` verifies `kernel::all`
   → exit 0, all modules pass (cached, no recompilation).

9. **Build**: kernel cargo build with project flags
   (`--features microvm,trace -Z build-std=...`) → exit 0; forced recompile
   of `identity_map.rs` produced **0 warnings, 0 errors**.

### Checklist verdicts

| # | Item | Verdict | Evidence |
|---|------|---------|----------|
| 1 | Zero `admit()` | **PASS** | `admit=0`; grep clean |
| 2 | Zero `assume()` | **PASS** | `assume=0`; grep clean |
| 3 | Zero trusted fns | **PASS** | `trusted=0` |
| 4 | Zero exec_allows_no_decreases | **PASS** | `no_decreases=0` |
| 5 | Zero cfg-gated exec code | **PASS** | module cfg_gate=0 (both gates are `include!`) |
| 6 | Zero unlisted external_body | **PASS** | ensure_pt / ensure_pte / identity_map_page all in `tcb-allowed.md` §182–217 |
| 7 | AST consistency: 0 mismatches | **PASS** | base-branch MATCH 14 fns + 1 struct; phase-start consistent |
| 8 | Exec rewrites documented | **PASS (vacuous)** | no exec rewrites exist (AST identical) |
| 9 | Each surviving external_body listed | **PASS** | all 3 enumerated in TCB allowlist |
| 10 | No specs weakened | **PASS** | spec_drift exit 0, 0 drift |
| 11 | Cross-module regression | **PASS** | `make verify-kernel` exit 0 (all modules) |
| 12 | Verification + build, 0 err/0 warn | **PASS** | verify exit 0; build 0 warnings |

### Minor observations (non-blocking, no fix demanded)

- `identity_map.proof.rs:16–17` carries a stale comment: "Bodies are left as
  `admit()` during the specification phase; the proving phase discharges
  them." The four lemmas now have real proof bodies (no `admit()`). This is
  a documentation staleness only — it does not affect verification, the
  cheating scan, or any checklist item, so it is **not** a FAIL. Optional
  cleanup for a future doc pass.
- The four `external_type_specification` registrations
  (ExTableIndex/ExPageDirectoryEntry/ExPageTableEntry/ExTable) use
  `#[verifier::external_body]` as part of the opaque foreign-type idiom. The
  cheating scanner classifies them as `external_type_spec` (not exec
  `external_body`), and they match the explicitly approved
  `ExLinkedList`/`ExFrameNumber` pattern documented in `.spec.rs:34–66` and
  `tcb-allowed.md:274–288`. Not individually enumerated under the
  identity_map TCB section, but this is the sanctioned type-registration
  mechanism, not an exec trust shim — **not** a blocker.

### Fix Request
None. All 12 checklist items PASS with tool-backed evidence. No code change
required this phase.

### Conclusion
RESOLVED — cheating-elimination checklist fully satisfied for
`mm::virt::identity_map`.
