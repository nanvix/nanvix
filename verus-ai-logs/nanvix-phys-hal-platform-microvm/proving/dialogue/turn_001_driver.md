## Turn 1: Full checklist walkthrough — hal::platform::microvm (proving)

### Progress
- Done (PASS, evidence below): ALL items
  1. No specs weakened
  2. Zero admit()
  3. Zero external_body (TCB rule)
  4. Zero assume/assume_specification
  5. No cfg-gated exec code
  6. Cheating audit (exact counts/locations)
  7. Claimed Verus limitation reproducer
  8. Exec rewrites minimal / `// VERUS REWRITE`
  9. Cross-module regression
  10. Verification + build (0 errors, 0 warnings)
- Current: final sign-off
- Remaining: none

### Verification

Scope under review (proving phase):
- `src/kernel/src/hal/platform/microvm/mod.rs`
- `src/kernel/src/hal/platform/microvm/mod.spec.rs`
- `src/kernel/src/hal/platform/microvm/mod.proof.rs`

Commands run:
- `make verify-kernel` → exit 0. `note: verifying module hal::platform::microvm`.
  Verus verification cached/passed (0 errors). Crate-wide cheating summary:
  `assume=0 external_body=11 admit=27 trusted=0 no_decreases=0 cfg_gate=14`,
  status `CHEATING_DETECTED`.
- `./z build -- all-kernel` → `[OK] Build complete.`, 0 errors, 0 warnings
  (kernel compiled with `--features "microvm trace"`).

**Item 1 — No specs weakened.** Git diff of the proving commit `194b299b8` shows the
spec file went from `verus! { } // verus!` (empty) to a real `pub open spec fn
spec_gva_to_gpa(gva: int) -> int { gva }` identity contract, and `gva_to_gpa` exec
gained `#[verus_spec(result => ensures result as int == spec_gva_to_gpa(gva as int))]`.
Strengthened from nothing to a load-bearing identity contract; nothing weakened. **PASS.**

**Item 2 — Zero admit().** `verus-logs/cheating-detail.txt` lists every admit in the
crate; grep for `microvm` returns ZERO matches. All 27 admits are in `mm/phys/*` and
`mm/virt/identity_map.rs` (other in-progress modules), none in the module under review.
`grep -rn "admit" microvm/` → 0. **PASS.**

**Item 3 — Zero external_body (TCB rule).** `cheating-detail.txt` external_body entries
are all in `mm/phys/{frame,kframe,manager,mod,upool}.rs` — none in microvm.
`grep -rn "external_body" microvm/` → 0. `verus-ai-logs/tcb-allowed.md` lists no microvm
function (none needed). **PASS.**

**Item 4 — Zero assume/assume_specification.** `grep -rn "assume" microvm/` → 0.
Crate `assume=0`. The only `assume`-style strings in mod.rs are English doc comments
("It assumes that the standard output device is present"). **PASS.**

**Item 5 — No cfg-gated exec code.** The repo's own detector (`scripts/verify.sh`
`count_cfg_gates`) defines cfg-gated exec cheating as `#[cfg(verus_keep_ghost)]` /
`#[cfg(not(verus_keep_ghost))]` gating of exec code (verified-vs-compiled divergence).
microvm has only `#[cfg(verus_keep_ghost)] include!("mod.spec.rs"/"mod.proof.rs")`
(lines 9–12), which the detector explicitly excludes (`include!` target). The remaining
`#[cfg(feature = "whp"/"pit"/"smp"/"stdio")]` gates are legitimate platform-build
configuration (not a verus_keep_ghost verification-hiding trick), are NOT flagged by the
tool, and were NOT introduced or touched by the proving commit. microvm contributes 0 to
the crate `cfg_gate=14` count. **PASS.**

**Item 6 — Cheating audit (exact counts/locations) for microvm.**
- admit: 0 (locations: none)
- external_body: 0 (locations: none)
- assume / assume_specification: 0 (locations: none)
- cfg-gated exec code (verus_keep_ghost): 0 (only excluded `include!` gates at mod.rs:9,11)
Evidence: `grep -rn "admit\|external_body\|assume_specification\|\bassume\b" microvm/`
returns nothing; `cheating-detail.txt` has no microvm line. **PASS.**

**Item 7 — Claimed Verus limitation reproducer.** No Verus limitation is claimed for
this module (no `verus-unsupported.md`, no bug note, no external_body/assume to justify).
Nothing to reproduce. **PASS (N/A).**

**Item 8 — Exec rewrites minimal / `// VERUS REWRITE`.** `grep -rn "VERUS REWRITE"
microvm/` → 0. The proving commit's only exec change is the added `#[verus_spec]`
attribute on `gva_to_gpa`; the body is unchanged (`gva`). No rewrites performed. **PASS.**

**Item 9 — Cross-module regression.** `make verify-kernel` runs the whole kernel crate
and exits 0 — no verified module fails. The crate-wide admit/external_body counts are
pre-existing intentional placeholders in still-in-progress modules (`mm/phys`, `mm/virt`)
documented in `tcb-allowed.md`, not regressions introduced by microvm. **PASS.**

**Item 10 — Verification + build (0 errors, 0 warnings).** `make verify-kernel` exit 0,
0 verification errors. `./z build -- all-kernel` → `[OK] Build complete.` with no warning
lines emitted. **PASS.**

### Fix Request
None. Every checklist item is PASS with concrete tool/code evidence. The microvm proving
phase is clean: a single verified function (`gva_to_gpa`) pinned to a strengthened
identity spec (`spec_gva_to_gpa`), zero admit/external_body/assume, zero verus_keep_ghost
exec gating, zero rewrites, verification and build both green.

RESOLVED.
