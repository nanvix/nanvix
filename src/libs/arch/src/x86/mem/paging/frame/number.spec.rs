verus! {

//==================================================================================================
// View
//==================================================================================================

impl View for FrameNumber {
    type V = int;

    // `closed`: callers reference `self@`, but the mapping to the inner `usize` field is hidden.
    // The abstract value is "the frame index as int".
    closed spec fn view(&self) -> int {
        self.0 as int
    }
}

//==================================================================================================
// Abstract bound + type invariant
//==================================================================================================

impl FrameNumber {
    // Interpreted bound: the largest representable frame index, mirroring the exec constant
    // `FrameNumber::MAX`. Defined (not `uninterp`) so the binding to `MAX` is discharged by
    // verification rather than assumed. The `nat` result makes the bound non-negative for
    // callers (e.g. constructing `FrameNumber::NULL`).
    //
    // The formula is the number of the highest frame fully contained in `[0, MAX_ADDRESS]`, i.e.
    // `(MAX_ADDRESS + 1) / FRAME_SIZE - 1` written to avoid `usize` overflow. It must stay byte-for-
    // byte identical to `FrameNumber::MAX` so `from_raw_value`'s exec/spec equivalence is discharged.
    pub open spec fn spec_max() -> nat {
        let max_addr: int = mem::MAX_ADDRESS as int;
        let frame_size: int = mem::FRAME_SIZE as int;
        (if max_addr % frame_size == frame_size - 1 {
            max_addr / frame_size
        } else {
            max_addr / frame_size - 1
        }) as nat
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
