// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("region.spec.rs");
#[cfg(verus_keep_ghost)]
include!("region.proof.rs");

use crate::hal::mem::types::{
    access::AccessPermission,
    address::{
        Address,
        PageAligned,
        PhysicalAddress,
        VirtualAddress,
    },
};
use ::alloc::string::{
    String,
    ToString,
};
use ::arch::mem::PAGE_ALIGNMENT;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::vstd::prelude::*;

//==================================================================================================
// Memory Region Type
//==================================================================================================

///
/// # Description
///
/// A type that represents the type of a memory region.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[verus_verify]
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
// MMIO Cache Policy
//==================================================================================================

///
/// # Description
///
/// Caching policy for MMIO memory regions. Controls the PWT (Page Write-Through) and PCD
/// (Page Cache Disable) bits in the page table entries that back the region.
///
#[verus_verify(external_derive)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MmioCachePolicy {
    /// If `true`, the page is mapped with the Write-Through attribute (PWT=1).
    write_through: bool,
    /// If `true`, caching is enabled for the page (PCD=0).
    cache_enabled: bool,
}

impl MmioCachePolicy {
    ///
    /// # Description
    ///
    /// Creates a new MMIO cache policy.
    ///
    /// # Parameters
    ///
    /// - `write_through`: If `true`, enables the Write-Through attribute (PWT=1).
    /// - `cache_enabled`: If `true`, enables caching for the page (PCD=0).
    ///
    /// # Returns
    ///
    /// A new [`MmioCachePolicy`] instance.
    ///
    pub const fn new(write_through: bool, cache_enabled: bool) -> Self { ... }

    /// Uncacheable: Write-Through Enabled, Cache Disabled.
    pub const UNCACHEABLE: Self = Self {
        write_through: true,
        cache_enabled: false,
    };

    /// Write-Back: Write-Through Disabled, Cache Enabled.
    pub const WRITE_BACK: Self = Self {
        write_through: false,
        cache_enabled: true,
    };

    ///
    /// # Description
    ///
    /// Returns whether the Write-Through attribute is set.
    ///
    /// # Returns
    ///
    /// `true` if the page is mapped with the Write-Through attribute (PWT=1).
    ///
    pub fn write_through(&self) -> bool { ... }

    ///
    /// # Description
    ///
    /// Returns whether caching is enabled.
    ///
    /// # Returns
    ///
    /// `true` if caching is enabled for the page (PCD=0).
    ///
    pub fn cache_enabled(&self) -> bool { ... }
}
//==================================================================================================
// Memory Region
//==================================================================================================

///
/// # Description
///
/// A memory region.
///
#[verus_verify(external_derive)]
#[derive(Debug, Clone)]
pub struct MemoryRegion<T: Address> {
    name: String,
    start: T,
    size: usize,
    typ: MemoryRegionType,
    perm: AccessPermission,
    cache_policy: Option<MmioCachePolicy>,
}

impl<T: Address> MemoryRegion<T> {
    /// Maximum byte-length of a memory region name.
    const MEMORY_REGION_NAME_MAX: usize = 32;

    /// Creates a new memory region.
    pub fn new(
        name: &str,
        start: T,
        size: usize,
        typ: MemoryRegionType,
        perm: AccessPermission,
    ) -> Result<Self, Error> { ... }

    pub fn name(&self) -> String { ... }

    /// Returns the first valid address that lies in the target memory region.
    pub fn start(&self) -> T { ... }

    /// Returns the size of the target memory region.
    pub fn size(&self) -> usize { ... }

    /// Returns the type of the target memory region.
    pub fn typ(&self) -> MemoryRegionType { ... }

    /// Returns the permissions of the target memory region.
    pub fn perm(&self) -> AccessPermission { ... }

    ///
    /// # Description
    ///
    /// Returns the MMIO cache policy of the target memory region, if set.
    ///
    /// # Returns
    ///
    /// The [`MmioCachePolicy`] if one was assigned, or `None` otherwise.
    ///
    pub fn cache_policy(&self) -> Option<MmioCachePolicy> { ... }

    ///
    /// # Description
    ///
    /// Sets the MMIO cache policy of the target memory region.
    ///
    /// # Parameters
    ///
    /// - `policy`: The cache policy to assign.
    ///
    pub fn set_cache_policy(&mut self, policy: MmioCachePolicy) { ... }
}

impl<T: Address> PartialEq for MemoryRegion<T> {
    fn eq(&self, other: &Self) -> bool { ... }
}

impl<T: Address> Eq for MemoryRegion<T> {}

impl<T: Address> PartialOrd for MemoryRegion<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { ... }
}

impl<T: Address> Ord for MemoryRegion<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering { ... }
}

//==================================================================================================
// Truncated Memory Region
//==================================================================================================

///
/// # Description
///
/// A memory region that has been truncated to a multiple of a page size.
///
#[verus_verify(external_derive)]
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
    ) -> Result<Self, Error> { ... }

    ///
    /// # Description
    ///
    /// Creates a new truncated MMIO memory region with an explicit cache policy.
    ///
    /// # Parameters
    ///
    /// - `name`: Name of the memory region.
    /// - `start`: Page-aligned start address.
    /// - `size`: Size of the region in bytes (rounded up to page alignment).
    /// - `perm`: Access permissions for the region.
    /// - `cache_policy`: Caching policy that controls PWT/PCD bits in page table entries.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a new [`TruncatedMemoryRegion`] with type
    /// [`MemoryRegionType::Mmio`] is returned. Upon failure, an error is returned instead.
    ///
    pub fn new_mmio(
        name: &str,
        start: PageAligned<T>,
        size: usize,
        perm: AccessPermission,
        cache_policy: MmioCachePolicy,
    ) -> Result<Self, Error> { ... }

    pub fn from_memory_region(region: MemoryRegion<T>) -> Result<Self, Error> { ... }

    pub fn name(&self) -> String { ... }

    /// Returns the first valid address that lies in the target memory region.
    pub fn start(&self) -> PageAligned<T> { ... }

    /// Returns the size of the target memory region.
    pub fn size(&self) -> usize { ... }

    /// Returns the type of the target memory region.
    pub fn typ(&self) -> MemoryRegionType { ... }

    /// Returns the permissions of the target memory region.
    pub fn perm(&self) -> AccessPermission { ... }

    ///
    /// # Description
    ///
    /// Returns the MMIO cache policy of the target memory region, if set.
    ///
    /// # Returns
    ///
    /// The [`MmioCachePolicy`] if one was assigned, or `None` otherwise.
    ///
    pub fn cache_policy(&self) -> Option<MmioCachePolicy> { ... }
}

impl<T: Address> PartialEq for TruncatedMemoryRegion<T> {
    fn eq(&self, other: &Self) -> bool { ... }
}

impl<T: Address> Eq for TruncatedMemoryRegion<T> {}

impl<T: Address> PartialOrd for TruncatedMemoryRegion<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { ... }
}

impl<T: Address> Ord for TruncatedMemoryRegion<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering { ... }
}

impl<T: Address> core::fmt::Debug for TruncatedMemoryRegion<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { ... }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

impl TruncatedMemoryRegion<PhysicalAddress> {
    /// Attempts to create a virtual memory region from a physical memory region.
    pub fn from_virtual_memory_region(region: MemoryRegion<VirtualAddress>) -> Result<Self, Error> { ... }
}

//==================================================================================================
// Material for verification
//==================================================================================================

#[cfg(verus_keep_ghost)]
verus! {

use crate::hal::mem::spec_page_size;

pub struct MemoryRegionView
{
    pub start: int,
    pub size: int,
    pub typ: MemoryRegionType,
    pub perm: AccessPermission,
    pub cache_policy: Option<MmioCachePolicy>,
}

impl<T: Address + View<V = int>> View for MemoryRegion<T>
{
    type V = MemoryRegionView;

    closed spec fn view(&self) -> MemoryRegionView
    {
        MemoryRegionView{
            start: self.start@,
            size: self.size as int,
            typ: self.typ,
            perm: self.perm,
            cache_policy: self.cache_policy,
        }
    }
}

impl<T: Address + View<V = int>> View for TruncatedMemoryRegion<T>
{
    type V = MemoryRegionView;

    closed spec fn view(&self) -> MemoryRegionView
    {
        self.0@
    }
}

impl<T: Address + View<V = int>> TruncatedMemoryRegion<T>
{
    pub open spec fn inv(&self) -> bool
    {
        &&& self@.start % spec_page_size() == 0
        &&& self@.size % spec_page_size() == 0
    }
}

} // end verus!
