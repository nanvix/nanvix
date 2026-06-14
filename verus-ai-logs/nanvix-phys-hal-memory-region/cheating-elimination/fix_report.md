# Cheating Elimination Report: hal-memory-region

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Scope is the `hal::mem::types::region` module: `region.rs`, `region.spec.rs`,
`region.proof.rs`. In-scope functions: `MemoryRegion::start`, `MemoryRegion::size`,
`TruncatedMemoryRegion::start`, `TruncatedMemoryRegion::size`.

The module-scoped run reports:

```
=== Cheating Pattern Check ===
  ✅ No cheating detected in module hal::mem::types::region.
=== Summary ===
  status: CLEAN
```

The global counts surfaced by the full-crate run
(`external_body=11 admit=27 cfg_gate=14`) all reside in **other** modules
(`mm/phys/*`, `arch/*`, etc.) and are out of scope for this task. The
`cheating-detail.txt` listing contains zero entries under
`src/kernel/src/hal/mem/types/region*` (`grep "hal/mem/types"` → no matches).
The only `*region*` entries are `mm/phys/mod.rs::book_physical_memory_regions`
and `mm/phys/mod.rs::book_mmio_regions`, which belong to the `mm::phys` module.

## Items Eliminated
- None required. The `hal::mem::types::region` module already carried real
  `#[verus_spec]` contracts and verified bodies for all four in-scope accessors,
  with no `admit`, `assume`, `external_body`, `assume_specification`, or
  cfg-gated exec code present. The specs:
  - `MemoryRegion::start` — `ensures result@ == self@.start`
  - `MemoryRegion::size` — `ensures result as int == self@.size`
  - `TruncatedMemoryRegion::start` — `ensures result@ == self@.start`
  - `TruncatedMemoryRegion::size` — `ensures result as int == self@.size`
  These verify directly from the closed `View` definitions in `region.spec.rs`
  and the underlying address `clone_address` contract; no proof-file lemmas are
  needed (`region.proof.rs` is `verus! { }`).

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-memory-region/verification_todo.md)
- None. No proof gaps remain in scope; no `admit()`/`assume()` were introduced
  or left behind.

## AST Consistency
- Zero mismatches confirmed: YES. `git diff verus-ai-prove-bottom-up -- region.rs
  region.spec.rs region.proof.rs` is empty; the exec sources are byte-identical
  to the base branch. No exec code was changed, so semantics, time complexity,
  and space complexity are trivially preserved.

## Result: PASS
