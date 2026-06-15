# Cheating Elimination Report: hal-memory-region

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Scope: module `hal::mem::types::region` (functions `MemoryRegion::start`,
`MemoryRegion::size`, `TruncatedMemoryRegion::start`, `TruncatedMemoryRegion::size`).

The module-scoped cheating gate reports:
`✅ No cheating detected in module hal::mem::types::region.`
There were **zero** cheating items in any of the in-scope files
(`region.rs`, `region.spec.rs`, `region.proof.rs`) before or after this pass.

## Items Eliminated
- None required. The four in-scope functions carry real `#[verus_spec]`
  postconditions and verify cleanly with no `admit`/`assume`/`external_body`.
  - `MemoryRegion::start` → `ensures spec_addr(&result) == self@.start`
  - `MemoryRegion::size` → `ensures result as int == self@.size`
  - `TruncatedMemoryRegion::start` → `ensures spec_addr(&result) == self@.start`
    (delegates to inner `MemoryRegion::start`)
  - `TruncatedMemoryRegion::size` → `ensures result as int == self@.size`
    (delegates to inner `MemoryRegion::size`)

## Global cheating items (out of scope, TCB-allowed)
`make verify-kernel` reports global `external_body=25 cfg_gate=7`. None of these
reside in the in-scope region files (`grep types/region cheating-detail.txt` →
NONE). Every one is an other-module item already governed by
`verus-ai-logs/tcb-allowed.md` (e.g. `mm/phys/*`, `hal/mem/types/address/*`).
They are outside the `hal-memory-region` scope and unchanged by this task.

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-memory-region/verification_todo.md)
- None. No proof gaps remain; no `admit()`/`assume()` recorded.

## AST Consistency
One exec-code line differs from the base branch
`verus-ai/hal-phys-address`: `MemoryRegion::start` returns `self.start`
instead of `self.start.clone()`.

- **Required by Verus (evidence):** reverting to `self.start.clone()` and
  re-running `make verify-kernel MODULE=hal::mem::types::region` fails with
  `error: postcondition not satisfied … crate::hal::mem::spec_addr(&result) == self@.start`
  (`4 verified, 1 errors`). Verus has no specification that `Clone::clone` on a
  generic `T: Address` preserves `spec_addr`, whereas a direct field copy is the
  identity, so `result == self.start` discharges the postcondition by congruence
  of `spec_addr`.
- **Semantics / complexity preserved:** the `Address` trait requires
  `Self: … + Copy` (`src/libs/sys/src/sys/mm/address/mod.rs:33`). For any `Copy`
  type, `clone()` is by contract identical to a bitwise copy, so the observable
  result is unchanged and both are O(1) time and O(1) space.

All other differences from the base are ghost-only (`#[verus_spec]` contracts and
the relocation of the view/invariant definitions into `region.spec.rs`), which do
not affect exec AST.

- Zero exec-semantics mismatches confirmed: YES

## Result: PASS
