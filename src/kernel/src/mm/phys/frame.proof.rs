verus! {

use super::UpoolView;
use crate::hal::mem::spec_page_size;

impl View for Inner {
    type V = UpoolView;

    uninterp spec fn view(&self) -> UpoolView;
}

impl Inner {
    pub closed spec fn internal_inv(&self) -> bool
    {
        true
    }
}

}
