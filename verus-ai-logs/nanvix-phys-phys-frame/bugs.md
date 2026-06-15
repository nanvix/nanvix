# Bugs / Suspicious code — `mm::phys::frame`

## [resolved] `Inner::alloc_range`: off-by-one between body and spec — confirmed safe

**Resolution (proving phase):** `TruncatedMemoryRegion::inv()`
(`src/kernel/src/hal/mem/types/region.spec.rs`) guarantees *both*
`self@.start % spec_page_size() == 0` **and** `self@.size % spec_page_size() == 0`
(via `is_page_aligned()`), plus `self@.size > 0`. With both endpoints page-aligned,
`(region@.start + region@.size) / PS == region@.start / PS + region@.size / PS`, so the
body's frame count `size/FS` exactly equals the spec's half-open count
`(start+size)/PS - start/PS`. No off-by-one. The booked set matches the spec'd set.

During proving, `alloc_range` was rewritten to use a **half-open** loop
(`start_frame_number .. start_frame_number + count`, `count = size/FS`) instead of the
original inclusive `..=(start + size/FS - 1)`. The two are behaviourally identical for
`count >= 1` (guaranteed by `size > 0 && size % PS == 0 ==> size >= PS ==> count >= 1`),
but the half-open form (a) aligns directly with the spec's `set_int_range` and the
`lemma_book_range` helper, and (b) removes the `- 1` on a `usize`, which would have
underflowed had `count == 0` ever been reachable. `admit()` removed; fully verified.

Status: RESOLVED. No code bug; behaviour-preserving refactor for provability.

---

## [RESOLVED] Top-of-memory frame is unrepresentable on 32-bit targets

**Where:** `Inner::alloc` / `Inner::alloc_contiguous` (the `FrameNumber::from_raw_value`
/ `FrameAddress::from_frame_number` `None`/`Err` branches).

**Bug (original):** `Inner::internal_inv` (frame.proof.rs) only guaranteed, for every
bitmap-managed index `i < num_bits`, that `frame_addr_of(i) = i * 4096 <= usize::MAX`.
On a 32-bit target this permits the bitmap to manage frame index
`idx = usize::MAX / 4096 = 0xFFFFF` (base address `0xFFFF_F000`). However
`FrameNumber::spec_max() = MAX_ADDRESS / FRAME_SIZE - 1 = 0xFFFFE`
(`src/libs/arch/src/x86/mem/paging/frame/number.spec.rs`), which **excludes** that top
index (the `- 1` reserves the final frame so the frame's *end* address does not
overflow). Hence on 32-bit the index→`FrameNumber` conversion could legitimately fail for
the top managed frame, making `alloc`/`alloc_contiguous` return `Err` while a free frame
still exists — violating the postcondition.

**Fix (turn 1 review):** `Inner::internal_inv` was strengthened with the *correct*
representability conjunct:
`self.bitmap@.num_bits <= FrameNumber::spec_max() + 1`.
This is the real fact established by `init` (`NFRAMES = MEMORY_SIZE / FRAME_SIZE`, far
below `spec_max()`); since `instance()`/`init` are the allow-listed TCB boundary that
constructs the singleton and `ensures (*r).inv()`, the conjunct is sound to assume from
the trust boundary. `alloc`/`alloc_contiguous` now discharge the conversion bound from
`idx < num_bits <= spec_max() + 1 ==> idx <= spec_max()` — **target-agnostic**, holding on
both 32- and 64-bit builds. The conjunct is preserved by every proof target because
`num_bits` is never resized. This replaces the previous `global size_of usize == 8`
workaround (removed; see below).

**Classification:** Auto-fixed (invariant strengthening; no spec weakened — strengthening
an invariant only adds guarantees). RESOLVED.

---

## [RESOLVED] `global size_of usize == 8` directive — removed

**Where (was):** `frame.proof.rs` (top-level `global size_of usize == 8;`).

The directive forced Verus to model `usize` as 8 bytes to discharge the frame-number
representability bound in `alloc`/`alloc_contiguous`. It caused a post-verification
codegen failure: `cargo verus verify` runs rustc **codegen** for the configured target
*after* Verus verification with `verus_keep_ghost` set, so on the default 32-bit `x86`
codegen target the const-eval assertion panicked with
`error[E0080]: evaluation panicked: does not have the expected size` at
`frame.proof.rs:1:1`, making `make verify-kernel` exit non-zero. The directive also made
the proof depend on a word size that does not hold on the compiled target.

**Fix (turn 1 review):** The directive was **deleted**. The representability bound is now
carried by `Inner::internal_inv` (`num_bits <= spec_max() + 1`, see the entry above), which
is target-agnostic and does not rely on `usize == 8`. `make verify-kernel` (default
`TARGET=x86`) now reports `112 verified, 0 errors` **and** compiles to exit 0 with no
E0080; `./z build` succeeds (exit 0).

Status: RESOLVED.

---

## [accepted] `into_frame_number()` → division robustness change

**Where:** `Inner::is_covered`, `Inner::book`, `Inner::refcount`, `Inner::alloc_range`.

These methods originally converted a physical address to a frame index via
`addr.into_frame_number().into_raw_value()`. `into_frame_number()` carries a
representable-frame-number precondition and can fail/panic on the reserved top-of-memory
frame (same root cause as the 32-bit bug above). They were changed to compute the index
by direct division: `addr.into_raw_value() / mem::FRAME_SIZE`. Both yield
`addr@ / FRAME_SIZE`, but division needs no precondition and cannot panic. This is a
behaviour-preserving robustness improvement that also makes the spec connection
(`frame_number as int == pa / spec_page_size()`) immediate.

Status: ACCEPTED. Behaviour-preserving; strengthens robustness, weakens no spec.

---

## [historical] `Inner::alloc_range`: possible off-by-one between body and spec

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
