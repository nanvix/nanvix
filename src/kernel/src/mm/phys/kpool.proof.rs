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
}

} // end verus!
