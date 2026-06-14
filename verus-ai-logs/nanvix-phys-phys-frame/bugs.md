# Bugs / Suspicious code — `mm::phys::frame`

## [open] `Inner::alloc_range`: possible off-by-one between body and spec

- Body (frame.rs:588-589, 598, 626):
  - `start_frame_number = region.start().into_frame_number().into_raw_value()`
  - `end_frame_number = start_frame_number + region.size() / mem::FRAME_SIZE - 1`
  - loops are inclusive: `for index in start_frame_number..=end_frame_number`
- Spec (frame.rs:561-564):
  - `start_frame_number = region@.start / spec_page_size()`
  - `end_frame_number = (region@.start + region@.size) / spec_page_size()` (exclusive)
  - `frame_numbers = set_int_range(start, end)` (half-open `[start, end)`)

The body's inclusive `..=(start + size/FS - 1)` covers frames
`start .. start + size/FS`, i.e. `size/FS` frames. The spec's half-open range covers
`(start+size)/FS - start/FS` frames. These coincide only when both `region@.start`
and `region@.size` are exact multiples of `FRAME_SIZE` (so integer division does not
truncate). `region.inv()` presumably guarantees page-alignment of `start`; the `size`
multiple-of-FRAME_SIZE assumption must be confirmed, otherwise the booked set differs
from the spec'd set by one frame.

Currently masked by `proof! { admit(); }` at the top of `alloc_range`. The proving
phase must confirm `region.inv() ==> region@.start % page_size == 0 && region@.size %
page_size == 0` (or adjust the spec/body) before removing the admit.

Status: to confirm in proving phase. Raised during specification-phase review (turn 1).

## [auto-fixed] panic-on-valid-input: `into_frame_number().unwrap()` on top-of-space aligned address

**Where**: `Inner::free` (frame.rs:300), `Inner::share` (:381), `Inner::refcount` (:444),
`Inner::book` (:499), `Inner::is_covered` (:536), `Inner::alloc_range` (:587).

**What**: Each method converted an input address to a frame index via
`X.into_frame_number().into_raw_value()`. `PhysicalAddress/FrameAddress::into_frame_number`
is a *checked* conversion: internally `FrameNumber::from_raw_value(addr >> FRAME_SHIFT).unwrap()`,
which **panics** when the frame number exceeds `FrameNumber::MAX = MAX_ADDRESS/FRAME_SIZE - 1`.

**Why**: With `MAX_ADDRESS == usize::MAX`, the single page-aligned address
`usize::MAX - 4095` maps to frame `usize::MAX/4096 = FrameNumber::MAX + 1`, which arch
deliberately excludes (the top frame's end address `base + FRAME_SIZE` would overflow `usize`).
The method preconditions only guarantee page-alignment (`frame.inv()` / `phys_addr.inv()`),
which does **not** rule out this address. So a caller passing the top-of-space aligned address
crashes the kernel. Each method already has a downstream guard that rejects oversized frame
numbers gracefully (`frame_number >= self.refcount.len()`, `>= num_bits`, or bitmap `Err`),
but the panic in `into_frame_number` fires *before* the guard, making the guard unreachable
for that input.

**Verification Failure**: `into_frame_number` requires
`spec_frame_number(self@) <= spec_max_frame_number()`; unprovable from page-alignment alone
(false for `self@ == usize::MAX - 4095`). Command: `make verify-kernel MODULE=mm::phys`.

**How Verus Helped**: The panic is unreachable on real hardware (physical addresses never reach
the top of the 64-bit space), so neither testing nor review would surface it. Formal
verification, modeling `MAX_ADDRESS == usize::MAX`, exposed the reachable `unwrap` panic.

**Severity**: safety-critical (kernel panic / DoS on a precondition-satisfying input).

**Suggested/Applied Fix**: Replace the checked, panicking conversion with the equivalent
*total* computation `X.into_raw_value() / mem::FRAME_SIZE` (same value `addr@ / PAGE_SIZE` for
all in-range frames). The existing downstream guards then reject the out-of-range top frame
cleanly with `Err`/`false`, matching each method's `Err`/coverage postcondition.

**Auto-Fixed**: Yes — replaced `X.into_frame_number().into_raw_value()` with
`X.into_raw_value() / mem::FRAME_SIZE` at the six sites above (`// VERUS BUG FIX:` comments).
No specs weakened; no changes to the arch/address layers.
