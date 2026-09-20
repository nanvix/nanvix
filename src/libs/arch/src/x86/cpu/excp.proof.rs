// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
verus! {

impl ErrorCode {
    /// The actual CPU error-code word consumed by the page-fault contract adapter.
    pub closed spec fn spec_raw(self) -> u32 {
        self.0
    }
}

proof fn lemma_fault_masks()
    ensures
        (1u32 << 0) == 1u32,
        (1u32 << 1) == 2u32,
        (1u32 << 2) == 4u32,
{
    assert((1u32 << 0) == 1u32) by (bit_vector);
    assert((1u32 << 1) == 2u32) by (bit_vector);
    assert((1u32 << 2) == 4u32) by (bit_vector);
}

} // verus!
