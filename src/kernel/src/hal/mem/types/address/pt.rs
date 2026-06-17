// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::hal::mem::types::address::{
    Address,
    PageAligned,
    PhysicalAddress,
};
use ::sys::error::Error;

/// Physical address of a page table.
///
/// A thin wrapper around `PageAligned<PhysicalAddress>` that provides type safety for PT
/// physical addresses, preventing accidental use where a PD, PDPT, or PML4 address is expected.
#[derive(Debug, Clone, Copy)]
pub struct PageTableAddress(PageAligned<PhysicalAddress>);

impl PageTableAddress {
    /// Creates a new page table address from a raw physical address.
    pub fn from_raw_value(value: usize) -> Result<Self, Error> {
        Ok(Self(PageAligned::from_address(PhysicalAddress::from_raw_value(value)?)?))
    }

    /// Returns the raw physical address value.
    pub fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }
}

impl PartialEq for PageTableAddress {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for PageTableAddress {}

impl PartialOrd for PageTableAddress {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PageTableAddress {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
