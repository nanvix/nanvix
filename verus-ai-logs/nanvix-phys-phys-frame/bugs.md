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
