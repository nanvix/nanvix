# Cheating Elimination Report: hal-memory-region

## Scope

Module `hal::mem::types::region`, files:
- `src/kernel/src/hal/mem/types/region.rs`
- `src/kernel/src/hal/mem/types/region.spec.rs`
- `src/kernel/src/hal/mem/types/region.proof.rs`

Verification-order target functions: `TruncatedMemoryRegion::start`,
`MemoryRegion::start`, `TruncatedMemoryRegion::size`, `MemoryRegion::size`.

## Cheating Counts (before → after)

Counts below are **for the in-scope module** (`make verify-kernel
MODULE=hal::mem::types::region`).

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

The three region files contain no `admit`, `assume`, `external_body`,
`assume_specification`, or cfg-gated exec code. The module verifier reports
`✅ No cheating detected in module hal::mem::types::region.`

## Items Eliminated

None required. At the cheating-elimination START commit the module already
carried real `#[verus_spec]` contracts on all four target functions and proved
them in-body:

- `MemoryRegion::<T>::start` — `ensures result@ == self@.start`; body is
  `self.start.clone_address()`, discharged from the `Address::clone_address`
  contract plus the `MemoryRegion` `view()` definition (`start: self.start@`).
- `MemoryRegion::<T>::size` — `ensures result as int == self@.size`; body returns
  `self.size`, discharged directly from the `view()` definition
  (`size: self.size as int`).
- `TruncatedMemoryRegion::<T>::start` — `ensures result@ == self@.start`; delegates
  to `self.0.start()`, with `TruncatedMemoryRegion::view()` defined as `self.0@`.
- `TruncatedMemoryRegion::<T>::size` — `ensures result as int == self@.size`;
  delegates to `self.0.size()`.

`make verify-kernel MODULE=hal::mem::types::region` → `5 verified, 0 errors`
(forced, non-cached run), `status: CLEAN`.

The global counts surfaced by a full `make verify-kernel` (admit=12,
external_body=19, cfg_gate=19) all reside in out-of-scope modules
(`mm/phys`, `mm/virt`, `arch`, …) and are accounted for in
`verus-ai-logs/tcb-allowed.md`. None live in any `region` file
(confirmed via `cheating-detail.txt`).

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-memory-region/verification_todo.md)

None. No proof gaps remain in scope; no `admit()`/`assume()` introduced or
outstanding.

## AST Consistency

- `git diff verus-ai-prove -- region.rs region.spec.rs region.proof.rs` is empty:
  the in-scope files are byte-identical to the base branch, so no exec code was
  modified, no cfg gates were introduced, and semantics / time complexity /
  space complexity are trivially preserved.
- Zero mismatches confirmed: **YES**

## Regression Check

- Full `make verify-kernel`: exit 0, `5 verified` in-module; global cheating
  counts (admit=12 external_body=19 cfg_gate=19) match the base branch commit
  `c4b739de0` exactly — no regression introduced.

## Result: PASS
