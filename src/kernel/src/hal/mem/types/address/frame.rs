// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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
    pub fn new(address: PageAligned<PhysicalAddress>) -> Self {
        Self(address)
    }

    pub fn into_physical_address(self) -> PageAligned<PhysicalAddress> {
        self.0
    }

    pub fn into_page_address(self) -> PageAddress {
        PageAddress::new(PageAligned::into_virtual_address(self.0))
    }

    pub fn from_frame_number(frame_number: FrameNumber) -> Result<Self, Error> {
        Ok(Self(PageAligned::from_address(PhysicalAddress::from_number(frame_number))?))
    }

    pub fn into_frame_number(self) -> FrameNumber {
        self.0.into_frame_number()
    }
}

// Dependency contract for the manager layer: raw-value conversions of a frame address.
#[cfg_attr(verus_keep_ghost, verus_verify)]
impl FrameAddress {
    // Succeeds only for page-aligned inputs, so the resulting frame address satisfies `inv()`.
    // `external_body` until the address layer is verified.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        ensures
            match result {
                Ok(fa) => fa.inv(),
                Err(_) => true,
            },
    ))]
    pub fn from_raw_value(raw_addr: usize) -> Result<Self, Error> {
        Ok(Self(PageAligned::from_address(PhysicalAddress::from_raw_value(raw_addr)?)?))
    }

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
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        ensures
            result as int == self@,
    ))]
    pub fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }
}

impl core::fmt::Debug for FrameAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "FrameAddress({:#010x})", self.into_raw_value())
    }
}

impl PartialEq for FrameAddress {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
