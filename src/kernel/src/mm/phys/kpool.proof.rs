verus! {

use super::KpoolView;
use crate::hal::mem::spec_page_size;

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

} // end verus!
