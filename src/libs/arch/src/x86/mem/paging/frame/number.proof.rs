verus! {

use vstd::arithmetic::div_mod::lemma_div_is_ordered;
use vstd::arithmetic::div_mod::lemma_fundamental_div_mod_converse_mod;
use vstd::arithmetic::power2::lemma2_to64;
use vstd::arithmetic::power2::lemma_pow2_adds;
use vstd::arithmetic::power2::lemma_pow2_pos;
use vstd::arithmetic::power2::pow2;
use vstd::bits::lemma_usize_shr_is_div;
use vstd::layout::unsigned_int_max_values;

impl FrameNumber {
    // Divisibility facts backing the `From<PhysicalAddress>` conversion: shifting a raw address
    // right by `FRAME_SHIFT` is exactly integer division by `FRAME_SIZE`, and the quotient of any
    // representable `usize` address is a valid frame index (`<= spec_max()`), so the conversion's
    // `from_raw_value(..).unwrap()` never panics.
    pub proof fn lemma_raw_shr_frame_shift(raw: usize)
        ensures
            (raw >> (mem::FRAME_SHIFT as usize)) as int == (raw as int) / (mem::FRAME_SIZE as int),
            (raw >> (mem::FRAME_SHIFT as usize)) as int <= Self::spec_max(),
    {
        // `raw >> FRAME_SHIFT == raw / 2^FRAME_SHIFT` and `2^FRAME_SHIFT == FRAME_SIZE`, which
        // discharges the first ensures.
        lemma_usize_shr_is_div(raw, mem::FRAME_SHIFT as usize);
        lemma2_to64();
        assert(pow2(mem::FRAME_SHIFT as nat) == mem::FRAME_SIZE as nat);
        // `usize::MAX` is the last byte of its frame, so `spec_max() == MAX_ADDRESS / FRAME_SIZE`.
        Self::lemma_usize_max_frame_boundary();
        // Division preserves order, so `raw / FRAME_SIZE <= MAX_ADDRESS / FRAME_SIZE == spec_max()`.
        lemma_div_is_ordered(raw as int, mem::MAX_ADDRESS as int, mem::FRAME_SIZE as int);
    }

    // `usize::MAX` is always the last addressable byte of its frame, independent of the (32- or
    // 64-bit) word size: `usize::MAX == 2^BITS - 1` and `2^BITS == FRAME_SIZE * 2^(BITS - FRAME_SHIFT)`,
    // hence `usize::MAX % FRAME_SIZE == FRAME_SIZE - 1`. This resolves the `spec_max()` branch to
    // `MAX_ADDRESS / FRAME_SIZE` without depending on `bit_vector` (which rejects the symbolic word
    // width).
    proof fn lemma_usize_max_frame_boundary()
        ensures
            (mem::MAX_ADDRESS as int) % (mem::FRAME_SIZE as int) == (mem::FRAME_SIZE as int) - 1,
    {
        let bits: nat = usize::BITS as nat;
        let shift: nat = mem::FRAME_SHIFT as nat;
        assert(bits == 32 || bits == 64);
        assert(shift == 12);
        assert(mem::FRAME_SIZE as int == 4096);
        // `usize::MAX == 2^BITS - 1` and `2^BITS == 2^FRAME_SHIFT * 2^(BITS - FRAME_SHIFT)`.
        unsigned_int_max_values();
        assert(mem::MAX_ADDRESS as int == pow2(bits) - 1);
        lemma2_to64();
        assert(pow2(shift) == 4096);
        lemma_pow2_pos((bits - shift) as nat);
        lemma_pow2_adds(shift, (bits - shift) as nat);
        assert(shift + (bits - shift) as nat == bits);
        let k: int = pow2((bits - shift) as nat) as int;
        assert(k >= 1);
        assert(pow2(bits) == pow2(shift) * pow2((bits - shift) as nat));
        assert(pow2(bits) == 4096 * k);
        // `usize::MAX == 4096 * k - 1 == (k - 1) * FRAME_SIZE + (FRAME_SIZE - 1)`.
        assert(mem::MAX_ADDRESS as int == 4096 * k - 1);
        assert((mem::MAX_ADDRESS as int)
            == (k - 1) * (mem::FRAME_SIZE as int) + ((mem::FRAME_SIZE as int) - 1));
        lemma_fundamental_div_mod_converse_mod(
            mem::MAX_ADDRESS as int,
            mem::FRAME_SIZE as int,
            k - 1,
            (mem::FRAME_SIZE as int) - 1,
        );
    }
}

} // verus!
