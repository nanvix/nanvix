// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::x86::mem::mmu,
        mem::types::{
            access::AccessPermission,
            address::{
                Address,
                PageAligned,
                PhysicalAddress,
                VirtualAddress,
            },
        },
    },
    klib,
};
use ::alloc::string::{
    String,
    ToString,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Memory Region Type
//==================================================================================================

///
/// # Description
///
/// A type that represents the type of a memory region.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MemoryRegionType {
    /// Usable memory.
    Usable,
    /// Reserved memory.
    Reserved,
    /// Memory mapped I/O.
    Mmio,
    /// Bad memory.
    Bad,
}
//==================================================================================================
// Memory Region
//==================================================================================================

///
/// # Description
///
/// A memory region.
///
#[derive(Debug, Clone)]
pub struct MemoryRegion<T: Address> {
    name: String,
    start: T,
    size: usize,
    typ: MemoryRegionType,
    perm: AccessPermission,
}

impl<T: Address> MemoryRegion<T> {
    /// Creates a new memory region.
    pub fn new(
        name: &str,
        start: T,
        size: usize,
        typ: MemoryRegionType,
        perm: AccessPermission,
    ) -> Result<Self, Error> {
        // Check if size of the memory region is valid.
        if size == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid memory region size"));
        }

        // Check if memory region is too big.
        let start_raw_addr: usize = start.clone().into_raw_value();
        let end_raw_addr: usize = match start_raw_addr.checked_add(size - 1) {
            Some(end_raw_addr) => end_raw_addr,
            None => {
                return Err(Error::new(ErrorCode::TooBig, "memory region is too big"));
            },
        };

        // Check if memory region spans outside the address space.
        if end_raw_addr > T::max_addr() {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "memory region spans outside the address space",
            ));
        }

        Ok(Self {
            name: name.to_string(),
            start,
            size,
            typ,
            perm,
        })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns the first valid address that lies in the target memory region.
    pub fn start(&self) -> T {
        self.start.clone()
    }

    /// Returns the size of the target memory region.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the type of the target memory region.
    pub fn typ(&self) -> MemoryRegionType {
        self.typ
    }

    /// Returns the permissions of the target memory region.
    pub fn perm(&self) -> AccessPermission {
        self.perm
    }
}

impl<T: Address> PartialEq for MemoryRegion<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.start == other.start
            && self.size == other.size
            && self.typ == other.typ
            && self.perm == other.perm
    }
}

impl<T: Address> Eq for MemoryRegion<T> {}

impl<T: Address> PartialOrd for MemoryRegion<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Address> Ord for MemoryRegion<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.start.cmp(&other.start)
    }
}

//==================================================================================================
// Truncated Memory Region
//==================================================================================================

///
/// # Description
///
/// A memory region that has been truncated to a multiple of a page size.
///
#[derive(Clone)]
pub struct TruncatedMemoryRegion<T: Address>(MemoryRegion<PageAligned<T>>);

impl<T: Address> TruncatedMemoryRegion<T> {
    /// Creates a new truncated memory region.
    pub fn new(
        name: &str,
        start: PageAligned<T>,
        size: usize,
        typ: MemoryRegionType,
        perm: AccessPermission,
    ) -> Result<Self, Error> {
        // Truncate the size of the memory region to a multiple of the page size.
        let size: usize = klib::align_up(size, mmu::PAGE_ALIGNMENT);
        Ok(Self(MemoryRegion::new(name, start, size, typ, perm)?))
    }

    pub fn from_memory_region(region: MemoryRegion<T>) -> Result<Self, Error> {
        let start: T = region.start().align_down(mmu::PAGE_ALIGNMENT)?;
        let start: PageAligned<T> = PageAligned::from_address(start)?;
        let name: String = region.name();
        let size: usize = region.size();
        let typ: MemoryRegionType = region.typ();
        let perm: AccessPermission = region.perm();
        Self::new(&name, start, size, typ, perm)
    }

    pub fn name(&self) -> String {
        self.0.name()
    }

    /// Returns the first valid address that lies in the target memory region.
    pub fn start(&self) -> PageAligned<T> {
        self.0.start()
    }

    /// Returns the size of the target memory region.
    pub fn size(&self) -> usize {
        self.0.size()
    }

    /// Returns the type of the target memory region.
    pub fn typ(&self) -> MemoryRegionType {
        self.0.typ()
    }

    /// Returns the permissions of the target memory region.
    pub fn perm(&self) -> AccessPermission {
        self.0.perm()
    }
}

impl<T: Address> PartialEq for TruncatedMemoryRegion<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Address> Eq for TruncatedMemoryRegion<T> {}

impl<T: Address> PartialOrd for TruncatedMemoryRegion<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Address> Ord for TruncatedMemoryRegion<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: Address> core::fmt::Debug for TruncatedMemoryRegion<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "TruncatedMemoryRegion {{ name: {}, start: {:?}, size: {}, typ: {:?}, perm: {:?} }}",
            self.name(),
            self.start(),
            self.size(),
            self.typ(),
            self.perm()
        )
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

impl TruncatedMemoryRegion<PhysicalAddress> {
    /// Attempts to create a virtual memory region from a physical memory region.
    pub fn from_virtual_memory_region(region: MemoryRegion<VirtualAddress>) -> Result<Self, Error> {
        let name: String = region.name();
        let start: PageAligned<PhysicalAddress> =
            PageAligned::from_address(PhysicalAddress::from_virtual_address(region.start())?)?;
        let size: usize = region.size();
        let typ: MemoryRegionType = region.typ();
        let perm: AccessPermission = region.perm();
        TruncatedMemoryRegion::new(&name, start, size, typ, perm)
    }
}
