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
