verus! {

// A frame index scaled by the frame size stays within `usize`, so `from_number`'s base-address
// multiply never overflows. This follows from `FrameNumber::MAX == MAX_ADDRESS / FRAME_SIZE - 1`,
// hence `(MAX + 1) * FRAME_SIZE == MAX_ADDRESS <= usize::MAX`. Proven in the proving phase.
pub proof fn lemma_from_number_no_overflow(frame: FrameNumber)
    ensures
        spec_frame_raw_value(frame) * spec_page_size() <= usize::MAX as int,
{
    admit();
}

// Under `inv()`, shifting the raw address right by `FRAME_SHIFT` yields the frame index
// `self@ / FRAME_SIZE`, which fits a `FrameNumber`. The shift-equals-divide step uses
// `FRAME_SIZE == 2^FRAME_SHIFT`. Proven in the proving phase.
pub proof fn lemma_frame_index(
    addr: PhysicalAddress,
    raw_addr: usize,
    shift: usize,
    frame_number: usize,
)
    requires
        addr.inv(),
        raw_addr as int == addr@,
        shift < 64,
        spec_page_size() == pow2(shift as nat),
        frame_number == raw_addr >> shift,
    ensures
        frame_number as int == spec_frame_number(addr@),
        frame_number as int <= spec_max_frame_number(),
{
    admit();
}

} // verus!
