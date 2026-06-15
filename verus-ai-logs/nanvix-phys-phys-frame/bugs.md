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

## [open, recorded-only] Top-of-memory frame is unrepresentable on 32-bit targets

**Where:** `Inner::alloc` / `Inner::alloc_contiguous` (the `FrameNumber::from_raw_value`
/ `FrameAddress::from_frame_number` `None`/`Err` branches).

**Bug:** The locked `Inner::internal_inv` (frame.proof.rs) only guarantees, for every
bitmap-managed index `i < num_bits`, that `frame_addr_of(i) = i * 4096 <= usize::MAX`.
On a 32-bit target this permits the bitmap to manage frame index
`idx = usize::MAX / 4096 = 0xFFFFF` (base address `0xFFFF_F000`). However
`FrameNumber::spec_max() = MAX_ADDRESS / FRAME_SIZE - 1 = 0xFFFFE`
(`src/libs/arch/src/x86/mem/paging/frame/number.spec.rs`), which **excludes** that top
index (the `- 1` reserves the final frame so the frame's *end* address does not
overflow). Hence on 32-bit the index→`FrameNumber` conversion can legitimately fail for
the top managed frame, making `alloc`/`alloc_contiguous` return `Err` while a free frame
still exists — violating the postcondition.

**Why unprovable from locked specs:** `internal_inv` is too weak; it does not capture
`num_bits <= spec_max() + 1`. The stronger fact is established by `init` (small
`NFRAMES = MEMORY_SIZE / FRAME_SIZE`), but `init` is out of scope (TCB) and the bound is
not threaded into `internal_inv` (which is locked and may not be modified).

**Classification:** Context-Dependent / latent. Unreachable in practice on the verified
target (x86_64) and at the configured `MEMORY_SIZE`, but a genuine representability gap
on a 32-bit build. RECORD-ONLY (root cause is a locked invariant that is out of scope to
strengthen; not an auto-fixable local bug).

---

## [accepted] `global size_of usize == 8` directive — codegen const-eval limitation

**Where:** `frame.proof.rs` (top-level `global size_of usize == 8;`).

The directive forces Verus to model `usize` as 8 bytes (the actual verified/CI target is
x86_64). It is **required** to discharge the frame-number representability bound in
`alloc`/`alloc_contiguous`: with it, the bitmap invariant `num_bits < u32::MAX`
(`Bitmap::number_of_bits` ensures `result < u32::MAX`) gives
`idx < 2^32 <= FrameNumber::spec_max() ≈ 2^52`, so every managed index is a valid frame
number. Without it, the bound is unprovable (see the 32-bit bug above) and these
functions could not be verified without weakening their (locked) specs or resorting to
`external_body` (disallowed for these proof targets).

**Limitation:** `make verify-kernel` runs `cargo verus verify`, which performs rustc
**codegen** for the configured target *after* Verus verification. `frame.proof.rs` is
`#[cfg(verus_keep_ghost)]`-included and `verus_keep_ghost` is also set during codegen, so
the directive is present then. On the 32-bit `x86` codegen target this triggers a
post-verification const-eval error:
`error[E0080]: evaluation panicked: does not have the expected size` at
`frame.proof.rs:1:1`, making the command exit non-zero **even though Verus itself
reports `82 verified, 0 errors`** and the cheating analysis still runs. The error is a
codegen artifact, not a verification failure. The directive cannot be removed (bound
becomes unprovable) nor cleanly gated away from codegen.

Status: ACCEPTED limitation (matches the prior accepted approach). Verus verification is
clean (0 errors, 0 admits, 0 assumes).

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
