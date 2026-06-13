// Spec helpers for `FrameAddress` are shared with the `phys` sibling module: a frame's abstract
// state is the physical address (`int`), its frame number is `self@ / spec_page_size()`, and its
// base address is `frame_index * spec_page_size()`.
use crate::hal::mem::types::address::phys::{
    spec_frame_number,
    spec_from_number,
    spec_frame_raw_value,
    spec_max_frame_number,
};

verus! {

// =================================================================================================
// Dependency contract for the not-yet-verified `Address` implementation of `PhysicalAddress`.
//
// `PhysicalAddress::from_raw_value` validates that `value` denotes a physical address; on success
// the abstract address is exactly the raw input. This placeholder is removed once `phys`'s
// `Address` impl carries its own `#[verus_spec]`.
// =================================================================================================
pub assume_specification[ <PhysicalAddress as ::sys::mm::Address>::from_raw_value ](value: usize)
    -> (result: Result<PhysicalAddress, ::sys::error::Error>)
    ensures
        match result {
            Ok(pa) => pa@ == value as int,
            Err(_) => true,
        },
;

} // verus!
