use vstd::arithmetic::div_mod::{
    lemma_fundamental_div_mod,
    lemma_mod_multiples_basic,
};

verus! {

// =================================================================================================
// Proof obligations for `FrameAddress`'s frame-number conversions.
//
// The facts below are elementary divisibility properties of `spec_page_size()`, which is the
// `#[verus_verify]` constant `arch::mem::PAGE_SIZE == 4096` and therefore strictly positive.
// =================================================================================================

// `frame_index * PAGE_SIZE` is a multiple of `PAGE_SIZE`, hence page-aligned. Used by
// `from_frame_number` to discharge `PageAligned::from_address`'s alignment success condition.
pub proof fn lemma_frame_base_aligned(frame: FrameNumber)
    ensures
        frame@ * spec_page_size() % spec_page_size() == 0,
{
    // `frame@ * spec_page_size()` is a multiple of `spec_page_size()`, so its remainder mod
    // `spec_page_size()` is zero.
    lemma_mod_multiples_basic(frame@, spec_page_size());
}

// For a page-aligned address, dividing by `PAGE_SIZE` then multiplying recovers the address
// exactly. Used by `into_frame_number` to relate the recovered frame number back to `self@`.
pub proof fn lemma_aligned_div_mul(addr: int)
    requires
        addr % spec_page_size() == 0,
    ensures
        (addr / spec_page_size()) * spec_page_size() == addr,
{
    // `addr == s * (addr / s) + (addr % s)` with `addr % s == 0`, so `(addr / s) * s == addr`.
    lemma_fundamental_div_mod(addr, spec_page_size());
}

} // verus!
