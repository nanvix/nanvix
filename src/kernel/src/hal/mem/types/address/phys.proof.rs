verus! {

use vstd::arithmetic::div_mod::{
    lemma_fundamental_div_mod,
    lemma_mod_bound,
};
use vstd::arithmetic::mul::lemma_mul_inequality;
use vstd::arithmetic::power2::lemma_pow2_pos;
use vstd::bits::lemma_usize_shr_is_div;

// A frame index scaled by the frame size stays within `usize`, so `from_number`'s base-address
// multiply never overflows. This follows from `FrameNumber::MAX == MAX_ADDRESS / FRAME_SIZE - 1`,
// hence `(MAX + 1) * FRAME_SIZE == MAX_ADDRESS <= usize::MAX`. The frame-index bound is supplied by
// the caller (via `FrameNumber`'s type invariant), mirroring `arch`'s `lemma_frame_address`.
pub proof fn lemma_from_number_no_overflow(frame: FrameNumber)
    requires
        0 <= spec_frame_raw_value(frame) <= spec_max_frame_number(),
    ensures
        spec_frame_raw_value(frame) * spec_page_size() <= usize::MAX as int,
{
    let raw: int = frame@;
    let s: int = spec_page_size();
    let m: int = ::arch::mem::MAX_ADDRESS as int;

    // `spec_page_size() == FRAME_SIZE` and `MAX_ADDRESS == usize::MAX` (transparent constants).
    assert(s == ::arch::mem::FRAME_SIZE as int);
    assert(m == usize::MAX as int);

    // The supplied bound, re-expressed over `int`: `raw <= spec_max() == m / s - 1`.
    assert(raw <= m / s - 1);

    // The product fits in `usize`: `raw <= m / s - 1`, and `(m / s) * s <= m`.
    lemma_mod_bound(m, s);
    lemma_fundamental_div_mod(m, s);
    lemma_mul_inequality(raw, m / s - 1, s);
    assert(raw * s <= m) by (nonlinear_arith)
        requires
            s > 0,
            0 <= raw,
            raw * s <= (m / s - 1) * s,
            m == s * (m / s) + (m % s),
            0 <= m % s,
    {
    }
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
    // `raw_addr >> shift == raw_addr / pow2(shift) == raw_addr / FRAME_SIZE`.
    lemma_usize_shr_is_div(raw_addr, shift);
    lemma_pow2_pos(shift as nat);

    // `pow2(shift) == spec_page_size()`, so the right shift computes `spec_frame_number(addr@)`.
    assert(frame_number as int == spec_frame_number(addr@));

    // `addr.inv()` is exactly `spec_frame_number(addr@) <= spec_max_frame_number()`.
    assert(frame_number as int <= spec_max_frame_number());
}

} // verus!
