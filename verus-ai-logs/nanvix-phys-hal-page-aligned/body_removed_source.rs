// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("page.spec.rs");
#[cfg(verus_keep_ghost)]
include!("page.proof.rs");

use crate::hal::mem::{
    types::address::{
        PhysicalAddress,
        VirtualAddress,
    },
    Address,
};
use ::arch::mem::PAGE_ALIGNMENT;
use ::core::ops::Deref;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::Alignment,
};

//==================================================================================================
// Structures
//==================================================================================================

#[verus_verify(external_derive)]
#[derive(Clone, Copy)]
pub struct PageAligned<T: Address>(T);

#[verus_verify]
impl<T: Address> PageAligned<T> {
    /// Constructs a page address from an aligned virtual address.
    #[verus_spec(ret =>
        ensures
            match ret {
                Ok(r) => spec_aligned(addr@) && r@ == addr@ && r.inv(),
                Err(e) => !spec_aligned(addr@) && e.code == ErrorCode::BadAddress,
            },
    )]
    pub fn from_address(addr: T) -> Result<Self, Error> { ... }

    pub fn into_inner(self) -> T { ... }
}

#[verus_verify]
impl<T: Address> Address for PageAligned<T> {
    fn into_raw_value(self) -> usize { ... }

    fn clone_address(&self) -> Self { ... }

    ///
    /// # Description
    ///
    /// Instantiates a new [`PageAligned`] from a raw value.
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
    fn from_raw_value(raw_addr: usize) -> Result<Self, Error> { ... }

    ///
    /// # Description
    ///
    ///  Aligns the target [`PageAligned`] to the provided `alignment`. If the address is already
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
    /// Aligns the target [`PageAligned`] down to the provided `alignment`. If the address is
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
    /// Checks if the target [`PageAligned`] is aligned to the provided `alignment`.
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
    /// Returns the maximum address for [`PageAligned`].
    ///
    /// # Returns
    ///
    /// The maximum [`PageAligned`].
    ///
    fn max_addr() -> usize { ... }

    fn as_ptr(&self) -> *const u8 { ... }

    fn as_mut_ptr(&self) -> *mut u8 { ... }
}

impl<T: Address> core::fmt::Debug for PageAligned<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { ... }
}

impl<T: Address> Deref for PageAligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { ... }
}

impl<T: Address> PartialEq for PageAligned<T> {
    fn eq(&self, other: &Self) -> bool { ... }
}

impl<T: Address> Eq for PageAligned<T> {}

impl<T: Address> PartialOrd for PageAligned<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { ... }
}

impl<T: Address> Ord for PageAligned<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering { ... }
}

impl PageAligned<VirtualAddress> {
    /// Converts a page-aligned virtual address to a page-aligned physical address.
    pub fn into_physical_address(self) -> Result<PageAligned<PhysicalAddress>, Error> { ... }
}

impl PageAligned<PhysicalAddress> {
    /// Converts a page-aligned physical address to a page-aligned virtual address.
    pub fn into_virtual_address(self) -> PageAligned<VirtualAddress> { ... }
}

//==================================================================================================
// Material for verification
//==================================================================================================

#[cfg(verus_keep_ghost)]
verus! {

use crate::hal::mem::spec_page_size;

impl<T: Address> PageAligned<T>
{
    pub open spec fn inv(&self) -> bool
    {
        self@ % spec_page_size() == 0
    }
}

}

verus! {

impl<T: Address> View for PageAligned<T>
{
    type V = int;

    closed spec fn view(&self) -> int
    { ... }
}

}
