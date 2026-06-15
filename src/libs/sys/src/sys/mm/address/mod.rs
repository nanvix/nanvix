// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("mod.spec.rs");
#[cfg(verus_keep_ghost)]
include!("mod.proof.rs");

mod virt;

//==================================================================================================
// Exports
//==================================================================================================

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
    Self: core::fmt::Debug + Clone + Copy + PartialEq + Eq + PartialOrd + Ord,
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
    // On success, the constructed address projects back to exactly the supplied
    // raw value (the inverse of `into_raw_value`). On failure, the value was
    // outside this type's domain and `Err(Error::BadAddress)` is returned.
    // (`VirtualAddress` never fails; refinement implementors fail on their own
    // domain predicate.) The universal pointer-sized bound `addr_inv(&a)` is not
    // restated here: it is derivable from `spec_addr(&a) == raw_addr as int`
    // because `raw_addr: usize` forces `0 <= raw_addr as int <= usize::MAX`.
    #[verus_spec(result =>
        ensures
            match result {
                Ok(a) => spec_addr(&a) == raw_addr as int,
                Err(e) => e.code == crate::error::ErrorCode::BadAddress,
            },
    )]
    fn from_raw_value(raw_addr: usize) -> Result<Self, Error>;

    // Total projection to the raw numeric address; never fails. Pure newtype
    // identity (`result as int == spec_addr(&self)`) — the inverse of
    // `from_raw_value`. This is the fact downstream crates pin as a trust
    // boundary today (`kernel`'s `phys.spec.rs` `assume_specification`).
    #[verus_spec(result =>
        ensures
            result as int == spec_addr(&self),
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
    // The boolean is exactly the alignment predicate over the abstract address
    // (`b == spec_addr(self) % align == 0`); this is the only payload callers
    // branch on. `Alignment` is a closed enum of valid powers of two, so there
    // is no genuine failure condition: concrete implementors never take the
    // `Err` arm, and the contract pins this as total (`result is Ok`) so that
    // alignment guards (`mprotect`/`munmap`/`heap`/`PageAligned`) can rely on
    // the query succeeding.
    #[verus_spec(result =>
        ensures
            result is Ok,
            result->Ok_0 == addr_is_aligned(spec_addr(self), align),
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
