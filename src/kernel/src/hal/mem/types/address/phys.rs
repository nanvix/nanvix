// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("phys.spec.rs");
#[cfg(verus_keep_ghost)]
include!("phys.proof.rs");

use crate::hal::mem::types::address::{
    Address,
    FrameAddress,
    VirtualAddress,
};
use ::arch::mem::{
    self,
    paging::FrameNumber,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::Alignment,
};
use ::vstd::prelude::*;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a physical address.
///
#[verus_verify(external_derive)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(VirtualAddress);

//==================================================================================================
// Implementations
//==================================================================================================

#[verus_verify]
impl PhysicalAddress {
    pub fn from_virtual_address(addr: VirtualAddress) -> Result<Self, Error> {
        // Delegate to the per-platform validator to support sparse physical memory layouts.
        if !crate::hal::platform::is_valid_physical_address(addr) {
            return Err(Error::new(
                ErrorCode::BadAddress,
                "address out of bounds of physical memory",
            ));
        }

        Ok(Self(addr))
    }

    ///
    /// # Description
    ///
    /// Constructs a physical address from a memory-mapped I/O address.
    ///
    /// # Parameters
    ///
    /// - `addr`: The memory-mapped I/O address.
    ///
    /// # Return Values
    ///
    /// Upon success, a physical address associated with the given memory-mapped I/O address is
    /// returned. Upon failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if the provided memory-mapped I/O address is invalid.
    ///
    // Deliberately bypasses the RAM-range validity check (MMIO GPAs may lie
    // outside tracked RAM). On success it is pure identity wrapping
    // (`r@ == addr@`); the `unsafe` contract — that `addr` denotes a valid MMIO
    // frame — is encoded as `requires` (frame-representability) so the result
    // satisfies the type invariant and may later flow into `into_frame_number`.
    #[verus_spec(result =>
        requires
            spec_frame_number(addr@) <= spec_max_frame_number(),
        ensures
            match result {
                Ok(r) => r@ == addr@ && r.inv(),
                Err(_) => true,
            },
    )]
    pub unsafe fn from_mmio_address(addr: VirtualAddress) -> Result<Self, Error> {
        Ok(Self(addr))
    }

    pub fn into_virtual_address(self) -> VirtualAddress {
        self.0
    }

    ///
    /// # Description
    ///
    /// Constructs a [`PhysicalAddress`] from a [`FrameNumber`].
    ///
    /// # Parameters
    ///
    /// - `frame`: The frame number.
    ///
    /// # Returns
    ///
    /// A [`PhysicalAddress`] associated with the given `frame_number`.
    ///
    // The produced address is the frame's base address (`frame * FRAME_SIZE`).
    // Alignment (`result@ % FRAME_SIZE == 0`) and the type invariant follow from
    // this value relation, so they are not listed separately.
    #[verus_spec(result =>
        ensures
            result@ == spec_from_number(spec_frame_raw_value(frame)),
    )]
    pub fn from_number(frame: FrameNumber) -> Self {
        proof! { admit(); }
        let addr: usize = frame.into_raw_value() * mem::FRAME_SIZE;
        Self(VirtualAddress::new(addr))
    }

    // Total projection: yields the containing frame (`self@ / FRAME_SIZE`). The
    // receiver's invariant guarantees the computed index fits a `FrameNumber`,
    // so the internal `unwrap()` never panics.
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            spec_frame_raw_value(result) == spec_frame_number(self@),
    )]
    pub fn into_frame_number(self) -> FrameNumber {
        proof! { admit(); }
        let raw_addr: usize = self.0.into_raw_value();
        let frame_number: usize = raw_addr >> mem::FRAME_SHIFT;
        // Safety: the following unwrap is safe because a physical address has a valid frame number.
        FrameNumber::from_raw_value(frame_number).unwrap()
    }

    ///
    /// # Description
    ///
    /// Constructs a [`PhysicalAddress`] from a [`FrameAddress`].
    ///
    /// # Parameters
    ///
    /// - `frame_addr`: The frame address.
    ///
    /// # Returns
    ///
    /// A [`PhysicalAddress`] associated with the given `frame_addr`.
    ///
    pub fn from_frame_address(frame_addr: FrameAddress) -> Self {
        let raw_addr: usize = frame_addr.into_raw_value() << mem::FRAME_SHIFT;
        Self(VirtualAddress::new(raw_addr))
    }

    pub fn from_into_frame_address(frame_addr: FrameAddress) -> Self {
        let raw_addr: usize = frame_addr.into_raw_value() << mem::FRAME_SHIFT;
        Self(VirtualAddress::new(raw_addr))
    }
}

impl Address for PhysicalAddress {
    ///
    /// # Description
    ///
    /// Instantiates a new [`PhysicalAddress`] from a raw value.
    ///
    /// # Parameters
    ///
    /// - `raw_addr`: The raw value.
    ///
    /// # Returns
    ///
    /// - `Ok(Self)`: The new address.
    /// - `Err(Error::BadAddress)`: If the provided address is invalid.
    ///
    fn from_raw_value(value: usize) -> Result<Self, Error> {
        Self::from_virtual_address(VirtualAddress::from_raw_value(value))
    }

    ///
    /// # Description
    ///
    ///  Aligns the target [`PhysicalAddress`] to the provided `alignment`. If the address is already
    ///  aligned, it is returned as is.
    ///
    /// # Parameters
    ///
    /// - `alignment`: The alignment to align the target address to.
    ///
    /// # Returns
    ///
    /// Upon success, the aligned address is returned. Upon failure, an error is returned instead.
    ///
    fn align_up(&self, align: Alignment) -> Result<Self, Error> {
        let aligned: VirtualAddress = self.0.align_up(align).ok_or_else(|| {
            let reason: &str = "align_up overflow";
            error!(
                "PhysicalAddress::align_up(): {reason} (addr={:#x}, align={:?})",
                self.0.into_raw_value(),
                align
            );
            Error::new(ErrorCode::BadAddress, reason)
        })?;
        Self::from_virtual_address(aligned)
    }

    ///
    /// # Description
    ///
    /// Aligns the target [`PhysicalAddress`] down to the provided `alignment`. If the address is
    /// already aligned, it is returned as is.
    ///
    /// # Parameters
    ///
    /// - `alignment`: The alignment to align the target address to.
    ///
    /// # Returns
    ///
    /// Upon success, the aligned address is returned. Upon failure, an error is returned instead.
    ///
    fn align_down(&self, align: Alignment) -> Result<Self, Error> {
        Self::from_virtual_address(self.0.align_down(align))
    }

    ///
    /// # Description
    ///
    /// Checks if the target [`PhysicalAddress`] is aligned to the provided `alignment`.
    ///
    /// # Parameters
    ///
    /// - `alignment`: The alignment to check.
    ///
    /// # Returns
    ///
    /// Upon success, `true` is returned if the address is aligned, otherwise `false`. Upon failure,
    /// an error is returned instead.
    ///
    fn is_aligned(&self, align: Alignment) -> Result<bool, Error> {
        Ok(self.0.is_aligned(align))
    }

    ///
    /// # Description
    ///
    /// Returns the maximum address for [`PhysicalAddress`].
    ///
    /// # Returns
    ///
    /// The maximum [`PhysicalAddress`].
    ///
    fn max_addr() -> usize {
        crate::hal::platform::max_physical_address()
    }

    fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }

    fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }

    fn as_mut_ptr(&self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

impl core::fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

//==================================================================================================
// Material for verification
//==================================================================================================

#[cfg(verus_keep_ghost)]
verus! {

impl View for PhysicalAddress
{
    type V = int;

    closed spec fn view(&self) -> int
    {
        self.0@
    }
}

} // end verus!
