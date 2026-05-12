verus! {

use super::KpoolView;

impl View for Inner {
    type V = KpoolView;

    closed spec fn view(&self) -> KpoolView
    {
        KpoolView{
            start: self.base@,
            num_pages: self.bitmap@.num_bits as int,
            used_page_indices: self.bitmap@.set_bits,
        }
    }
}

impl Inner {
    pub closed spec fn internal_inv(&self) -> bool
    {
        &&& self.base.inv()
        &&& self.bitmap.inv()
        &&& spec_page_size() > 0
        &&& self.base@ >= 0
        &&& self.base@ + self.bitmap@.num_bits * spec_page_size() <= usize::MAX as int + 1
        &&& self.bitmap@.num_bits < u32::MAX as int
    }

    /// Reveals all conjuncts of internal_inv for use in proof blocks.
    proof fn lemma_internal_inv(&self)
        requires self.internal_inv(),
        ensures
            self.base.inv(),
            self.bitmap.inv(),
            spec_page_size() > 0,
            self.base@ >= 0,
            self.base@ + self.bitmap@.num_bits * spec_page_size() <= usize::MAX as int + 1,
            self.bitmap@.num_bits < u32::MAX as int,
    {}

    /// Establish internal_inv from its individual conjuncts.
    proof fn lemma_internal_inv_intro(&self)
        requires
            self.base.inv(),
            self.bitmap.inv(),
            spec_page_size() > 0,
            self.base@ >= 0,
            self.base@ + self.bitmap@.num_bits * spec_page_size() <= usize::MAX as int + 1,
            self.bitmap@.num_bits < u32::MAX as int,
        ensures self.internal_inv(),
    {}
}

/// Proves index * page_size doesn't overflow and base + index * page_size ≤ usize::MAX.
#[verifier::spinoff_prover]
proof fn lemma_index_mul_no_overflow(base: int, index: int, num_bits: int, ps: int)
    requires
        ps > 0,
        base >= 0,
        0 <= index < num_bits,
        base + num_bits * ps <= usize::MAX as int + 1,
    ensures
        index * ps < num_bits * ps,
        index * ps <= usize::MAX as int,
        base + index * ps <= usize::MAX as int,
{
    vstd::arithmetic::mul::lemma_mul_strict_inequality(index, num_bits, ps);
}

/// Proves (base + k * page_size) % page_size == 0 when base is page-aligned.
#[verifier::spinoff_prover]
proof fn lemma_addr_page_aligned(base: int, k: int, ps: int)
    requires
        ps > 0,
        base % ps == 0,
    ensures
        (base + k * ps) % ps == 0,
{
    vstd::arithmetic::div_mod::lemma_mod_multiples_vanish(k, base, ps);
}

/// Proves (k * page_size) / page_size == k.
#[verifier::spinoff_prover]
proof fn lemma_page_index_eq(k: int, ps: int)
    requires
        ps > 0,
        k >= 0,
    ensures
        (k * ps) / ps == k,
{
    vstd::arithmetic::div_mod::lemma_div_by_multiple(k, ps);
}

/// Proves a / d < 0 when a < 0 and d > 0.
#[verifier::spinoff_prover]
proof fn lemma_negative_div(a: int, d: int)
    requires
        a < 0,
        d > 0,
    ensures
        a / d < 0,
{
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(a, d);
    let q: int = a / d;
    let r: int = a % d;
    assert(a == d * q + r);
    assert(0 <= r);
    if q >= 0 {
        vstd::arithmetic::mul::lemma_mul_inequality(0, q, d);
    }
}

/// Proves base_addr + i * ps == base + (index + i) * ps (distributivity).
#[verifier::spinoff_prover]
proof fn lemma_offset_sum(base: int, index: int, i: int, ps: int)
    requires
        ps > 0,
    ensures
        (index + i) * ps == index * ps + i * ps,
        base + (index + i) * ps == base + index * ps + i * ps,
{
    vstd::arithmetic::mul::lemma_mul_is_distributive_add_other_way(ps, index, i);
}

} // end verus!
