// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Error,
    hal::mem::types::address::Address,
    klib::{
        self,
        Alignment,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a virtual address.
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(usize);

//==================================================================================================
// Implementations
//==================================================================================================

impl VirtualAddress {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }
}

impl Address for VirtualAddress {
    ///
    /// # Description
    ///
    /// Instantiates a new [`VirtualAddress`] from a raw value.
    ///
    /// # Parameters
    ///
    /// - `raw_addr`: The raw value.
    ///
    /// # Returns
    ///
    /// - `Ok(Self)`: The new address.
    ///
    fn from_raw_value(raw_addr: usize) -> Result<Self, Error> {
        Ok(VirtualAddress::new(raw_addr))
    }

    ///
    /// # Description
    ///
    /// Aligns the target [`VirtualAddress`] to the provided `alignment`. If the address is already
    /// aligned, it is returned as is.
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
        Ok(VirtualAddress::new(klib::align_up(self.0, align)))
    }

    ///
    /// # Description
    ///
    /// Aligns the target [`VirtualAddress`] down to the provided `alignment`. If the address is
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
        Ok(VirtualAddress::new(klib::align_down(self.0, align)))
    }

    ///
    /// # Description
    ///
    /// Checks if the target [`VirtualAddress`] is aligned to the provided `alignment`.
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
        Ok(klib::is_aligned(self.0, align))
    }

    ///
    /// # Description
    ///
    /// Returns the maximum address for [`VirtualAddress`].
    ///
    /// # Returns
    ///
    /// The maximum [`VirtualAddress`].
    ///
    fn max_addr() -> usize {
        usize::MAX
    }

    fn into_raw_value(self) -> usize {
        self.0
    }

    fn as_ptr(&self) -> *const u8 {
        self.0 as *const u8
    }

    fn as_mut_ptr(&self) -> *mut u8 {
        self.0 as *mut u8
    }
}

impl core::fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:#010x}", self.0)
    }
}
