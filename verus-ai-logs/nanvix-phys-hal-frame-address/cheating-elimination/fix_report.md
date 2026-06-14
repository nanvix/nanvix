# Cheating Elimination Report: hal-frame-address

## Cheating Counts (before → after)
Module-scoped (`make verify-kernel MODULE=hal::mem::types::address::frame`):

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 1 | 1 | 0 (TCB-approved, gate does not count it) |
| cfg-gated exec | 1 | 0 | 1 |

Module verdict: **CLEAN** (was `CHEATING_DETECTED`). Verification: `6 verified, 0 errors`.
(The crate-global `external_body=11 admit=29 cfg_gate=14` totals belong to other,
out-of-scope modules and are pre-existing.)

## Items Eliminated
- **cfg-gated exec code (frame.rs:36)** — the `verus! { … }` block holding the
  spec items (`spec_page_size`, `impl View for FrameAddress`, `FrameAddress::inv`)
  was preceded by `#[cfg(verus_keep_ghost)]`. The cheating-gate heuristic
  (`scripts/verify.sh::count_cfg_gates`) flags any `#[cfg(verus_keep_ghost)]`
  whose following item is not a `use`/`include!`/`mod`, so this gate counted as
  one cfg-gated-exec hit. Removed the gate, matching the verified sibling
  `phys.rs` (whose `verus!` View-impl block is ungated). This is pure ghost/spec
  code: under non-ghost `cargo build` the `verus!` macro erases it, so leaving it
  ungated is semantically identical and the gate no longer fires.
  - Evidence it is safe: non-ghost kernel build succeeds
    (`cargo build … --features microvm,trace` → `Finished` 0 errors), and Verus
    still processes the block under `verus_keep_ghost` (6 verified, 0 errors).

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-frame-address/verification_todo.md)
- `<PhysicalAddress as ::sys::mm::Address>::from_raw_value` `assume_specification`
  (frame.spec.rs:20): external-bottom placeholder for the not-yet-verified `phys`
  sibling module's `from_raw_value` (phys.rs:185 has no `#[verus_spec]`).
  Explicitly listed in `verus-ai-logs/tcb-allowed.md`; not counted by the cheating
  gate; removed when `phys` is verified bottom-up. Out of scope (would require
  editing an unlisted function in another module).

## AST Consistency
- Zero mismatches confirmed: YES
  (`ast_consistency.py --base-ref verus-ai-prove-bottom-up frame.rs summary`:
  matched=9 mismatched=0 missing=0 extra=0). The cfg-gate removal does not alter
  any exec function/struct AST.

## Result: PASS
