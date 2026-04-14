// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::hal::mem::types::address::{
    Address,
    PageAligned,
    PhysicalAddress,
};
use ::sys::error::Error;

/// Physical address of a PML4 (Page Map Level 4) table.
///
/// A thin wrapper around `PageAligned<PhysicalAddress>` that provides type safety for PML4
/// physical addresses, preventing accidental use where a PDPT or PD address is expected.
#[derive(Debug, Clone, Copy)]
pub struct Pml4Address(PageAligned<PhysicalAddress>);

impl Pml4Address {
    /// Creates a new PML4 address from a raw physical address.
    pub fn from_raw_value(value: usize) -> Result<Self, Error> {
        Ok(Self(PageAligned::from_address(PhysicalAddress::from_raw_value(value)?)?))
    }

    /// Returns the raw physical address value.
    pub fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }
}

impl From<Pml4Address> for usize {
    fn from(addr: Pml4Address) -> usize {
        addr.into_raw_value()
    }
}

impl PartialEq for Pml4Address {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Pml4Address {}

impl PartialOrd for Pml4Address {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pml4Address {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
