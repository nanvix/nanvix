## Turn 1: Full checklist sweep — hal-platform-microvm cheating elimination

### Progress
- Done (PASS): all 12 checklist items verified PASS with tool evidence (see below).
- Current: final sweep complete.
- Remaining: none.

### Scope note
Review target is the `kernel::hal::platform::microvm` module:
- `src/kernel/src/hal/platform/microvm/mod.rs` (993 lines)
- `mod.spec.rs`, `mod.proof.rs`, plus submodules `pvclock.rs`, `start.rs`, `start16.rs`.

`make verify-kernel` runs a **crate-wide** cheating scan, so its global summary
(`external_body=11 admit=27 cfg_gate=14 status=CHEATING_DETECTED`) includes
pre-existing TCB items in *other* modules (`mm/phys/*`, `mm/virt/identity_map.rs`),
which are separate review targets and out of scope here. I re-ran every scan
**module-scoped** to the microvm directory.

### Verification

**Cheating markers — module-scoped (tree-sitter `detect_cheating`):**
```
start16.rs   assume=0 external_body=0 admit=0 trusted=0 no_decreases=0
mod.rs       assume=0 external_body=0 admit=0 trusted=0 no_decreases=0
pvclock.rs   assume=0 external_body=0 admit=0 trusted=0 no_decreases=0
start.rs     assume=0 external_body=0 admit=0 trusted=0 no_decreases=0
mod.proof.rs assume=0 external_body=0 admit=0 trusted=0 no_decreases=0
mod.spec.rs  assume=0 external_body=0 admit=0 trusted=0 no_decreases=0
COMBINED {"assume":0,"external_body":0,"admit":0,"trusted":0,"no_decreases":0}
```
Confirmed independently with grep over the module dir (0 hits for `admit()`,
`assume(`, `external_body`, `trusted`, `exec_allows_no_decreases_clause`).

**cfg-gated exec — module-scoped (verify.sh `count_cfg_gates` logic):**
`microvm cfg_gate exec count: 0`. The only `#[cfg(verus_keep_ghost)]` gates
(mod.rs:9,11) sit directly above `include!("mod.spec.rs")` / `include!("mod.proof.rs")`,
which the counter explicitly excludes (target starts with `include!`). All other
`#[cfg(...)]` in the module are legitimate platform feature gates
(`whp`, `pit`, `smp`, `stdio`, `debug_assertions`) — the cfg-gate cheating pattern
only matches `verus_keep_ghost`, so none are flagged.

**AST consistency (`ast_consistency.py --base-ref dev ... summary`):**
`Consistent: ✅ YES (matched=28 mismatched=0 missing=0 extra=0)` — all 28 exec
functions and 3 structs report MATCH against the pristine `dev` baseline. No exec
rewrites exist (0 `VERUS REWRITE` comments), consistent with 0 AST mismatches.

**Spec drift (`spec_drift.py git-diff`):**
- vs cheating-elimination start `d65075eb`: `✅ No contract drift detected` (0 ensures removed, 0 requires added).
- vs proving boundary `a74af586`: `✅ No contract drift detected`.
- `git diff --stat` and `git status --porcelain` over the module: **empty** — the
  module was untouched during the cheating-elimination phase (nothing to eliminate).

**Cross-module regression (`make verify-kernel`, whole crate):** Exit code 0,
`cached (no recompilation)` — all verified modules pass.

**Build (`./z build -- all`):** Exit code 0. Grep for `warning:` / `error:` over the
build output returns no compiler diagnostics (only a benign environment note about a
missing `sysroot-debug` symlink, unrelated to code). 0 errors, 0 warnings.

### Checklist results
| # | Item | Result | Evidence |
|---|------|--------|----------|
| 1 | Zero admit() | PASS | module scan admit=0 |
| 2 | Zero assume() | PASS | module scan assume=0 |
| 3 | Zero trusted functions | PASS | module scan trusted=0 |
| 4 | Zero exec_allows_no_decreases_clause | PASS | module scan no_decreases=0 |
| 5 | Zero cfg-gated exec code | PASS | module cfg_gate exec count=0 (only include!-guarding verus_keep_ghost) |
| 6 | Zero external_body unless TCB-listed | PASS | module scan external_body=0 (none to list) |
| 7 | AST consistency: zero mismatches | PASS | matched=28 mismatched=0 missing=0 extra=0 |
| 8 | Exec rewrites have VERUS REWRITE + reproducer | PASS (vacuous) | 0 rewrites, 0 AST mismatches |
| 9 | Each surviving external_body in tcb-allowed | PASS (vacuous) | none surviving |
| 10 | No specs weakened | PASS | spec_drift 0 contract drift; module diff empty |
| 11 | Cross-module regression | PASS | make verify-kernel exit 0 |
| 12 | Verification + build, 0 errors/0 warnings | PASS | verify exit 0; build exit 0, no diagnostics |

### Fix Request
None. All checklist items PASS with tool-backed evidence. The microvm module
entered the cheating-elimination phase with zero cheating markers and was not
modified during the phase; specs and exec AST are unchanged from baseline, and
both verification and build are clean.

Verdict: **RESOLVED**.
