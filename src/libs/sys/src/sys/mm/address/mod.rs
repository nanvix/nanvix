// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("mod.spec.rs");

mod phys;
mod virt;

//==================================================================================================
// Exports
//==================================================================================================

pub use phys::*;
pub use virt::*;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Error,
    mm::Alignment,
};

//==================================================================================================
// Traits
//==================================================================================================

#[verus_verify]
pub trait Address
where
    Self: core::fmt::Debug + Clone + Copy + PartialEq + Eq + PartialOrd + Ord + View<V = int>,
{
    ///
    /// # Description
    ///
    /// Instantiates a new [`Address`] from a raw value.
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
    #[verus_spec(result =>
        ensures
            match result {
                Ok(a) => a@ == raw_addr as int,
                Err(e) => e.code == crate::error::ErrorCode::BadAddress,
            },
    )]
    fn from_raw_value(raw_addr: usize) -> Result<Self, Error>;

    #[verus_spec(result =>
        ensures
            result as int == self@,
    )]
    fn into_raw_value(self) -> usize;

    ///
    /// # Description
    ///
    ///  Aligns the target [`Address`] to the provided `alignment`. If the address is already
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
    fn align_up(&self, align: Alignment) -> Result<Self, Error>;

    ///
    /// # Description
    ///
    /// Aligns the target [`Address`] down to the provided `alignment`. If the address is
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
    fn align_down(&self, align: Alignment) -> Result<Self, Error>;

    ///
    /// # Description
    ///
    /// Checks if the target [`Address`] is aligned to the provided `alignment`.
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
    #[verus_spec(result =>
        ensures
            result matches Ok(aligned)
                && aligned == spec_addr_is_aligned(self@, align),
    )]
    fn is_aligned(&self, align: Alignment) -> Result<bool, Error>;

    ///
    /// # Description
    ///
    /// Returns the maximum address for [`Address`].
    ///
    /// # Returns
    ///
    /// The maximum [`Address`].
    ///
    fn max_addr() -> usize;

    fn as_ptr(&self) -> *const u8;

    fn as_mut_ptr(&self) -> *mut u8;
}
