verus! {

use vstd::bits::lemma_usize_shl_is_mul;
use vstd::arithmetic::power2::{pow2, lemma2_to64};
use vstd::arithmetic::div_mod::{
    lemma_fundamental_div_mod,
    lemma_mod_bound,
    lemma_mod_multiples_basic,
};
use vstd::arithmetic::mul::lemma_mul_inequality;

// The physical base address of a frame is its index shifted left by `FRAME_SHIFT`, which equals the
// index multiplied by `FRAME_SIZE` and is `FRAME_SIZE`-aligned. The frame-index bound supplied by
// `FrameNumber`'s type invariant (`raw <= FrameNumber::spec_max() <= MAX_ADDRESS / FRAME_SIZE`)
// keeps the product within `usize`, so the shift is overflow-free.
pub proof fn lemma_frame_address(raw: usize)
    requires
        0 <= raw as int <= FrameNumber::spec_max(),
    ensures
        (raw << crate::mem::FRAME_SHIFT) as int == raw as int * (crate::mem::FRAME_SIZE as int),
        (raw << crate::mem::FRAME_SHIFT) as int % (crate::mem::FRAME_SIZE as int) == 0,
{
    let s: int = crate::mem::FRAME_SIZE as int;
    let m: int = crate::mem::MAX_ADDRESS as int;

    // `pow2(FRAME_SHIFT) == FRAME_SIZE` (i.e. `pow2(12) == 4096`).
    lemma2_to64();
    assert(pow2(crate::mem::FRAME_SHIFT as nat) == crate::mem::FRAME_SIZE);

    // `spec_max()` is either `m / s` or `m / s - 1`, hence `<= m / s`.
    assert(FrameNumber::spec_max() <= m / s);
    assert(raw as int <= m / s);

    // The product fits in `usize`: `raw <= m / s`, and `(m / s) * s <= m`.
    lemma_mod_bound(m, s);
    lemma_fundamental_div_mod(m, s);
    lemma_mul_inequality(raw as int, m / s, s);
    assert(raw as int * s <= m) by (nonlinear_arith)
        requires
            s > 0,
            0 <= raw as int,
            raw as int * s <= (m / s) * s,
            m == s * (m / s) + (m % s),
            0 <= m % s,
    {
    }

    // `(raw << FRAME_SHIFT) == raw * pow2(FRAME_SHIFT) == raw * FRAME_SIZE`.
    lemma_usize_shl_is_mul(raw, crate::mem::FRAME_SHIFT);

    // `(raw * FRAME_SIZE) % FRAME_SIZE == 0`.
    lemma_mod_multiples_basic(raw as int, s);
}

} // verus!
