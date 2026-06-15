## Turn 1: Full Checklist Walkthrough — Cheating Elimination (hal-frame-address)

### Progress
- Done (PASS): all 12 checklist items — see Verification below.
- Current: none — full checklist verified.
- Remaining: none.

### Scope
Module files reviewed:
- `src/kernel/src/hal/mem/types/address/frame.rs`
- `src/kernel/src/hal/mem/types/address/frame.spec.rs`
- `src/kernel/src/hal/mem/types/address/frame.proof.rs`

Note on global counters: `make verify-kernel` reports repo-wide cheating
counts (`external_body=20 admit=12 cfg_gate=19`). These aggregate **all** kernel
modules. The per-file detail (`verus-logs/cheating-detail.txt`) confirms this
module contributes exactly **one** entry; every other count belongs to other
in-flight modules (`mm/phys/*`, `mm/virt/*`) with their own TCB entries and is
out of scope for this review.

### Verification

**Verus run** — `make verify-kernel`
- `note: verifying module hal::mem::types::address::frame`
- Exit code: 0 (cached, no recompilation). Cross-module run is `Module: <all>`.

**1. Zero admit() — PASS**
`grep -rn admit` on the three files: 0 hits.
`cheating-detail.txt`: no admit entries under `types/address/frame`.

**2. Zero assume() — PASS**
`grep -rn assume`: only historical references in comments (`frame.rs:40`,
`frame.spec.rs:11`). No `assume(...)` call sites.

**3. Zero trusted functions — PASS**
Global cheating check: `trusted=0`. No `#[trusted]` / `external_fn_specification`
in the module.

**4. Zero exec_allows_no_decreases_clause — PASS**
Global check `no_decreases=0`; `grep` for `no_decreases` in module: 0 hits.

**5. Zero cfg-gated exec code — PASS**
Only `#[cfg(verus_keep_ghost)]` occurrences are at `frame.rs:9,11,36`:
- 9, 11 gate `include!("frame.spec.rs")` / `include!("frame.proof.rs")` (ghost).
- 36 gates the `verus! { ... }` block containing only spec items
  (`spec_page_size`, `impl View`, `FrameAddress::inv`).
No exec code is cfg-gated.

**6. Zero external_body unless TCB-listed — PASS**
`cheating-detail.txt` shows exactly one for this module:
`hal/mem/types/address/frame.rs:102 from_raw_value: external_body`.
Listed in `verus-ai-logs/tcb-allowed.md:137-138`
(`FrameAddress::from_raw_value`). `into_raw_value` is also TCB-listed but is now
a plain `#[verus_spec]` delegate (no `external_body`) — stronger than required.

**7. AST consistency — PASS**
`scripts/ast_consistency.py` (tree-sitter exec hash):
- vs pristine baseline `38885545d` (caller-analysis START): `summary` →
  all 9 functions + 1 struct `MATCH`; `Consistent: YES (mismatched=0)`.
- vs proving start `1f6205c5` and cheating-elim start `5e93724e`: consistent.
Exec code is byte-for-byte (AST) unchanged across the whole pipeline.

**8. All exec rewrites have VERUS REWRITE comment + reproducer — PASS (vacuous)**
AST consistency shows zero exec changes, so there are no rewrites to document.

**9. Each surviving external_body confirmed TCB-listed — PASS**
Only `from_raw_value`; confirmed present in `tcb-allowed.md` (see item 6).

**10. No specs weakened — PASS**
`scripts/spec_drift.py git-diff frame.rs --before 5e93724e` (cheating-elim phase
start → working tree): `Functions with changes: 0`, `Contract drift: 0`,
`Ensures removed: 0`, `Requires added: 0`. `✅ No contract drift detected.`

**11. Cross-module regression — PASS**
`make verify-kernel` runs `Module: <all>`, exit code 0 (cached). All verified
modules pass.

**12. Verification + build — PASS**
- Verus: `make verify-kernel` exit 0.
- Build: `./z build -- all-kernel` exit 0. Forced recompile (touch frame.rs +
  rebuild) produced no `warning`/`error` lines. 0 errors, 0 warnings.

### Result
All 12 checklist items PASS with concrete tool evidence. No fix requests.
Creating STOP = RESOLVED.
