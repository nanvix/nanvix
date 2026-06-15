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

// The `View`/`inv` for `FrameAddress`, the `spec_page_size()` projection, and
// the `hal::mem` trust-boundary `assume_specification`s are verification
// material; they live in `frame.spec.rs` (included only under
// `cfg(verus_keep_ghost)`) so that no verification-only construct is cfg-gated
// inside this exec source file.

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

    // On `Ok`, the produced frame address denotes the base of `frame_number`
    // (`fa@ == frame_number * PAGE_SIZE`) and satisfies the type invariant
    // (page-aligned and frame-representable). Construction never fails: the base
    // address of a representable frame number is page-aligned, so the internal
    // `PageAligned::from_address` alignment check always succeeds (`result is Ok`).
    #[verus_spec(result =>
        ensures
            result is Ok,
            result matches Ok(fa) ==> fa@ == spec_from_number(spec_frame_raw_value(frame_number))
                && fa.inv(),
    )]
    pub fn from_frame_number(frame_number: FrameNumber) -> Result<Self, Error> {
        // VERUS DEVIATION (pre-approved: `f(complex_expr)` -> `let x = complex_expr; f(x)`):
        // the physical address is bound to a local so the bridge lemma can relate
        // its universal `spec_addr` projection (consumed by `PageAligned::from_address`)
        // to its `View` (guaranteed by `PhysicalAddress::from_number`).
        let physical_address: PhysicalAddress = PhysicalAddress::from_number(frame_number);
        proof {
            lemma_phys_view_is_spec_addr(physical_address);
        }
        Ok(Self(PageAligned::from_address(physical_address)?))
    }

    // Total projection: yields the frame index this address belongs to
    // (`self@ / PAGE_SIZE`). The receiver's invariant guarantees the index is
    // representable, so the internal `unwrap()` never panics.
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            spec_frame_raw_value(result) == spec_frame_number(self@),
    )]
    pub fn into_frame_number(self) -> FrameNumber {
        self.0.into_frame_number()
    }

    // On `Ok`, the produced frame address is the newtype identity of `raw_addr`
    // (`fa@ == raw_addr`) and satisfies the type invariant (page-aligned and
    // frame-representable). On `Err`, `raw_addr` was not a valid (in-range,
    // page-aligned) physical address; the caller propagates it with `?`.
    #[verus_spec(result =>
        ensures
            result matches Ok(fa) ==> fa@ == raw_addr as int && fa.inv(),
    )]
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
    // Pure newtype-identity projection: the raw value coincides with the
    // address's abstract view (`result as int == self@`). Body-verified directly
    // against the inner `PageAligned` identity (`into_raw_value` returns exactly
    // `self.0@ == self@`).
    #[verus_spec(result =>
        ensures
            result as int == self@,
    )]
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
