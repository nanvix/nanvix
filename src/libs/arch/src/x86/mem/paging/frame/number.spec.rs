verus! {

//==================================================================================================
// Module spec constant
//==================================================================================================

// Models `FrameNumber::MAX = MAX_ADDRESS / FRAME_SIZE - 1`: the largest representable frame index.
// A module-wide constant, not per-value state. It is `uninterp` (its concrete value derives from the
// build-time page-size constants, an external-bottom boundary) and is tied to the exec constant
// `FrameNumber::MAX` by the `assume_specification` below. Realizes the kernel's uninterpreted
// `spec_max_frame_number()` (see `hal/mem/types/address/phys.spec.rs`).
pub uninterp spec fn spec_max_frame_number() -> nat;

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

//==================================================================================================
// Exec constant binding
//==================================================================================================

// Ties the exec constant `FrameNumber::MAX` to the abstract `spec_max_frame_number()`. `MAX`'s
// body (`MAX_ADDRESS / FRAME_SIZE - 1`) bottoms out at build-time page-size constants that Verus
// treats as external; this trusted contract names that boundary without interpreting it. The
// `usize` result makes `spec_max_frame_number()` non-negative for callers.
pub assume_specification[ FrameNumber::MAX ] -> (result: usize)
    ensures
        result as int == spec_max_frame_number() as int,
;

} // verus!
