// Spec helpers for `FrameAddress` are shared with the `phys` sibling module: a frame's abstract
// state is the physical address (`int`), its frame number is `self@ / spec_page_size()`, and its
// base address is `frame_index * spec_page_size()`.
use crate::hal::mem::types::address::phys::{
    spec_frame_number,
    spec_from_number,
    spec_frame_raw_value,
};

// The former `assume_specification` for `<PhysicalAddress as ::sys::mm::Address>::from_raw_value`
// was removed: it supplied a trusted, unverified contract to a kernel-internal (intra-crate)
// callee that is not sanctioned in `tcb-allowed.md`. Its sole consumer, `FrameAddress::from_raw_value`,
// is instead carried as a TCB-listed `#[verus_verify(external_body)]` (see `frame.rs`), so no
// intra-crate trust hole remains here.

verus! {

// The architectural page size, delegating to the `arch` crate's verified `PAGE_SIZE` constant.
// Formerly an `uninterp spec fn` paired with a placeholder `assume_specification[PAGE_SIZE]`; now
// that `arch` carries a real verified spec for `PAGE_SIZE`, that placeholder is superseded and this
// definition names the same concrete value the proofs already relied on.
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

