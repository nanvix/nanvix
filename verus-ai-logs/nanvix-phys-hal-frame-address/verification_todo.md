# Verification TODOs: hal-frame-address

## Proof gaps
None. All admits in this task's scope are eliminated with real proofs.

- `phys.proof.rs::lemma_from_number_no_overflow` — proven (no `admit`). The
  frame-index bound is supplied by the caller via `FrameNumber::into_raw_value`'s
  postcondition; the no-overflow product is discharged with
  `lemma_mod_bound` / `lemma_fundamental_div_mod` / `lemma_mul_inequality`
  (mirrors `arch`'s verified `pde.proof.rs::lemma_frame_address`).
- `phys.proof.rs::lemma_frame_index` — proven (no `admit`). The shift-equals-divide
  step uses `vstd::bits::lemma_usize_shr_is_div`; the bound follows from `addr.inv()`.

## Remaining trusted assumption (TCB-approved, not a proof gap)
- `<PhysicalAddress as ::sys::mm::Address>::from_raw_value` `assume_specification`
  (frame.spec.rs:20): external-bottom placeholder for the not-yet-verified `phys`
  sibling module. Listed in `verus-ai-logs/tcb-allowed.md`; not counted by the
  cheating gate; removed when `phys` is verified bottom-up.
