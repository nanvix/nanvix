use crate::hal::mem::spec_page_size;

verus! {

// Success condition for the validating constructor, stated purely on the abstract
// address value. `from_address` validates, it does NOT normalize: success requires the
// *input* to already be page-aligned.
pub open spec fn spec_aligned(addr_view: int) -> bool {
    addr_view % spec_page_size() == 0
}

impl<T: Address> PageAligned<T>
{
    pub open spec fn inv(&self) -> bool
    {
        self@ % spec_page_size() == 0
    }
}

} // verus!
