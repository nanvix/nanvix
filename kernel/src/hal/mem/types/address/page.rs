// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::{
    arch::mem,
    hal::mem::types::address::{
        Address,
        PageAligned,
        PageTableAligned,
        VirtualAddress,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PageAddress(PageAligned<VirtualAddress>);

impl PageAddress {
    pub fn new(address: PageAligned<VirtualAddress>) -> Self {
        Self(address)
    }

    pub fn into_virtual_address(self) -> PageAligned<VirtualAddress> {
        self.0
    }

    pub fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }

    pub fn get_pte_index(&self) -> usize {
        let addr: usize = self.into_raw_value();
        (addr & (mem::PGTAB_MASK ^ mem::PAGE_MASK)) >> mem::PAGE_SHIFT
    }
}

impl PartialEq for PageAddress {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for PageAddress {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PageTableAddress(PageTableAligned<VirtualAddress>);

impl PageTableAddress {
    pub fn new(address: PageTableAligned<VirtualAddress>) -> Self {
        Self(address)
    }
    pub fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }

    pub fn get_pde_index(&self) -> usize {
        let addr: usize = self.into_raw_value();
        (addr & mem::PGTAB_MASK) >> mem::PGTAB_SHIFT
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
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for PageTableAddress {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
