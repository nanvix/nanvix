verus! {

//==================================================================================================
// Module spec constant
//==================================================================================================

// Models `FrameNumber::MAX = MAX_ADDRESS / FRAME_SIZE - 1`: the largest representable frame index.
// A module-wide constant, not per-value state. Realizes the kernel's uninterpreted
// `spec_max_frame_number()` (see `hal/mem/types/address/phys.spec.rs`).
pub open spec fn spec_max_frame_number() -> int {
    FrameNumber::MAX as int
}

//==================================================================================================
// View
//==================================================================================================

impl View for FrameNumber {
    type V = int;

    // `closed`: callers reference `self@`, but the mapping to the inner `usize` field is hidden.
    // The abstract value is "the frame index as int". When `arch` is verified this realizes the
    // kernel's uninterpreted `spec_frame_raw_value(frame)`.
    closed spec fn view(&self) -> int {
        self.0 as int
    }
}

//==================================================================================================
// Type invariant
//==================================================================================================

impl FrameNumber {
    // Every constructible `FrameNumber` is bounded by `MAX`. Enforced as a type invariant so the
    // bound holds unconditionally for any value callers hold (PTE/PDE treat it as an always-valid
    // token), which is exactly what lets `into_raw_value() << FRAME_SHIFT` not overflow `usize`.
    #[verifier::type_invariant]
    pub open spec fn inv(&self) -> bool {
        0 <= self@ <= spec_max_frame_number()
    }
}

} // verus!
