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

//==================================================================================================
// Implementations
//==================================================================================================

impl FrameAddress {
    pub fn new(address: PageAligned<PhysicalAddress>) -> Self { ... }

    pub fn into_physical_address(self) -> PageAligned<PhysicalAddress> { ... }

    pub fn into_page_address(self) -> PageAddress { ... }
}

// Raw-value and frame-number conversions of a frame address. The frame address denotes a single
// page-aligned physical frame; these functions expose its two equivalent identities (raw physical
// address and frame number) and the lossless, mutually-inverse mappings between them.
#[verus_verify]
impl FrameAddress {
    // Succeeds only for page-aligned inputs, so the resulting frame address satisfies `inv()` and
    // its abstract address equals the raw input. `external_body` (TCB-sanctioned per
    // `tcb-allowed.md`) until the intra-crate `PhysicalAddress` `Address` impl carries its own
    // verified `#[verus_spec]`; the strengthened contract below is preserved verbatim so callers
    // keep the full `fa@ == raw_addr` guarantee.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        ensures
            match result {
                Ok(fa) => fa.inv() && fa@ == raw_addr as int,
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
    // Dependency contract: the raw value is the abstract frame address. Verified against the
    // `PageAligned::into_raw_value` dependency contract.
    #[verus_spec(result =>
        ensures
            result as int == self@,
    )]
    pub fn into_raw_value(self) -> usize { ... }

    // Constructs a frame address from a frame number. The frame's base address is
    // `frame_number * PAGE_SIZE`, page-aligned by construction, so the call always succeeds and the
    // result satisfies `inv()`.
    #[verus_spec(result =>
        ensures
            result is Ok,
            (result->Ok_0).inv(),
            (result->Ok_0)@ == spec_from_number(spec_frame_raw_value(frame_number)),
    )]
    pub fn from_frame_number(frame_number: FrameNumber) -> Result<Self, Error> { ... }

    // Recovers the frame number of a frame address (`self@ / PAGE_SIZE`). Requires only that the
    // address is page-aligned (so the round-trip `from_number(into_frame_number(self)) == self`
    // holds); representability is automatic because the underlying
    // `PhysicalAddress::into_frame_number` is total. The result is the exact inverse of
    // `from_frame_number`.
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            spec_frame_raw_value(result) == spec_frame_number(self@),
            spec_from_number(spec_frame_raw_value(result)) == self@,
    )]
    pub fn into_frame_number(self) -> FrameNumber { ... }
}

impl core::fmt::Debug for FrameAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { ... }
}

impl PartialEq for FrameAddress {
    fn eq(&self, other: &Self) -> bool { ... }
}
