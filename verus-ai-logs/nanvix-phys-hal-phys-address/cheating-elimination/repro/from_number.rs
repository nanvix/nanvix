// Minimal reproducer for the `PhysicalAddress::from_number` VERUS REWRITE.
//
// Models the real situation: `FrameNumber` is an opaque newtype whose bound
// (`self@ <= spec_max()`) is *only* exposed through the postcondition of its exec
// method `into_raw_value()` — exactly as in the `arch` crate, where the
// `#[verifier::type_invariant]` is private to that crate and cannot be opened with
// `use_type_invariant` from the kernel crate.
//
// `from_number` must compute `frame.into_raw_value() * SIZE`, an overflow-bearing
// multiply. The no-overflow lemma needs `frame@ <= spec_max()` in context, which only
// becomes available AFTER the `into_raw_value()` call returns. A single expression
// `frame.into_raw_value() * SIZE` leaves no point to invoke the lemma between the call
// and the multiply, so Verus reports a possible-overflow error. Splitting the call into
// an intermediate `addr_raw` binding (then calling the lemma, then multiplying) verifies.
//
// Run:
//   verus from_number.rs                       # FAILS  (function `bad`)
//   verus --verify-function good from_number.rs # PASSES (function `good`)
//
// Toggle which one is active by commenting the other out; both are kept for the record.

use vstd::prelude::*;

verus! {

pub const SIZE: usize = 4096;

pub uninterp spec fn spec_max() -> int;

pub struct FrameNumber(usize);

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

// The product fits in usize (assumed here to isolate the *ordering* limitation, not the
// arithmetic; in the kernel this is `lemma_from_number_no_overflow`, fully proven).
#[verifier::external_body]
pub proof fn lemma_no_overflow(frame: FrameNumber)
    requires
        frame@ <= spec_max(),
    ensures
        frame@ * (SIZE as int) <= usize::MAX as int,
{
}

// ---------------------------------------------------------------------------------------
// BAD: original single-line form. Verus cannot place the lemma between the `into_raw_value`
// call and the multiply, so the multiply has no proof that it does not overflow.
//
//   error: possible arithmetic underflow/overflow
//       let addr: usize = frame.into_raw_value() * SIZE;
//                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
// ---------------------------------------------------------------------------------------
pub fn bad(frame: FrameNumber) -> usize {
    let addr: usize = frame.into_raw_value() * SIZE;
    addr
}

// ---------------------------------------------------------------------------------------
// GOOD: the VERUS REWRITE. Split the call into `addr_raw` so the `into_raw_value`
// postcondition (`addr_raw@ == frame@ <= spec_max()`) is in context, invoke the lemma,
// then multiply. Same value, same operations, same time/space complexity.
// ---------------------------------------------------------------------------------------
pub fn good(frame: FrameNumber) -> usize {
    let addr_raw: usize = frame.into_raw_value();
    proof {
        lemma_no_overflow(frame);
    }
    let addr: usize = addr_raw * SIZE;
    addr
}

} // verus!

fn main() {}
