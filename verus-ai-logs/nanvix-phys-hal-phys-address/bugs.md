# Bugs — `src/kernel/src/hal/mem/types/address/phys.rs`

None.

The verification targets (`from_mmio_address`, `from_number`, `into_frame_number`,
and the `PhysicalAddress` view/`inv`) were all proven without uncovering any code
bug. The two non-trivial proof obligations were genuine proof gaps, not defects:

- `from_number`'s base-address multiply (`frame.into_raw_value() * FRAME_SIZE`) is
  overflow-free because every `FrameNumber` is bounded by `FrameNumber::spec_max()
  == MAX_ADDRESS / FRAME_SIZE - 1`. The bound is supplied by `into_raw_value`'s
  postcondition; `lemma_from_number_no_overflow` closes the arithmetic.
- `into_frame_number`'s `raw_addr >> FRAME_SHIFT` equals `addr / FRAME_SIZE` and
  fits a `FrameNumber` under `inv()`; `lemma_frame_index` discharges this via
  `lemma_usize_shr_is_div`, after deriving `FRAME_SHIFT == 12 < usize::BITS` from
  `spec_page_size() == pow2(FRAME_SHIFT)` (the kernel target is 32-bit, so
  `usize::BITS == 32`).
