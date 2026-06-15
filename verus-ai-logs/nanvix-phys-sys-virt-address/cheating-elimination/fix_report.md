# Cheating Elimination Report: sys-virt-address

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 (in scope) | 0 (in scope) | 0 |

Scope = `src/libs/sys/src/sys/mm/address/virt.rs` (+ `virt.spec.rs`, `virt.proof.rs`)
and the in-scope functions `VirtualAddress::into_raw_value`,
`VirtualAddress::from_raw_value`, `VirtualAddress::new`, and `VirtualAddress`.

### Detection details
- `grep -rEn "admit|assume|external_body|assume_specification"` over the three
  virt files: **no matches**.
- `make verify-sys` (crate-wide) and `make verify-sys MODULE=mm::address::virt`:
  both report `assume=0 external_body=0 admit=0 trusted=0 no_decreases=0`,
  **exit code 0, 0 verification errors**.
- The crate-wide scan reports `cfg_gate=1`. Locating it precisely (using the
  same Python heuristic as `scripts/verify.sh::count_cfg_gates`) shows the single
  hit is **`src/libs/sys/src/sys/mm/alignment.rs:151`** —
  `#[cfg(verus_keep_ghost)]` preceding a `verus! { ... }` ghost block. This is a
  **different module** (`mm::alignment`), not the virt-address scope.
- The two `#[cfg(verus_keep_ghost)]` lines in `virt.rs` (lines 9, 11) gate only
  `include!("virt.spec.rs")` / `include!("virt.proof.rs")`; the detector
  explicitly excludes `include!`/`use`/`mod`/`extern` targets, so virt.rs
  contributes **0** to the cfg_gate count.

## Items Eliminated
- **None required.** The virt-address module contained zero cheating items at the
  start of this phase. `virt.rs`, `virt.spec.rs`, and `virt.proof.rs` are
  byte-identical to `verus-ai-prove` (`git diff --stat` is empty), the proving
  phase already produced verified contracts for `new` / `from_raw_value`, and the
  module verifies with 0 errors. There was no `admit`, `assume`, `external_body`,
  `assume_specification`, or in-scope cfg-gated exec code to replace.

## Verification TODOs (verus-ai-logs/nanvix-phys-sys-virt-address/verification_todo.md)
- No outstanding proof gaps in the virt-address scope (no `admit()`/`assume()`
  remain; verification is clean).
- Out-of-scope note recorded in `verification_todo.md`: the crate-wide
  `cfg_gate=1` originates from `mm::alignment` (`alignment.rs:151`), introduced by
  a prior module's verus work (commit `64079d8db`, module
  `kernel::hal::mem::types::address::aligned::page`). Per the hard rule
  "Do not touch unlisted functions / unrelated code paths," it is not modified
  here; it belongs to the `mm::alignment` module's own elimination scope.

## AST Consistency
- Zero mismatches confirmed: **YES**. `git diff verus-ai-prove` for all three virt
  files is empty — no exec code, signatures, or cfg gates were changed (no changes
  were needed). Semantics, time complexity, and space complexity are trivially
  preserved.

## Verification
- `make verify-sys`: exit 0, 0 errors, `assume=0 external_body=0 admit=0` for the
  whole `sys` crate.
- No source changes were made, so there is no regression risk to the rest of the
  crate; the pipeline's post-proving gate ("Build + Verify passed after proving")
  remains valid.

## Result: PASS
The sys-virt-address module is free of all cheating constructs
(admit/assume/external_body/assume_specification/in-scope cfg-gated exec) and
verifies cleanly. The lone crate-wide `cfg_gate=1` belongs to the unrelated
`mm::alignment` module and is outside this task's scope.
