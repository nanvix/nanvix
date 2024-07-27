// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::x86::mem::mmu,
        mem::Address,
    },
    klib::Alignment,
};
use ::core::ops::Deref;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone, Copy)]
pub struct PageTableAligned<T: Address>(T);

impl<T: Address> PageTableAligned<T> {
    /// Constructs a page address from an aligned virtual address.
    pub fn from_address(addr: T) -> Result<Self, Error> {
        // Check if `addr` is not aligned to a page boundary.
        if !addr.is_aligned(mmu::PGTAB_ALIGNMENT)? {
            return Err(Error::new(ErrorCode::BadAddress, "unaligned virtual address"));
        }

        Ok(Self(addr))
    }
}

impl<T: Address> Address for PageTableAligned<T> {
    fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }

    ///
    /// # Description
    ///
    /// Instantiates a new [`PageTableAligned`] from a raw value.
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
    fn from_raw_value(raw_addr: usize) -> Result<Self, Error> {
        Self::from_address(T::from_raw_value(raw_addr)?)
    }

    ///
    /// # Description
    ///
    ///  Aligns the target [`PageTableAligned`] to the provided `alignment`. If the address is already
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
        Self::from_address(self.0.align_up(align)?)
    }
    ///
    /// # Description
    ///
    /// Aligns the target [`PageTableAligned`] down to the provided `alignment`. If the address is
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
        Self::from_address(self.0.align_down(align)?)
    }

    ///
    /// # Description
    ///
    /// Checks if the target [`PageTableAligned`] is aligned to the provided `alignment`.
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
        self.0.is_aligned(align)
    }

    ///
    /// # Description
    ///
    /// Returns the maximum address for [`PageTableAligned`].
    ///
    /// # Returns
    ///
    /// The maximum [`PageTableAligned`].
    ///
    fn max_addr() -> usize {
        T::max_addr()
    }

    fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }

    fn as_mut_ptr(&self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

impl<T: Address> core::fmt::Debug for PageTableAligned<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<T: Address> Deref for PageTableAligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Address> PartialEq for PageTableAligned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(other)
    }
}

impl<T: Address> Eq for PageTableAligned<T> {}

impl<T: Address> PartialOrd for PageTableAligned<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Address> Ord for PageTableAligned<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(other)
    }
}
