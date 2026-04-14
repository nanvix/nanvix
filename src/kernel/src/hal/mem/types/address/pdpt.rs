// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::hal::mem::types::address::{
    Address,
    PageAligned,
    PhysicalAddress,
};
use ::sys::error::Error;

/// Physical address of a PDPT (Page Directory Pointer Table).
///
/// A thin wrapper around `PageAligned<PhysicalAddress>` that provides type safety for PDPT
/// physical addresses, preventing accidental use where a PML4 or PD address is expected.
#[derive(Debug, Clone, Copy)]
pub struct PdptAddress(PageAligned<PhysicalAddress>);

impl PdptAddress {
    /// Creates a new PDPT address from a raw physical address.
    pub fn from_raw_value(value: usize) -> Result<Self, Error> {
        Ok(Self(PageAligned::from_address(PhysicalAddress::from_raw_value(value)?)?))
    }

    /// Returns the raw physical address value.
    pub fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }
}

impl PartialEq for PdptAddress {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for PdptAddress {}

impl PartialOrd for PdptAddress {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PdptAddress {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
