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

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a frame address.
///
#[derive(Clone, Copy)]
pub struct FrameAddress(PageAligned<PhysicalAddress>);

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
}

// Raw-value and frame-number conversions of a frame address. The frame address denotes a single
// page-aligned physical frame; these functions expose its two equivalent identities (raw physical
// address and frame number) and the lossless, mutually-inverse mappings between them.

impl FrameAddress {
    // Succeeds only for page-aligned inputs, so the resulting frame address satisfies `inv()` and
    // its abstract address equals the raw input. `external_body` (TCB-sanctioned per
    // `tcb-allowed.md`) until the intra-crate `PhysicalAddress` `Address` impl carries its own
    // verified `#[verus_spec]`; the strengthened contract below is preserved verbatim so callers
    // keep the full `fa@ == raw_addr` guarantee.

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
    pub fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }

    // Constructs a frame address from a frame number. The frame's base address is
    // `frame_number * PAGE_SIZE`, page-aligned by construction, so the call always succeeds and the
    // result satisfies `inv()`.

    pub fn from_frame_number(frame_number: FrameNumber) -> Result<Self, Error> {

        Ok(Self(PageAligned::from_address(PhysicalAddress::from_number(frame_number))?))
    }

    // Recovers the frame number of a frame address (`self@ / PAGE_SIZE`). Requires only that the
    // address is page-aligned (so the round-trip `from_number(into_frame_number(self)) == self`
    // holds); representability is automatic because the underlying
    // `PhysicalAddress::into_frame_number` is total. The result is the exact inverse of
    // `from_frame_number`.

    pub fn into_frame_number(self) -> FrameNumber {

        self.0.into_frame_number()
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
