# Cheating Elimination Report: hal-phys-address

Module: `hal::mem::types::address::phys`
Files: `phys.rs`, `phys.spec.rs`, `phys.proof.rs`

## Cheating Counts (before → after)

Counts are for the in-scope module as measured by the module-scoped gate
(`make verify-kernel MODULE=hal::mem::types::address::phys`). The module-scoped
cheating scan covers `phys.rs` (the exec source); `phys.spec.rs` / `phys.proof.rs`
are verification-material files included only under `cfg(verus_keep_ghost)`.

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body (functions) | 0 | 0 | 0 |
| assume_specification | 9 | 9 | 0 |
| cfg-gated exec | 1 | 0 | 1 |

Module gate result before: `CHEATING_DETECTED` (cfg-gated exec code: 1).
Module gate result after: `✅ No cheating detected in module` (4 verified, 0 errors).

## Items Eliminated

- **cfg-gated exec code (1) in `phys.rs`** — the `View` impl for
  `PhysicalAddress` had been added during the specification phase as a
  `#[cfg(verus_keep_ghost)] verus! { impl View for PhysicalAddress { ... } }`
  block at the bottom of `phys.rs`. The cfg-gate detector flags the
  `verus! {` target line as cfg-gated exec code. It is pure verification
  material (a `closed spec fn view`), not exec logic.
  **Fix:** moved the `View` impl into `phys.spec.rs` (already a `verus! { }`
  block included only under `cfg(verus_keep_ghost)`), and removed the
  cfg-gated block from `phys.rs`. Semantics are identical — the impl still
  compiles only under `verus_keep_ghost` — but the exec source file
  (`phys.rs`) now carries zero cfg-gated verification constructs. A short
  comment in `phys.rs` points to the new location.

## Items Retained (justified, non-eliminable, not flagged by the gate)

- **`external_type_specification` + `external_body` on `ExFrameNumber`
  (`phys.spec.rs`)** — declares the foreign `arch::mem::paging::FrameNumber`
  as an opaque external datatype so it can appear in spec signatures.
  `external_type_specification` introduces no logical assumption and is
  explicitly permitted by the **verus-constraints** skill
  ("`external_type_specification` is safe to use freely"). It is on a *type*,
  not a function, so it is not an in-scope `external_body` blocker. Not counted
  by the module-scoped cheating gate.

- **9 `assume_specification`s (`phys.spec.rs`)** — library-edge
  (external-bottom) trust boundaries for the not-yet-Verus-enabled `arch`/`sys`
  crates: `::arch::mem::FRAME_SIZE`, `::arch::mem::FRAME_SHIFT`,
  `VirtualAddress::new`, `<VirtualAddress as Address>::into_raw_value`,
  `FrameNumber::into_raw_value`, `FrameNumber::from_raw_value`. These mirror the
  existing `sys`/`arch` boundaries the codebase already draws (e.g.
  `::arch::mem::PAGE_SIZE` in `frame.rs`). They were reviewed and approved
  during the specification phase (verdict `dialogue-RESOLVED`). Removing them
  would require verifying the foreign `arch`/`sys` crates — explicitly out of
  scope ("do not touch unlisted functions") and not a proof gap. They are not
  counted by the cheating gate.

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-phys-address/verification_todo.md)

None. The proof is complete: `4 verified, 0 errors` with no `admit()` /
`assume()` and no proof gaps. No `verification_todo.md` was created.

## AST Consistency

Two exec functions report `MISMATCH` vs the base
`verus-ai/hal-page-aligned:.../phys.rs`:

- `PhysicalAddress::from_number` and `PhysicalAddress::into_frame_number`.

Both are the **pre-approved deviation** `f(complex_expr)` →
`let x = complex_expr; f(x)` ("Intermediate value for assertions", per the
**ast-consistency** skill): the `frame * FRAME_SIZE` product and the
`raw_addr >> FRAME_SHIFT` shift are bound to intermediate locals so the proof
blocks can discharge the no-overflow / shift-equals-division obligations on
named values. Both are now documented in `phys.rs` with `VERUS DEVIATION`
comments naming the deviation. The other 14 functions and the
`PhysicalAddress` struct are `MATCH`. No un-approved exec changes were
introduced; the `View`-impl move touches only verification material, not exec
logic.

- Zero mismatches confirmed: NO (2 pre-approved, documented deviations only)

## Verification

- Module: `make verify-kernel MODULE=hal::mem::types::address::phys` →
  `4 verified, 0 errors`, module gate `✅ No cheating detected`.
- Full crate: `make verify` → all crates exit 0, `0 errors` (no regressions).
  Global counts (`external_body=25`, `cfg_gate=9`) are entirely from other
  modules' `tcb-allowed.md`-approved boundaries; the `hal-phys-address` module
  contributes none.

## Result: PASS
