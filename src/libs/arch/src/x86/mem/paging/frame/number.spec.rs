verus! {

use crate::mem;

// ── Frame size / representability bound ───────────────────────────────────────
//
// `FrameNumber` is, to its callers, the abstract identity of one physical page
// frame: a bounded non-negative integer index. The two spec constants below fix
// the vocabulary every contract speaks. They share the names the upstream
// callers already trust in `phys.spec.rs` / `tcb-allowed.md`, so the in-module
// contracts and the callers' assumed contracts coincide by construction.

// The frame size as an unbounded integer, tied to the exec `mem::FRAME_SIZE`
// (an alias of `PAGE_SIZE`). Strictly positive; positivity is the load-bearing
// fact behind the no-overflow reasoning of every caller that scales a frame
// index by the frame size.
pub open spec fn spec_frame_size() -> int {
    mem::FRAME_SIZE as int
}

// The inclusive maximum frame index, mirroring the exec
// `FrameNumber::MAX == mem::MAX_ADDRESS / mem::FRAME_SIZE - 1`. A frame index in
// `0 ..= spec_max_frame_number()` scaled by `spec_frame_size()` stays within
// `usize`, which is exactly the no-overflow guarantee `phys.rs::from_number`,
// `pde.rs` and `pte.rs` depend on.
pub open spec fn spec_max_frame_number() -> int {
    mem::MAX_ADDRESS as int / spec_frame_size() - 1
}

// ── View ──────────────────────────────────────────────────────────────────────
//
// A `FrameNumber` abstracts to a single mathematical integer: the raw frame
// index. `closed` so the `usize`-newtype mapping does not leak; callers still
// obtain a usable `int` (`f@`) for indexing, arithmetic and comparison, while
// the concrete `usize` is recovered through `into_raw_value`'s contract
// (`result as int == self@`).
impl View for FrameNumber {
    type V = int;

    closed spec fn view(&self) -> int {
        self.0 as int
    }
}

// ── Well-formedness invariant ─────────────────────────────────────────────────
//
// Every `FrameNumber` names a representable frame: its index lies in
// `0 ..= spec_max_frame_number()`. Declared as a `#[verifier::type_invariant]`
// so the bound holds for *every* value of the type without a precondition —
// `into_raw_value`'s callers rely on the range unconditionally (they never hold
// a precondition obligation when projecting a frame they already own). Stated
// purely over the abstract index `self@`, independent of the inner `usize`.
impl FrameNumber {
    #[verifier::type_invariant]
    pub open spec fn inv(&self) -> bool {
        &&& 0 <= self@
        &&& self@ <= spec_max_frame_number()
    }
}

} // verus!
