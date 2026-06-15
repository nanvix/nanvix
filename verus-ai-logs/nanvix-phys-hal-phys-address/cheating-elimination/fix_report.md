# Cheating Elimination Report: hal-phys-address

Module: `kernel::hal::mem::types::address::phys`
Files: `phys.rs`, `phys.spec.rs`, `phys.proof.rs`
Command: `make verify-kernel MODULE=hal::mem::types::address::phys` → exit 0, `status: CLEAN`,
"✅ No cheating detected in module hal::mem::types::address::phys".

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 1 | 1 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Counts measured by the official detector (`guardrails.detect_cheating`, patterns in
`config.CHEATING_PATTERNS`) over the three module files:

```
phys.rs       assume 0 external_body 0 admit 0 trusted 0
phys.spec.rs  assume 0 external_body 0 admit 0 trusted 0
phys.proof.rs assume 0 external_body 0 admit 0 trusted 0
```

The module-scoped cheating gate reports `assume=0 external_body=0 admit=0 trusted=0
no_decreases=0` for `hal::mem::types::address::phys`. The crate-wide totals printed by
`make verify` (`external_body=19 admit=14 cfg_gate=19`) all originate in **other,
out-of-scope** modules (`mm::phys::frame`, `mm::phys::kframe`, `arch::*`,
`bump_allocator`, …); none reference `PhysicalAddress` / `address/phys` — confirmed by
`grep` over `cheating-detail.txt` ("NO phys-address entries").

## Items Eliminated
- None required: the module was already free of all gate-counted cheating
  (`admit`, `assume(`, `external_body`, `trusted`, `no_decreases`/cfg-gated exec) on the
  base branch `verus-ai-prove`. The four in-scope functions
  (`PhysicalAddress` struct, `from_mmio_address`, `from_number`, `into_frame_number`)
  carry real `#[verus_spec]` contracts discharged by in-body `proof!` blocks plus the
  proven lemmas `lemma_from_number_no_overflow` and `lemma_frame_index`
  (`phys.proof.rs`). No `admit`/`assume`/`external_body` is present anywhere in the
  module to remove.

## assume_specification (not a gate-counted cheating item; documented for completeness)
- `phys.spec.rs:61` —
  `assume_specification[ <::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value ]`
  with `ensures result as int == addr@`.
  - **Not flagged by the cheating gate.** The detector's `assume` pattern is
    `\bassume\s*\(` (a call `assume(...)`); `assume_specification[...]` does not match
    (no `(` follows `assume`, and `assume_specification` is a distinct token). Verified
    empirically: `detect_cheating` returns `assume=0` for `phys.spec.rs`.
  - **Legitimate cross-module placeholder, not eliminable from within this module.**
    The real, verified contract already exists on the trait method
    `sys::mm::Address::into_raw_value` (`address/mod.rs:63-67`,
    `ensures result as int == self@`). It is not yet *inherited* by the concrete
    `impl Address for VirtualAddress` (`sys/.../virt.rs:167`) because that impl block is
    not annotated `#[verus_verify]`. Annotating it would force verification of the whole
    impl (`align_up`/`align_down`/`is_aligned`/`as_ptr`/…) and edits the **`sys` crate**,
    which is a *separate verification target* (`sys::sys::mm::address::virt`) and outside
    the `hal-phys-address` scope ("do not touch unlisted functions").
  - This mirrors the established, documented pattern for the sibling placeholders that
    were removed only once their owning module was verified (see the `phys.spec.rs`
    comments at lines 56-77: `VirtualAddress::new`, `FRAME_SHIFT`,
    `FrameNumber::{from,into}_raw_value` were each superseded by real specs). This one is
    superseded when `sys::sys::mm::address::virt` lands its in-body verification of
    `impl Address for VirtualAddress`.

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-phys-address/verification_todo.md)
- 1 entry recorded: the cross-module `assume_specification` above (owned by the `sys`
  virt-address module). No `admit()`/`assume()`/`external_body` proof gaps remain in this
  module.

## AST Consistency
- Zero mismatches confirmed: YES.
  `git diff verus-ai-prove -- phys.rs phys.spec.rs phys.proof.rs` is empty — the module's
  exec (and spec/proof) code is byte-identical to the base branch. No exec signatures,
  semantics, time complexity, or space complexity were changed (no changes at all were
  required). No `cfg`-gated exec divergence introduced.

## Result: PASS
- `make verify-kernel MODULE=hal::mem::types::address::phys`: exit 0, module CLEAN,
  "No cheating detected".
- `make verify` (full crate): exit 0, no regressions (crate-wide cheating counts unchanged
  from base; all residual items are in out-of-scope modules).
- Zero gate-counted cheating in scope (`admit=0 assume=0 external_body=0 trusted=0
  cfg_gate=0`).
