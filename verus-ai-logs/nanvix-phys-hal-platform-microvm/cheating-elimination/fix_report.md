# Cheating Elimination Report: hal-platform-microvm

## Scope
In-scope module: `src/kernel/src/hal/platform/microvm/mod.rs`
In-scope (verification-order) function: `gva_to_gpa`

## Cheating Counts (before → after)
Counts below are scoped to the microvm module (`src/kernel/src/hal/platform/microvm/`).

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Note: `make verify-kernel` reports crate-wide tallies (`external_body=19 admit=12
cfg_gate=19`). These originate from other modules (out of scope here) and are
covered by `verus-ai-logs/tcb-allowed.md`. The module verification status is
`CLEAN` (exit 0).

## Items Eliminated
- None required. The module had no cheating to begin with.
  - `gva_to_gpa` already carries a real, complete contract via
    `#[verus_spec(result => ensures result as int == spec_gva_to_gpa(gva as int))]`
    with the implementation body `gva`. The spec `spec_gva_to_gpa(gva) = gva`
    (identity, MicroVM GVA==GPA platform invariant), so the postcondition
    discharges trivially in-body. No `external_body`, `admit`, `assume`, or
    `assume_specification` is present.

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-platform-microvm/verification_todo.md)
- None. No proof gaps remain in scope.

## AST Consistency
- Zero mismatches confirmed: YES.
  `git diff verus-ai-prove -- src/kernel/src/hal/platform/microvm/` is empty; the
  module is byte-identical to the base branch. The `#[cfg(...)]` attributes in the
  module are pre-existing platform/feature gates (`whp`, `pit`, `smp`, `stdio`,
  `exception-stack-guard`) and the standard `#[cfg(verus_keep_ghost)]` spec/proof
  includes — none are cheating gates of exec code and none were introduced.

## Verification
- `make verify-kernel MODULE=hal::platform::microvm` → status: CLEAN, exit 0.
  `gva_to_gpa` is the 1/31 function carrying a verified contract; it is absent
  from the unverified list.

## Result: PASS
