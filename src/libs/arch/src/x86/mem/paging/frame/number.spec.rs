verus! {

//==================================================================================================
// View
//==================================================================================================

impl View for FrameNumber {
    type V = int;

    // `closed`: callers reference `self@`, but the mapping to the inner `usize` field is hidden.
    // The abstract value is "the frame index as int". Realizes the kernel's uninterpreted
    // `spec_frame_raw_value(frame)`.
    closed spec fn view(&self) -> int {
        self.0 as int
    }
}

//==================================================================================================
// Abstract bound + type invariant
//==================================================================================================

impl FrameNumber {
    // Interpreted bound: the largest representable frame index, mirroring the exec constant
    // `FrameNumber::MAX = MAX_ADDRESS / FRAME_SIZE - 1`. Defined (not `uninterp`) so the binding to
    // `MAX` is discharged by verification rather than assumed. Realizes the kernel's
    // `spec_max_frame_number()` (see `hal/mem/types/address/phys.spec.rs`). The `nat` result makes
    // the bound non-negative for callers (e.g. constructing `FrameNumber::NULL`).
    pub open spec fn spec_max() -> nat {
        (mem::MAX_ADDRESS / mem::FRAME_SIZE - 1) as nat
    }

    // Every constructible `FrameNumber` is bounded by `MAX`. Enforced as a type invariant so the
    // bound holds unconditionally for any value callers hold (PTE/PDE treat it as an always-valid
    // token), which is exactly what lets `into_raw_value() << FRAME_SHIFT` not overflow `usize`.
    #[verifier::type_invariant]
    pub open spec fn inv(&self) -> bool {
        0 <= self@ <= Self::spec_max()
    }
}

} // verus!
