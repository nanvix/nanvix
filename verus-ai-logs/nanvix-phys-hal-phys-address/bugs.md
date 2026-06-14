# Bugs — hal::mem::types::address::phys

Module: `src/kernel/src/hal/mem/types/address/phys.rs`

## Summary

No code bugs found during verification.

The target functions (`PhysicalAddress::into_frame_number`, `PhysicalAddress::from_number`,
`PhysicalAddress` view, `PhysicalAddress::from_mmio_address`) verify cleanly with their existing
proofs (`phys.proof.rs`: `lemma_from_number_no_overflow`, `lemma_frame_index`). The no-overflow
obligation for the base-address multiply and the shift-equals-divide step for the frame index are
both discharged without weakening any contract.

Verification result (module-scoped, fresh / non-cached):
`6 verified, 0 errors` — CLEAN, no `admit()` / `assume()` / `external_body` in module scope.
