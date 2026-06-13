verus! {

// =================================================================================================
// Proof obligations for `FrameAddress`'s frame-number conversions.
//
// Bodies are `admit()` placeholders during the specification phase; the proving phase discharges
// them (the facts below are elementary divisibility properties of `spec_page_size()`, which is a
// power of two and therefore strictly positive).
// =================================================================================================

// `frame_index * PAGE_SIZE` is a multiple of `PAGE_SIZE`, hence page-aligned. Used by
// `from_frame_number` to discharge `PageAligned::from_address`'s alignment success condition.
pub proof fn lemma_frame_base_aligned(frame: FrameNumber)
    ensures
        spec_from_number(spec_frame_raw_value(frame)) % spec_page_size() == 0,
{
    admit();
}

// For a page-aligned address, dividing by `PAGE_SIZE` then multiplying recovers the address
// exactly. Used by `into_frame_number` to relate the recovered frame number back to `self@`.
pub proof fn lemma_aligned_div_mul(addr: int)
    requires
        addr % spec_page_size() == 0,
    ensures
        spec_from_number(spec_frame_number(addr)) == addr,
{
    admit();
}

} // verus!
