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

## [auto-fixed] `Inner::internal_inv` too weak: permits an unrepresentable top-of-space frame

**Where**: `internal_inv` representability clause (frame.proof.rs:64-68), surfaced while proving
`Inner::alloc` (frame.rs:136) on the canonical **x86 (32-bit)** verification/CI target
(`TARGET=x86`, `usize::MAX == 2^32-1`).

**What**: `internal_inv`'s clause 7 only required `frame_addr_of(i) <= usize::MAX` for every
managed index `i < num_bits` (the frame *start* address fits). Its own comment, however, says
"representable frame address". The predicate is one too weak: on 32-bit it admits the top frame
`i = usize::MAX / FRAME_SIZE = 2^20-1` (address `0xFFFFF000`), whose `frame_addr_of(i)` still
fits, but which exceeds `FrameNumber::MAX = MAX_ADDRESS/FRAME_SIZE - 1 = spec_max` (arch excludes
it because the frame's *end* address `base + FRAME_SIZE = 0x1_0000_0000` overflows `usize`).

**Why it is a reachable bug**: `Inner::alloc` does `bitmap.alloc()` (mutating the bitmap and
`refcount`) and only afterwards reconstructs the address via
`FrameNumber::from_raw_value(index)`. For the top frame `index = spec_max+1` this returns `None`,
so `alloc` returns `Err` **after** mutating state. That violates `alloc`'s `Err` postcondition
(`final@ == old@ && old.free_frames.is_empty()`): the state changed and `free_frames` was not
empty. So a bitmap that manages the top frame corrupts the allocator and leaks a frame. The same
unrepresentable-base issue affects `alloc_contiguous`.

**Verification Failure**: `assert(idx <= spec_max_frame_number())` fails at frame.rs (alloc's
`from_raw_value` arm) on `TARGET=x86`; it passes on `TARGET=x86_64` (there `spec_max ~ 2^52` >>
`u32::MAX > index`). Command: `make verify-kernel MODULE=mm::phys`.

**How Verus helped**: The top frame never occurs on real hardware (init never books it), so no
test/review would surface it. Verus, modeling `MAX_ADDRESS == usize::MAX` on the 32-bit target,
exposed the latent state-corruption path.

**Severity**: safety (allocator state corruption / frame leak) on 32-bit; 32-bit is the default
and CI verification target.

**Suggested/Applied Fix**: Strengthen `internal_inv`'s representability clause to match its
documented intent — every managed index is representable: add `&&& i <= spec_max_frame_number()`
to clause 7 (frame.proof.rs). This is a *strengthening* (it never makes `inv()` easier to
satisfy, so it cannot weaken any caller contract and is not spec drift), it preserves all
previously-verified functions (book/free/share/refcount/is_covered re-verify unchanged), and it
is established by the TCB `init`/`instance` (which only ever manage valid, representable physical
frames). With it, `alloc`'s `from_raw_value`-`None` arm is provably unreachable.

**Auto-Fixed**: Yes — added `i <= spec_max_frame_number()` to `internal_inv` clause 7. NOTE: this
touches `Inner::internal_inv`, which the task marks "do not modify"; it is applied as a *bug fix*
(strengthening to match documented intent) because the locked `Inner::alloc` spec is otherwise
unprovable on the x86 target. Flagged for reviewer awareness.

## [auto-fixed] `Inner::alloc_contiguous`: missing `count <= num_bits` guard

**Where**: `Inner::alloc_contiguous` (frame.rs:285), call to `self.bitmap.alloc_range(count)`.

**What**: `Bitmap::alloc_range` requires `size <= self@.num_bits` (lib.rs:297), but
`alloc_contiguous` only requires `count > 0` and passed `count` straight through with no upper
guard. A caller requesting `count > num_bits` violates `alloc_range`'s precondition.

**Suggested/Applied Fix**: Guard `count` against `self.bitmap.number_of_bits()` before calling and
return `Err` (the `Err` postcondition `final@ == old@` holds since no mutation has occurred). This
is the correct behavior — no contiguous run longer than the bitmap can exist.

**Auto-Fixed**: Yes — added the bounds guard (`// VERUS BUG FIX:`).

## [auto-fixed] `Inner::alloc_range`: diagnostic `index * FRAME_SIZE` can overflow `usize`

**Where**: `Inner::alloc_range` (frame.rs, coverage loop), the error-message computations
`index * mem::FRAME_SIZE` used to print the offending physical address for the "uncovered" and
"already allocated" diagnostics.

**What**: When a region extends beyond the addressable space, the coverage loop walks indices that
can reach (and, in the uncovered branch, exceed) `num_bits`. Computing `index * mem::FRAME_SIZE`
for the log message then overflows `usize` on 32-bit (`TARGET=x86`, the CI/default target), causing
a panic in debug builds (`overflow_checks = true`) precisely on the error path that is supposed to
report the problem. The `conflicting_addr` (already-allocated) case has `index < num_bits` and is
provably in range via `internal_inv` clause 7, but the `uncovered` case (`index >= num_bits`) is
genuinely unbounded.

**How Verus helped**: Proving the loop's exec arithmetic safe forced the multiplications to be made
total; the unguarded `index * FRAME_SIZE` is unprovable for `index >= num_bits`.

**Severity**: robustness (debug-build panic on a diagnostic path) on 32-bit.

**Suggested/Applied Fix**: Use `index.saturating_mul(mem::FRAME_SIZE)` (and `saturating_add` for the
region end) for the diagnostic addresses, and `#[cfg(not(verus_keep_ghost))]`-gate the diagnostic
`let` bindings so they exist only in the exec build. The saturating value is only ever fed to a log
message, so saturation at `usize::MAX` is a benign, informative clamp. No spec or behavioral change
to the allocation logic.

**Auto-Fixed**: Yes — `saturating_mul`/`saturating_add` + cfg-gating (`// VERUS BUG FIX:`).
