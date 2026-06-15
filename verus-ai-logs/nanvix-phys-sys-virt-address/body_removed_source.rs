// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("virt.spec.rs");
#[cfg(verus_keep_ghost)]
include!("virt.proof.rs");

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
#[verus_verify(external_derive)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(usize);

#[cfg(target_pointer_width = "32")]
::static_assert::assert_eq_size!(VirtualAddress, ::core::mem::size_of::<u32>());

//==================================================================================================
// Implementations
//==================================================================================================

#[verus_verify]
impl VirtualAddress {
    #[verus_spec(result =>
        ensures
            result@ == value as int,
    )]
    pub const fn new(value: usize) -> Self { ... }

    ///
    /// # Description
    ///
    /// Instantiates a new [`VirtualAddress`] from a raw value.
    ///
    /// # Parameters
    ///
    /// - `raw_addr`: The raw value.
    ///
    #[verus_spec(result =>
        ensures
            result@ == raw_addr as int,
    )]
    pub fn from_raw_value(raw_addr: usize) -> Self { ... }
}

impl VirtualAddress {
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
    pub fn align_up(&self, align: Alignment) -> Option<Self> { ... }

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
    pub fn align_down(&self, align: Alignment) -> Self { ... }

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
    pub fn is_aligned(&self, align: Alignment) -> bool { ... }

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
    pub fn checked_add(&self, rhs: usize) -> Option<Self> { ... }

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
    pub fn checked_sub(&self, rhs: usize) -> Option<Self> { ... }
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
    fn from_raw_value(raw_addr: usize) -> Result<Self, Error> { ... }

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
    fn align_up(&self, align: Alignment) -> Result<Self, Error> { ... }

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
    fn align_down(&self, align: Alignment) -> Result<Self, Error> { ... }

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
    fn is_aligned(&self, align: Alignment) -> Result<bool, Error> { ... }

    ///
    /// # Description
    ///
    /// Returns the maximum address for [`VirtualAddress`].
    ///
    /// # Returns
    ///
    /// The maximum [`VirtualAddress`].
    ///
    fn max_addr() -> usize { ... }

    fn into_raw_value(self) -> usize { ... }

    fn clone_address(&self) -> Self { ... }

    fn as_ptr(&self) -> *const u8 { ... }

    fn as_mut_ptr(&self) -> *mut u8 { ... }
}

impl core::fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { ... }
}

impl ::core::ops::Add<usize> for VirtualAddress {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output { ... }
}

impl ::core::ops::AddAssign<usize> for VirtualAddress {
    fn add_assign(&mut self, rhs: usize) { ... }
}

impl From<u32> for VirtualAddress {
    fn from(value: u32) -> Self { ... }
}

#[cfg(target_pointer_width = "32")]
impl From<VirtualAddress> for u32 {
    fn from(value: VirtualAddress) -> Self { ... }
}

impl From<VirtualAddress> for u64 {
    fn from(value: VirtualAddress) -> Self { ... }
}

impl From<VirtualAddress> for usize {
    fn from(value: VirtualAddress) -> Self { ... }
}

//==================================================================================================
// Material for verification
//==================================================================================================

verus! {

impl View for VirtualAddress {
    type V = int;

    closed spec fn view(&self) -> int
    { ... }
}

} // end verus!
