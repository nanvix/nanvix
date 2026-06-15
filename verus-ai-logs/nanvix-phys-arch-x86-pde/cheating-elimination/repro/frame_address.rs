// Minimal reproducer for the `PageDirectoryEntry::frame_address` VERUS REWRITE.
//
// Models the real situation: `FrameNumber` is an opaque newtype whose bound
// (`self@ <= spec_max()`) is *only* exposed through the postcondition of its exec
// method `into_raw_value()` — exactly as in the `arch` crate.
//
// `frame_address` must compute `frame.into_raw_value() << FRAME_SHIFT`, an
// overflow-bearing left shift whose result must additionally satisfy two `ensures`:
//   - `result as int == self@ * FRAME_SIZE` (shift == multiply, no overflow), and
//   - `result as int % FRAME_SIZE == 0` (FRAME_SIZE-alignment).
// The bridging lemma (`lemma_frame_address`, fully proven in pde.proof.rs via
// `lemma_usize_shl_is_mul` + the div/mod lemmas) needs `raw <= spec_max()` in
// context, which only becomes available AFTER the `into_raw_value()` call returns.
// A single expression `frame.into_raw_value() << FRAME_SHIFT` leaves no point to
// invoke the lemma between the call and the shift, and an exec call cannot appear
// inside `proof!`, so the operand must be named. Splitting the call into an
// intermediate `raw` binding (then calling the lemma, then shifting) verifies.
//
// Run:
//   verus frame_address.rs                                        # FAILS  (function `bad`)
//   verus --verify-root --verify-function good frame_address.rs   # PASSES (function `good`)
//
// Both `bad` and `good` are kept for the record; comment out `bad` to see `good` pass.

use vstd::prelude::*;

verus! {

pub const FRAME_SHIFT: usize = 12;
pub const FRAME_SIZE: usize = 4096;

pub uninterp spec fn spec_max() -> int;

pub struct FrameNumber(usize);

impl Clone for FrameNumber {
    fn clone(&self) -> (r: FrameNumber)
        ensures r == *self
    { FrameNumber(self.0) }
}

impl Copy for FrameNumber {}

impl View for FrameNumber {
    type V = int;
    closed spec fn view(&self) -> int { self.0 as int }
}

impl FrameNumber {
    // The bound is exposed ONLY via this exec postcondition (cross-crate opaque newtype).
    #[verifier::external_body]
    pub fn into_raw_value(self) -> (r: usize)
        ensures
            r as int == self@,
            0 <= self@ <= spec_max(),
    {
        self.0
    }
}

// The shift equals the multiply and is FRAME_SIZE-aligned; the bound keeps it in `usize`.
// (In the `arch` crate this is `lemma_frame_address`, fully proven from
// `lemma_usize_shl_is_mul` + `lemma_mod_multiples_basic`. Assumed here to isolate the
// *ordering* limitation, not the arithmetic.)
#[verifier::external_body]
pub proof fn lemma_frame_address(raw: usize)
    requires
        0 <= raw as int <= spec_max(),
    ensures
        (raw << FRAME_SHIFT) as int == raw as int * (FRAME_SIZE as int),
        (raw << FRAME_SHIFT) as int % (FRAME_SIZE as int) == 0,
{
}

// ---------------------------------------------------------------------------------------
// BAD: original single-line form. There is no point between the `into_raw_value` call and
// the shift to invoke `lemma_frame_address`, and the call cannot live inside `proof!`, so
// the two `ensures` (shift==multiply, alignment) are unproven.
//
//   error: postcondition not satisfied
//       self.frame.into_raw_value() << FRAME_SHIFT
//       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
// ---------------------------------------------------------------------------------------
pub struct Pde { pub frame: FrameNumber }

impl Pde {
    pub fn bad(&self) -> (result: usize)
        ensures
            result as int == self.frame@ * (FRAME_SIZE as int),
            result as int % (FRAME_SIZE as int) == 0,
    {
        self.frame.into_raw_value() << FRAME_SHIFT
    }

    // -----------------------------------------------------------------------------------
    // GOOD: the VERUS REWRITE. Split the call into `raw` so the `into_raw_value`
    // postcondition (`raw@ == self.frame@ <= spec_max()`) is in context, invoke the lemma,
    // then shift. Same value, same operations, same time/space complexity.
    // -----------------------------------------------------------------------------------
    pub fn good(&self) -> (result: usize)
        ensures
            result as int == self.frame@ * (FRAME_SIZE as int),
            result as int % (FRAME_SIZE as int) == 0,
    {
        let raw: usize = self.frame.into_raw_value();
        proof { lemma_frame_address(raw); }
        raw << FRAME_SHIFT
    }
}

} // verus!

fn main() {}
