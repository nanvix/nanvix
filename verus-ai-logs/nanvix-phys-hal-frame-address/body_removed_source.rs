// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("frame.spec.rs");
#[cfg(verus_keep_ghost)]
include!("frame.proof.rs");

use crate::hal::mem::types::address::{
    Address,
    PageAddress,
    PageAligned,
    PhysicalAddress,
};
use ::arch::mem::paging::FrameNumber;
use ::sys::error::Error;
use ::vstd::prelude::*;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a frame address.
///
#[verus_verify(external_derive)]
#[derive(Clone, Copy)]
pub struct FrameAddress(PageAligned<PhysicalAddress>);

#[cfg(verus_keep_ghost)]
verus! {

pub uninterp spec fn spec_page_size() -> int;

pub assume_specification[ ::arch::mem::PAGE_SIZE ] -> (result: usize)
    ensures
        result == spec_page_size(),
;

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

//==================================================================================================
// Implementations
//==================================================================================================

impl FrameAddress {
    pub fn new(address: PageAligned<PhysicalAddress>) -> Self { ... }

    pub fn into_physical_address(self) -> PageAligned<PhysicalAddress> { ... }

    pub fn into_page_address(self) -> PageAddress { ... }

    pub fn from_frame_number(frame_number: FrameNumber) -> Result<Self, Error> { ... }

    pub fn into_frame_number(self) -> FrameNumber { ... }
}

// Dependency contract for the manager layer: raw-value conversions of a frame address.
#[verus_verify]
impl FrameAddress {
    // Succeeds only for page-aligned inputs, so the resulting frame address satisfies `inv()`.
    // `external_body` until the address layer is verified.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        ensures
            match result {
                Ok(fa) => fa.inv(),
                Err(_) => true,
            },
    )]
    pub fn from_raw_value(raw_addr: usize) -> Result<Self, Error> { ... }

    ///
    /// # Description
    ///
    /// Converts a [`FrameAddress`] into a raw value.
    ///
    /// # Returns
    ///
    /// The raw value of the target [`FrameAddress`].
    ///
    // Dependency contract: the raw value is the abstract frame address. `external_body` until the
    // address layer is verified.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        ensures
            result as int == self@,
    )]
    pub fn into_raw_value(self) -> usize { ... }
}

impl core::fmt::Debug for FrameAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { ... }
}

impl PartialEq for FrameAddress {
    fn eq(&self, other: &Self) -> bool { ... }
}
