// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::types::address::{
    Address,
    PageAligned,
    PhysicalAddress,
};
use ::sys::error::Error;

//==================================================================================================
// Page Directory Address
//==================================================================================================

/// Physical address of a page directory.
#[derive(Debug, Clone, Copy)]
pub struct PageDirectoryAddress(PageAligned<PhysicalAddress>);

impl PageDirectoryAddress {
    /// Creates a new page directory address from a raw physical address.
    pub fn from_raw_value(value: usize) -> Result<Self, Error> {
        Ok(Self(PageAligned::from_raw_value(value)?))
    }

    /// Returns the raw physical address value.
    pub fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }
}

impl From<PageDirectoryAddress> for usize {
    fn from(addr: PageDirectoryAddress) -> usize {
        addr.into_raw_value()
    }
}
