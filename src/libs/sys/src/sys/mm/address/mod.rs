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

// `Address` carries its own specification-level validity predicate, `spec_valid_raw`. It is
// declared as a `spec fn` member of the trait so that [`Address::from_raw_value`] can state its
// success/failure condition in terms of a per-type validity predicate. The trait therefore lives
// inside a `verus!` block, because Verus attribute mode (`#[verus_verify]`) cannot declare a
// `spec fn` member directly inside a trait. The default meaning is "every raw value is valid",
// which fits types whose constructor never fails (such as a plain virtual address); types with a
// restricted address space (physical, page-aligned, …) override it.
verus! {

pub trait Address
where
    Self: core::fmt::Debug + Clone + Copy + PartialEq + Eq + PartialOrd + Ord + View<V = int>,
{
    // Whether `raw_addr` denotes a valid address of this type. The default admits every raw value;
    // address types with a restricted address space override it. `from_raw_value` returns `Ok`
    // exactly when this predicate holds.
    open spec fn spec_valid_raw(raw_addr: usize) -> bool {
        true
    }

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
    fn from_raw_value(raw_addr: usize) -> (result: Result<Self, Error>)
        ensures
            match result {
                Ok(a) => Self::spec_valid_raw(raw_addr) && a@ == raw_addr as int,
                Err(e) => !Self::spec_valid_raw(raw_addr)
                    && e.code == crate::error::ErrorCode::BadAddress,
            };

    fn into_raw_value(self) -> (result: usize)
        ensures
            result as int == self@;

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
    fn is_aligned(&self, align: Alignment) -> (result: Result<bool, Error>)
        ensures
            result matches Ok(aligned)
                && aligned == spec_addr_is_aligned(self@, align);

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

} // verus!
