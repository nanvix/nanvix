// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// FrameAddress — Specifications
//
// A `FrameAddress` is an opaque, page-aligned physical frame address: a `Copy`
// value object that names one physical frame. Its abstract value is exactly the
// frame's base physical address as an unbounded integer (`FrameAddress@ : int`,
// defined by the `View` impl below, delegating to the inner
// `PageAligned<PhysicalAddress>`). Callers exchange it through two equivalent
// identities of that single integer:
//
//   * the raw physical address — `into_raw_value` / `from_raw_value`; and
//   * the frame index — `into_frame_number` / `from_frame_number`, where the
//     index is `address / PAGE_SIZE` (`spec_frame_number`).
//
// The frame-index helpers (`spec_frame_number`, `spec_from_number`), the frame
// representability bound (`spec_max_frame_number`), the `arch` `FrameNumber`
// projection (`spec_frame_raw_value`) and the universal address projection
// (`spec_addr`) are all reused from the sibling `phys` / `aligned::page`
// specifications, so every member of the address tower speaks the same
// vocabulary.

verus! {

use crate::hal::mem::{
    spec_addr,
    spec_frame_number,
    spec_from_number,
    spec_frame_raw_value,
    spec_max_frame_number,
};

// ── Canonical page/frame size ────────────────────────────────────────────────
//
// `spec_page_size()` is the canonical frame size, re-exported from
// `crate::hal::mem` and shared by the whole address tower
// (`FRAME_SIZE == PAGE_SIZE == spec_page_size()`). It is an external-bottom
// trust boundary: a per-architecture runtime constant with no concrete value at
// the spec level. Kept here (verification material) so the exec source carries
// no cfg-gated verification constructs, mirroring `phys.rs`.
pub uninterp spec fn spec_page_size() -> int;

// `::arch::mem::PAGE_SIZE` is the runtime page size, tied to `spec_page_size()`.
pub assume_specification[ ::arch::mem::PAGE_SIZE ] -> (result: usize)
    ensures
        result == spec_page_size(),
;

// ── View ─────────────────────────────────────────────────────────────────────
//
// `FrameAddress` abstracts to a single mathematical integer: the frame's base
// physical address, delegated from the inner `PageAligned<PhysicalAddress>`.
// `closed` so the two-level newtype delegation does not leak; callers obtain a
// usable `int` (`fa@`) for arithmetic, frame indexing, set membership, and
// comparison.
impl View for FrameAddress {
    type V = int;

    closed spec fn view(&self) -> int {
        self.0@
    }
}

// ── Well-formedness invariant ────────────────────────────────────────────────
//
// Two structural guarantees every constructor establishes and callers rely on:
//
//   * page alignment (`self@ % spec_page_size() == 0`) — relied on at every MMU
//     / page-table / allocator site, never re-checked; and
//   * frame-number representability
//     (`spec_frame_number(self@) <= spec_max_frame_number()`) — the load-bearing
//     fact that makes `into_frame_number` total (its inner
//     `FrameNumber::from_raw_value(..).unwrap()` never panics) and lets the frame
//     allocator use the result directly as a bitmap / refcount index.
//
// Stated purely over the abstract address `self@`, independent of the inner
// representation.
impl FrameAddress {
    pub open spec fn inv(&self) -> bool {
        &&& self@ % spec_page_size() == 0
        &&& spec_frame_number(self@) <= spec_max_frame_number()
    }
}

// ── `hal::mem` library-edge trust boundary ───────────────────────────────────
//
// `<PhysicalAddress as Address>::from_raw_value` is a method of the external
// `sys::mm::Address` trait, below this module's verification boundary; a
// trait-impl method cannot be body-verified in place without marking the whole
// `impl Address for PhysicalAddress` verified (pulling its unsupported
// `usize as *const u8` sibling casts into scope — see
// `verus-ai-logs/verus-unsupported.md`). It is therefore specced here with
// `assume_specification`, mirroring the trust boundary the codebase already
// draws for `<PageAligned<T> as Address>::into_raw_value` (`page.spec.rs`) and
// `::arch::mem::PAGE_SIZE` (above).
//
// On success it validates and wraps without rounding: the abstract address
// equals the raw value (`spec_addr(&r) == value`) and the validated address has
// a representable frame number (`spec_frame_number(..) <= spec_max_frame_number()`,
// i.e. it lies within physical memory). On failure (`value` is not a valid
// physical address) no address is produced; the dynamic validity predicate is
// platform-specific, so the failure condition is left unconstrained —
// `from_raw_value`'s sole caller only branches on `Ok`/`Err`. Removed when
// `hal::mem` is verified.
pub assume_specification[ <PhysicalAddress as Address>::from_raw_value ](
    value: usize,
) -> (result: Result<PhysicalAddress, ::sys::error::Error>)
    ensures
        match result {
            Ok(r) => spec_addr(&r) == value as int
                && spec_frame_number(spec_addr(&r)) <= spec_max_frame_number(),
            Err(_) => true,
        },
;

} // verus!
