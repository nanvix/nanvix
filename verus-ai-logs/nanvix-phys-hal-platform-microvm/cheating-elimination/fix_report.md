# Cheating Elimination Report: hal-platform-microvm

## Scope

Module: `src/kernel/src/hal/platform/microvm` (source `mod.rs`, spec
`mod.spec.rs`, proof `mod.proof.rs`). In-scope target function: `gva_to_gpa`.

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Counts above are for the in-scope module `hal::platform::microvm`. A grep of the
module directory for `admit(`, `assume(`, `external_body`, `assume_specification`
returns no matches (the single textual `admit()` hit in `mod.proof.rs:10` is
prose inside a comment, not code). The crate-wide cheating gate reports
`external_body=25` / `cfg_gate=7`, but `cheating-detail.txt` confirms **every**
one of those items lives in `mm/phys/*`, `hal/mem/*`, or `mm/virt/*` and is
enumerated in `verus-ai-logs/tcb-allowed.md`; **none** are in
`hal/platform/microvm`.

## Items Eliminated

None required — the in-scope module already contained zero cheating items.

- `gva_to_gpa` (`mod.rs:436`): real exec body (`gva`), verified against a
  `#[verus_spec]` `ensures result == gva` and the View-vocabulary restatement
  `result as nat == (MicrovmTranslationView {}).spec_gva_to_gpa(gva as nat)`.
  No `external_body`, no `admit`, no `assume`.
- `lemma_translation_injective` (`mod.proof.rs:20`): discharges `v.injective()`
  with an **empty** proof body (no `admit()`), following directly from the `open`
  identity definition `spec_gva_to_gpa(x) == x`.
- The two `#[cfg(verus_keep_ghost)]` attributes (`mod.rs:9,11`) gate only the
  `include!("mod.spec.rs")` / `include!("mod.proof.rs")` ghost-content
  inclusion — the standard spec/proof wiring pattern, not a cfg-gated change to
  any exec function body.

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-platform-microvm/verification_todo.md)

None. No proof gaps remain in scope.

## AST Consistency

- `gva_to_gpa` exec body compared against
  `git show verus-ai/hal-memory-region:src/kernel/src/hal/platform/microvm/mod.rs`:
  the body is byte-for-byte identical (`{ gva }`). Only ghost artifacts (the
  `#[verus_spec]` contract and the spec/proof include files) differ, all of which
  are ghost and erased from the exec build.
- Zero mismatches confirmed: YES

## Verification Results

- `make verify-kernel MODULE=hal::platform::microvm`: exit 0, status **CLEAN**
  (`assume=0 external_body=0 admit=0` in scope; `gva_to_gpa` verified).
- `make verify` (full crate): exit 0. Crate-wide gate reports
  `assume=0 external_body=25 admit=0`; all 25 `external_body` (and the
  `external_type_specification` / `cfg_gate` items) are pre-existing,
  TCB-allowed boundaries in other modules — no regressions introduced.

## Result: PASS
