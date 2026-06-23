use vstd::arithmetic::div_mod::{
    lemma_div_is_ordered,
    lemma_fundamental_div_mod,
    lemma_mod_bound,
};
use vstd::arithmetic::mul::lemma_mul_inequality;
use vstd::arithmetic::power2::{
    lemma2_to64,
    lemma_pow2_strictly_increases,
};
use vstd::bits::lemma_usize_shr_is_div;
use vstd::layout::unsigned_int_max_values;

verus! {

// A frame index scaled by the frame size stays within `usize`, so `from_number`'s base-address
// multiply never overflows. With the corrected `FrameNumber::MAX` this follows from
// `FrameNumber::spec_max() <= MAX_ADDRESS / FRAME_SIZE`, hence `spec_max() * FRAME_SIZE <=
// (MAX_ADDRESS / FRAME_SIZE) * FRAME_SIZE <= MAX_ADDRESS == usize::MAX`. Proven in the proving phase.
pub proof fn lemma_from_number_no_overflow(frame: FrameNumber)
    requires
        spec_frame_raw_value(frame) <= spec_max_frame_number(),
    ensures
        spec_frame_raw_value(frame) * spec_page_size() <= usize::MAX as int,
{
    let raw: int = frame@;
    let s: int = spec_page_size();
    let m: int = usize::MAX as int;

    assert(s == mem::FRAME_SIZE as int);
    assert(m == mem::MAX_ADDRESS as int);

    // The corrected `spec_max()` is either `MAX_ADDRESS / FRAME_SIZE` or that minus one, so in
    // either case `spec_max() <= m / s`. Only this `<=` direction is needed to rule out overflow.
    assert(FrameNumber::spec_max() <= m / s);

    // `(m / s) * s <= m`, since `m == s * (m / s) + (m % s)` with `0 <= m % s`.
    lemma_mod_bound(m, s);
    lemma_fundamental_div_mod(m, s);
    lemma_mul_inequality(raw, m / s, s);
    assert(raw * s <= m) by (nonlinear_arith)
        requires
            s > 0,
            raw * s <= (m / s) * s,
            m == s * (m / s) + (m % s),
            0 <= m % s,
    {
    }
}

// Shifting the raw address right by `FRAME_SHIFT` yields the frame index `self@ / FRAME_SIZE`,
// which always fits a `FrameNumber`. The shift-equals-divide step uses `FRAME_SIZE == 2^FRAME_SHIFT`;
// the bound uses only that the raw address is a `usize` (`raw_addr <= usize::MAX == MAX_ADDRESS`),
// together with the corrected `FrameNumber::spec_max() == MAX_ADDRESS / FRAME_SIZE`. No `inv()` is
// needed, which is exactly what makes `into_frame_number` total. Proven in the proving phase.
pub proof fn lemma_frame_index(
    addr: PhysicalAddress,
    raw_addr: usize,
    shift: usize,
    frame_number: usize,
)
    requires
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
    if shift < 12 {
        lemma_pow2_strictly_increases(shift as nat, 12);
        assert(false);
    }
    if shift > 12 {
        lemma_pow2_strictly_increases(12, shift as nat);
        assert(false);
    }

    // `raw_addr >> shift == raw_addr / 2^shift == raw_addr / spec_page_size()`.
    lemma_usize_shr_is_div(raw_addr, shift);

    // Bridge the `nat` division from the shift lemma to the `int` division in
    // `spec_frame_number`.
    assert(frame_number as nat == raw_addr as nat / pow2(shift as nat));

    // The frame index is bounded by the maximum frame number. The raw address is a `usize`, so
    // `raw_addr <= usize::MAX == MAX_ADDRESS`; dividing both sides by the (positive) page size and
    // using `spec_max_frame_number() == MAX_ADDRESS / FRAME_SIZE` (the corrected formula, since
    // `MAX_ADDRESS % FRAME_SIZE == FRAME_SIZE - 1` for a power-of-two frame size) gives the bound.
    let s: int = spec_page_size();
    let m: int = mem::MAX_ADDRESS as int;
    assert(m == usize::MAX as int);
    // `usize::MAX % 4096 == 4095` for any `usize` width (32 or 64): `usize::MAX == 2^BITS - 1` and
    // `4096 == 2^12` divides `2^BITS`, so `MAX_ADDRESS` is the last byte of its frame and the
    // corrected `spec_max()` selects the `MAX_ADDRESS / FRAME_SIZE` branch, equal to `m / s`.
    unsigned_int_max_values();
    assert(usize::BITS == 32 || usize::BITS == 64);
    if usize::BITS == 32 {
        assert(pow2(32) == 0x1_0000_0000) by (compute);
        assert(m == 0xFFFF_FFFF);
        assert((0xFFFF_FFFF as int) % 4096 == 4095) by (compute);
    } else {
        assert(pow2(64) == 0x1_0000_0000_0000_0000) by (compute);
        assert(m == 0xFFFF_FFFF_FFFF_FFFF);
        assert((0xFFFF_FFFF_FFFF_FFFF as int) % 4096 == 4095) by (compute);
    }
    assert(m % s == s - 1);
    assert(spec_max_frame_number() == m / s);
    lemma_div_is_ordered(raw_addr as int, m, s);
}

} // verus!
