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

impl PhysicalAddress {
    pub fn from_virtual_address(addr: VirtualAddress) -> Result<Self, Error> { ... }

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
    pub unsafe fn from_mmio_address(addr: VirtualAddress) -> Result<Self, Error> { ... }

    pub fn into_virtual_address(self) -> VirtualAddress { ... }

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
    pub fn from_number(frame: FrameNumber) -> Self { ... }

    pub fn into_frame_number(self) -> FrameNumber { ... }

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
    pub fn from_frame_address(frame_addr: FrameAddress) -> Self { ... }

    pub fn from_into_frame_address(frame_addr: FrameAddress) -> Self { ... }
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
    fn from_raw_value(value: usize) -> Result<Self, Error> { ... }

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
    fn align_up(&self, align: Alignment) -> Result<Self, Error> { ... }

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
    fn align_down(&self, align: Alignment) -> Result<Self, Error> { ... }

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
    fn is_aligned(&self, align: Alignment) -> Result<bool, Error> { ... }

    ///
    /// # Description
    ///
    /// Returns the maximum address for [`PhysicalAddress`].
    ///
    /// # Returns
    ///
    /// The maximum [`PhysicalAddress`].
    ///
    fn max_addr() -> usize { ... }

    fn into_raw_value(self) -> usize { ... }

    fn as_ptr(&self) -> *const u8 { ... }

    fn as_mut_ptr(&self) -> *mut u8 { ... }
}

impl core::fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { ... }
}

//==================================================================================================
// Material for verification
//==================================================================================================

verus! {

impl View for PhysicalAddress
{
    type V = int;

    closed spec fn view(&self) -> int
    { ... }
}

} // end verus!
