// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        Address,
        Alignment,
        VirtualAddress,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a physical address.
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(VirtualAddress);

//==================================================================================================
// Implementations
//==================================================================================================

impl PhysicalAddress {
    ///
    /// # Description
    ///
    /// Constructs a [`PhysicalAddress`] from a [`VirtualAddress`].
    ///
    /// # Parameters
    ///
    /// - `addr`: Virtual address whose raw value identifies a physical address.
    ///
    /// # Returns
    ///
    /// Upon success, the physical address is returned. Upon failure, an error is returned instead.
    ///
    /// # Errors
    ///
    /// This function returns an error if the address is outside the physical address space.
    ///
    pub fn from_virtual_address(addr: VirtualAddress) -> Result<Self, Error> {
        if !is_valid_physical_address(addr) {
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
    /// # Returns
    ///
    /// The physical address associated with the given memory-mapped I/O address.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if the provided memory-mapped I/O address is invalid.
    ///
    pub unsafe fn from_mmio_address(addr: VirtualAddress) -> Self {
        Self(addr)
    }

    ///
    /// # Description
    ///
    /// Converts this physical address into a corresponding virtual address.
    ///
    /// # Returns
    ///
    /// The virtual address.
    ///
    pub fn into_virtual_address(self) -> VirtualAddress {
        self.0
    }

    ///
    /// # Description
    ///
    /// Performs a checked addition of a [`PhysicalAddress`] and a `usize`.
    ///
    /// # Parameters
    ///
    /// - `rhs`: The value to add.
    ///
    /// # Returns
    ///
    /// Upon success, the new [`PhysicalAddress`] is returned. Upon failure (overflow), `None` is
    /// returned instead.
    ///
    pub fn checked_add(&self, rhs: usize) -> Option<Self> {
        self.0
            .checked_add(rhs)
            .map(Self::from_virtual_address)
            .transpose()
            .ok()
            .flatten()
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
        let aligned: VirtualAddress = self
            .0
            .align_up(align)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "align_up overflow"))?;
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
        ::config::kernel::MEMORY_SIZE - 1
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

impl TryFrom<u64> for PhysicalAddress {
    type Error = Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let value: usize = usize::try_from(value)
            .map_err(|_| Error::new(ErrorCode::BadAddress, "physical address exceeds usize"))?;
        PhysicalAddress::from_raw_value(value)
    }
}

impl TryFrom<u32> for PhysicalAddress {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        PhysicalAddress::from_raw_value(value as usize)
    }
}

#[cfg(target_pointer_width = "32")]
impl From<PhysicalAddress> for u32 {
    fn from(value: PhysicalAddress) -> Self {
        value.0.into()
    }
}

impl From<PhysicalAddress> for u64 {
    fn from(value: PhysicalAddress) -> Self {
        value.0.into()
    }
}

impl From<PhysicalAddress> for usize {
    fn from(value: PhysicalAddress) -> Self {
        value.0.into()
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Checks whether the given virtual address corresponds to a valid physical address.
///
/// # Parameters
///
/// - `addr`: The virtual address to validate.
///
/// # Returns
///
/// `true` if `addr` falls within the physical address space, `false` otherwise.
///
#[inline(always)]
pub fn is_valid_physical_address(addr: VirtualAddress) -> bool {
    addr < VirtualAddress::from_raw_value(config::kernel::MEMORY_SIZE)
}

//==================================================================================================
// Material for verification
//==================================================================================================

verus! {

impl View for PhysicalAddress {
    type V = int;

    closed spec fn view(&self) -> int
    {
        self.0@
    }
}

} // end verus!
