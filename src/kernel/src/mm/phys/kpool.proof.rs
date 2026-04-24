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
        true
    }
}

} // end verus!
