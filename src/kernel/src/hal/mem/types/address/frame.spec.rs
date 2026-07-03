verus! {

/// The architectural page size.
pub open spec fn spec_page_size() -> int {
    ::arch::mem::PAGE_SIZE as int
}

impl View for FrameAddress
{
    type V = int;

    closed spec fn view(&self) -> int
    {
        self.0@
    }
}

impl FrameAddress {
    pub open spec fn inv(&self) -> bool
    {
        self@ % spec_page_size() == 0
    }
}

}
