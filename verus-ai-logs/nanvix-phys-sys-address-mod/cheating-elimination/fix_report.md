# Cheating Elimination Report: sys-address-mod

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Scope: `src/libs/sys/src/sys/mm/address/{mod.rs,mod.spec.rs,mod.proof.rs}`,
in-scope functions `is_aligned`, `into_raw_value`, `from_raw_value`.

The three in-scope items are trait *method declarations*. They carry only
ghost `#[verus_spec]` contracts (added in the phase START commit) and no
bodies, so they introduce no proof obligations of their own. No `assume`,
`admit`, `external_body`, `assume_specification`, or cfg-gated exec code is
present in the module — confirmed by the verify guardrail
(`✅ No cheating detected`, `assume=0 external_body=0 admit=0 trusted=0
no_decreases=0 cfg_gate=0`).

## Items Eliminated
- No source-level cheating items existed in the address module; nothing had to
  be rewritten. The module was already cheating-free.
- **Real blocker found and fixed (environment, not source):** `make verify-sys`
  failed with `compilation/setup error (verus did not run)` — vstd failed to
  compile (`error: expected generics to match` in `vstd .../std_specs/atomic.rs`).
  Root cause: the installed Verus at `/home/ruize/toolchain/verus`
  (`VERUS_EXECUTABLE_DIR`) had drifted to `0.2026.06.14.4ea7d0f`, while the repo
  pins `0.2026.05.31.5dd6d83` (`build/verus-version`) matching the
  `vstd 0.0.0-2026-05-31-0205` pinned in `Cargo.lock`. The version mismatch broke
  vstd compilation for **every** crate (reproduced on `verify-bitmap` too), not
  just sys — proving it was not caused by the address changes.
  Fix: reinstalled the pinned version with
  `bash scripts/setup/verus.sh /home/ruize/toolchain/verus` (used the already
  cached `.verus-cache/verus-0.2026.05.31.5dd6d83-x86-linux.zip`; no network).
  After this, `make verify-sys` → `6 verified, 0 errors`, status CLEAN.

## Verification TODOs (verus-ai-logs/nanvix-phys-sys-address-mod/verification_todo.md)
- None. Zero proof gaps; no `verification_todo.md` was created.

## AST Consistency
- Zero mismatches confirmed: YES
- Diff vs `verus-ai/hal-frame-address` for `mod.rs` touches only ghost
  constructs: added `#[verus_spec(...)]` contracts on the three trait method
  declarations, the standard `#[cfg(verus_keep_ghost)] include!("mod.spec.rs")`
  / `include!("mod.proof.rs")` lines, and relocated the existing
  `use ::vstd::prelude::*;` to `use vstd::prelude::*;` (same import). No exec
  signature, control flow, time complexity, or space complexity changed. The
  cfg-gated includes pull in ghost spec/proof only and follow the established
  repo-wide pattern — not prohibited cfg-gated exec code.

## Verification Result
- `make verify-sys`: 6 verified, 0 errors — status CLEAN, no cheating.
- `make verify` (full crate, regression check): all crates 0 errors
  (sys: 6, bitmap: 70, nanvix-slab: 35, kernel: 47). Pre-existing,
  TCB-allowed kernel cheating (`external_body=24 cfg_gate=6`) and
  bump-allocator (`external_body=2`) are unchanged from the base commit
  (`245138857`/`c92df3991`) and are outside the sys-address-mod scope.

## Result: PASS
