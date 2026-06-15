# Verification TODOs: sys-virt-address

## In-scope proof gaps
None. The virt-address module (`virt.rs` + `virt.spec.rs` + `virt.proof.rs`)
contains no `admit()`, `assume()`, `external_body`, or `assume_specification`,
and verifies with 0 errors via `make verify-sys`. There is nothing deferred.

## Out-of-scope observation (not actionable here)
- `make verify-sys` reports a crate-wide `cfg_gate=1`. The single occurrence is
  `src/libs/sys/src/sys/mm/alignment.rs:151` — `#[cfg(verus_keep_ghost)]` on a
  `verus! { ... }` ghost block in the `mm::alignment` module.
- It was introduced by an earlier module's verus pass (commit `64079d8db`,
  "verify PASS (cheating detected): kernel::hal::mem::types::address::aligned::page").
- It is **not** part of the sys-virt-address scope (`virt.rs` and the four listed
  `VirtualAddress` functions). Per the hard rule "Do not touch unlisted functions /
  unrelated code paths," it is left untouched and should be addressed by the
  `mm::alignment` module's own cheating-elimination task.
