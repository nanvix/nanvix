# Cheating Elimination Report: hal-phys-address

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 1 | 1 | 0 (TCB-allowed) |
| cfg-gated exec | 0 | 0 | 0 |

Scope = the three module files
`src/kernel/src/hal/mem/types/address/{phys.rs, phys.spec.rs, phys.proof.rs}`.
Module-scoped gate `make verify-kernel MODULE=hal::mem::types::address::phys`:
**exit 0**, `✅ No cheating detected in module hal::mem::types::address::phys`
(`assume=0 external_body=0 admit=0 cfg_gate=0` for the module; the global
`external_body=11 admit=27 cfg_gate=14` belong to out-of-scope `mm/phys/*`
modules and are explicitly not in scope — "Do not touch unlisted functions").

## Items Eliminated
- None required. The four in-scope verification-order targets
  (`PhysicalAddress` struct/`View`, `from_mmio_address`, `from_number`,
  `into_frame_number`) are already fully proven with real `#[verus_spec]`
  contracts and real proofs:
  - `from_number` — no-overflow obligation discharged by
    `lemma_from_number_no_overflow` (proof file), nonlinear-arith over the
    `FrameNumber::spec_max()` bound. No cheating.
  - `into_frame_number` — shift-equals-divide and frame-index bound discharged
    by `lemma_frame_index` (proof file) via `lemma_usize_shr_is_div` +
    `lemma2_to64`. The `.unwrap()` is underwritten by `inv()`. No cheating.
  - `from_mmio_address` — identity-wrap contract proven directly. No cheating.
  - `PhysicalAddress` / `View` — `closed spec fn view` over `self.0@`. No cheating.

## Retained trust construct (not eliminable from this module's scope)
- `assume_specification[ <::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value ]`
  (`phys.spec.rs:74`, `ensures result as int == addr@`).
  - **Escalation ladder followed.** Searched vstd; wrote two isolated reproducers
    (`specification/whole_impl_rule.rs`, `specification/ptr_cast.rs`); tried the
    equivalent rewrite of verifying the real method in `sys`.
  - **Why it cannot be eliminated:** `into_raw_value` is a trait-impl method
    (`<VirtualAddress as Address>::into_raw_value`). Verus requires the *entire*
    trait impl to be verified as a unit ("the entire impl must be verified").
    The same `impl Address for VirtualAddress` block contains `as_ptr`/`as_mut_ptr`,
    whose `usize as *const u8` / `usize as *mut u8` int-to-pointer casts Verus
    rejects ("Verus does not support this cast"). Both errors are reproduced
    verbatim by the two isolated reproducers above. Verifying it would instead
    require `external_body` on `as_ptr`/`as_mut_ptr` — i.e. *expanding* the TCB
    to remove a single trivial assumption (body `self.0` trivially satisfies the
    contract). The smaller, more honest trust boundary is the `assume_specification`.
  - **It is not flagged by the cheating gate** (module check reports `assume=0`,
    `trusted=0`, and it is absent from `cheating-detail.txt`). It is a cross-crate
    dependency contract for the not-yet-verifiable `sys` crate, documented in
    `verus-ai-logs/tcb-allowed.md` ("`assume_specification` retained due to a
    genuine Verus limitation"). Eliminating it would require touching the
    out-of-scope `sys` crate.

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-phys-address/verification_todo.md)
- None. There are zero proof gaps (no `admit()`/`assume()` anywhere in scope), so
  no `verification_todo.md` is created. The retained `assume_specification` is a
  TCB-allowed cross-crate boundary, not a proof gap.

## AST Consistency
- Zero mismatches confirmed: YES. All three in-scope files are byte-identical to
  the base branch (`git diff verus-ai-prove-bottom-up -- <file>` is empty for
  `phys.rs`, `phys.spec.rs`, `phys.proof.rs`). The only `#[cfg(verus_keep_ghost)]`
  gates are the standard `include!("phys.spec.rs")` / `include!("phys.proof.rs")`
  ghost-include lines (present on the base branch); they gate spec/proof inclusion,
  not exec code. No exec code was changed; semantics, time, and space complexity
  are unchanged.

## Result: PASS
