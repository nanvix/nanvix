// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::{
    AccessPermission,
    PageAligned,
    TruncatedMemoryRegion,
    VirtualAddress,
};
use ::alloc::rc::Rc;
use ::core::cell::RefCell;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone)]
pub struct IoMemoryRegion(Rc<RefCell<TruncatedMemoryRegion<VirtualAddress>>>);

//==================================================================================================
// Implementations
//==================================================================================================

impl IoMemoryRegion {
    pub fn base(&self) -> PageAligned<VirtualAddress> {
        self.0.borrow().start()
    }

    pub fn perm(&self) -> AccessPermission {
        self.0.borrow().perm()
    }

    pub(super) fn new(region: TruncatedMemoryRegion<VirtualAddress>) -> Self {
        Self(Rc::new(RefCell::new(region)))
    }

    pub(super) fn ref_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }
}

impl core::fmt::Debug for IoMemoryRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        write!(f, "{:?}", self.base())
    }
}

impl PartialEq for IoMemoryRegion {
    fn eq(&self, other: &Self) -> bool {
        self.base() == other.base()
    }
}

impl Eq for IoMemoryRegion {}

impl PartialOrd for IoMemoryRegion {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.base().partial_cmp(&other.base())
    }
}

impl Ord for IoMemoryRegion {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.base().cmp(&other.base())
    }
}
