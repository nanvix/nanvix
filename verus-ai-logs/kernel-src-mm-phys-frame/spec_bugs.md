# Spec Bugs Found During Verification: kernel/src/mm/phys/frame.rs

## Spec Bug 1: Missing Overflow Guard in alloc_range

- **Location**: `Inner::alloc_range` requires clause
- **Problem**: The original spec did not require `region@.start + region@.size <= usize::MAX`.
  The exec code computes `start_frame_number + num_frames` (both usize), which can
  overflow when the region extends beyond usize address space. The spec variables
  `start_frame_number` and `end_frame_number` are computed as divisions of
  `region@.start` and `(region@.start + region@.size)` by `spec_page_size()`, and
  these are used as usize loop bounds.
- **Fix applied**: Added `region@.start + region@.size <= usize::MAX as int` to the
  requires clause. This is a genuine precondition (not a weakening) — callers must
  ensure the region fits within the addressable memory space.
- **Justification**: This is a necessary constraint because `usize` arithmetic in
  the exec code requires the region end address to be representable. The spec was
  genuinely incomplete, not just hard to prove.
