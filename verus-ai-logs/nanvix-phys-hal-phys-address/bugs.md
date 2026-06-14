# Bugs — hal::mem::types::address::phys

Module: `src/kernel/src/hal/mem/types/address/phys.rs`

## B1 — `make verify-sys` regression: `#[verus_verify]` on an unverifiable trait impl (FIXED)

- **Where:** `src/libs/sys/src/sys/mm/address/virt.rs`, `impl Address for VirtualAddress` block.
- **Symptom:** `make verify-sys` failed with a compilation/setup error:
  ```
  error: Verus does not support this cast: `usize` to `*const u8`
  error: Verus does not support this cast: `usize` to `*mut u8`
  ```
- **Root cause:** A prior commit added `#[verus_verify]` to the trait-impl block. Verus then
  requires the *entire* impl to verify, but the sibling `as_ptr`/`as_mut_ptr` methods perform
  `usize as *const u8` / `usize as *mut u8` int-to-pointer casts that Verus cannot translate. Git
  history confirms the regression: `d54fd253d` PASS (block un-annotated) → later commit added the
  attribute → `c7a556350` FAIL.
- **Classification:** Auto-fixable regression (un-buildable verification target). Fixed by reverting
  the breaking attribute (block left un-annotated, matching the last PASS state) and adding an
  explanatory note. `make verify-sys` now PASSES (6 verified, 0 errors, CLEAN).
- **Consequence for this module:** `into_raw_value` (a trait-impl method) therefore cannot be
  verified in `sys` (whole-impl rule + unsupported pointer casts), so the kernel retains a single
  documented `assume_specification` for it (`ensures result as int == addr@`). Recorded in
  `verus-ai-logs/tcb-allowed.md`; isolated reproducers in `../specification/whole_impl_rule.rs` and
  `../specification/ptr_cast.rs`.

## Target functions

No code bugs in the in-scope target functions (`PhysicalAddress::into_frame_number`, `from_number`,
view, `from_mmio_address`). They verify cleanly with the existing proofs; no contract weakened.
Module-scoped result: `6 verified, 0 errors` — CLEAN, `assume=0`, no `admit()`/`external_body` in
the module.
