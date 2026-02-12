// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        self,
        Address,
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

#[cfg(target_pointer_width = "32")]
::static_assert::assert_eq_size!(VirtualAddress, ::core::mem::size_of::<u32>());

//==================================================================================================
// Implementations
//==================================================================================================

impl VirtualAddress {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    ///
    /// # Description
    ///
    /// Instantiates a new [`VirtualAddress`] from a raw value.
    ///
    /// # Parameters
    ///
    /// - `raw_addr`: The raw value.
    ///
    pub fn from_raw_value(raw_addr: usize) -> Self {
        VirtualAddress::new(raw_addr)
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
    /// Upon success, the aligned address is returned. Upon failure (overflow), `None` is returned
    /// instead.
    ///
    pub fn align_up(&self, align: Alignment) -> Option<Self> {
        mm::align_up(self.0, align).map(VirtualAddress::new)
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
    pub fn align_down(&self, align: Alignment) -> Self {
        VirtualAddress::new(mm::align_down(self.0, align))
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
    pub fn is_aligned(&self, align: Alignment) -> bool {
        mm::is_aligned(self.0, align)
    }

    ///
    /// # Description
    ///
    /// Performs a checked addition of a [`VirtualAddress`] and a `usize`.
    ///
    /// # Parameters
    ///
    /// - `rhs`: The value to add.
    ///
    /// # Returns
    ///
    /// Upon success, the new [`VirtualAddress`] is returned. Upon failure (overflow), `None` is
    /// returned instead.
    ///
    pub fn checked_add(&self, rhs: usize) -> Option<Self> {
        self.0.checked_add(rhs).map(VirtualAddress::from_raw_value)
    }

    ///
    /// # Description
    ///
    /// Performs a checked subtraction of a [`VirtualAddress`] and a `usize`.
    ///
    /// # Parameters
    ///
    /// - `rhs`: The value to subtract.
    ///
    /// # Returns
    ///
    /// Upon success, the new [`VirtualAddress`] is returned. Upon failure (underflow), `None` is
    /// returned instead.
    ///
    pub fn checked_sub(&self, rhs: usize) -> Option<Self> {
        self.0.checked_sub(rhs).map(VirtualAddress::from_raw_value)
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
        Ok(VirtualAddress::from_raw_value(raw_addr))
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
        self.align_up(align)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "align_up overflow"))
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
        Ok(self.align_down(align))
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
        Ok(self.is_aligned(align))
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#010x}", self.0)
    }
}

impl ::core::ops::Add<usize> for VirtualAddress {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        VirtualAddress::new(self.0 + rhs)
    }
}

impl ::core::ops::AddAssign<usize> for VirtualAddress {
    fn add_assign(&mut self, rhs: usize) {
        self.0 = self.0 + rhs;
    }
}

impl From<u32> for VirtualAddress {
    fn from(value: u32) -> Self {
        VirtualAddress::new(value as usize)
    }
}

#[cfg(target_pointer_width = "32")]
impl From<VirtualAddress> for u32 {
    fn from(value: VirtualAddress) -> Self {
        value.0 as u32
    }
}

impl From<VirtualAddress> for u64 {
    fn from(value: VirtualAddress) -> Self {
        value.0 as u64
    }
}

impl From<VirtualAddress> for usize {
    fn from(value: VirtualAddress) -> Self {
        value.0
    }
}
