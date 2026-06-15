use vstd::arithmetic::div_mod::{
    lemma_fundamental_div_mod,
    lemma_mod_bound,
};
use vstd::arithmetic::mul::lemma_mul_inequality;
use vstd::arithmetic::power2::{
    lemma2_to64,
    lemma_pow2_strictly_increases,
};
use vstd::bits::lemma_usize_shr_is_div;

verus! {

// A frame index scaled by the frame size stays within `usize`, so `from_number`'s base-address
// multiply never overflows. This follows from `FrameNumber::MAX == MAX_ADDRESS / FRAME_SIZE - 1`,
// hence `(MAX + 1) * FRAME_SIZE == MAX_ADDRESS <= usize::MAX`. Proven in the proving phase.
pub proof fn lemma_from_number_no_overflow(frame: FrameNumber)
    requires
        spec_frame_raw_value(frame) <= spec_max_frame_number(),
    ensures
        spec_frame_raw_value(frame) * spec_page_size() <= usize::MAX as int,
{
    let raw: int = frame@;
    let s: int = spec_page_size();
    let m: int = usize::MAX as int;

    // `spec_page_size()` is `PAGE_SIZE == FRAME_SIZE` and
    // `spec_max() == MAX_ADDRESS / FRAME_SIZE - 1`, with `MAX_ADDRESS == usize::MAX`.
    assert(s == mem::FRAME_SIZE as int);
    assert(FrameNumber::spec_max() == (m / s - 1) as nat);
    assert(raw <= m / s - 1);

    // `(m / s) * s <= m`, since `m == s * (m / s) + (m % s)` with `0 <= m % s`.
    lemma_mod_bound(m, s);
    lemma_fundamental_div_mod(m, s);
    lemma_mul_inequality(raw, m / s - 1, s);
    assert(raw * s <= m) by (nonlinear_arith)
        requires
            s > 0,
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
    // From `spec_page_size() == pow2(shift)` and `PAGE_SIZE == 4096 == pow2(12)`, the shift is
    // exactly `12`, hence below `usize::BITS` on every supported target. `pow2` is strictly
    // increasing, so this is the only solution.
    lemma2_to64();
    assert(spec_page_size() == 4096);
    assert(pow2(shift as nat) == 4096);
    if shift < 12 {
        lemma_pow2_strictly_increases(shift as nat, 12);
        assert(false);
    }
    if shift > 12 {
        lemma_pow2_strictly_increases(12, shift as nat);
        assert(false);
    }
    assert(shift == 12);

    // `raw_addr >> shift == raw_addr / 2^shift == raw_addr / spec_page_size()`.
    lemma_usize_shr_is_div(raw_addr, shift);

    // Bridge the `nat` division from the shift lemma to the `int` division in
    // `spec_frame_number`.
    assert(frame_number as nat == raw_addr as nat / pow2(shift as nat));
    assert(frame_number as int == spec_frame_number(addr@));

    // The frame index bound carries over from `addr.inv()`.
    assert(frame_number as int <= spec_max_frame_number());
}

} // verus!
