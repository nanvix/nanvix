// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("region.spec.rs");

use crate::hal::mem::types::{
    access::AccessPermission,
    address::{
        Address,
        PageAligned,
        PhysicalAddress,
        VirtualAddress,
    },
};
use ::alloc::string::String;
use ::arch::mem::PAGE_ALIGNMENT;
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
// MMIO Cache Policy
//==================================================================================================

///
/// # Description
///
/// Caching policy for MMIO memory regions. Controls the PWT (Page Write-Through) and PCD
/// (Page Cache Disable) bits in the page table entries that back the region.
///
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
    pub const fn new(write_through: bool, cache_enabled: bool) -> Self {
        Self {
            write_through,
            cache_enabled,
        }
    }

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
    pub fn write_through(&self) -> bool {
        self.write_through
    }

    ///
    /// # Description
    ///
    /// Returns whether caching is enabled.
    ///
    /// # Returns
    ///
    /// `true` if caching is enabled for the page (PCD=0).
    ///
    pub fn cache_enabled(&self) -> bool {
        self.cache_enabled
    }
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
    ) -> Result<Self, Error> {
        // Check if name is too long (byte length).
        if name.len() > Self::MEMORY_REGION_NAME_MAX {
            let reason: &str = "memory region name is too long";
            error!("{reason} (name.len={}, NAME_MAX={})", name.len(), Self::MEMORY_REGION_NAME_MAX);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if size of the memory region is valid.
        if size == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid memory region size"));
        }

        // Check if memory region is too big.
        let start_raw_addr: usize = start.into_raw_value();
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
            name: crate::mm::try_string_from_str(name)?,
            start,
            size,
            typ,
            perm,
            cache_policy: None,
        })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns the first valid address that lies in the target memory region.
    // `T: Address` requires `Copy`, so this is a plain copy: the returned value has the same
    // abstract address (`res@ == self@.start`), discharging the postcondition below. A `.clone()`
    // here would NOT verify — `<T as Clone>::clone` on an abstract `T` carries no Verus contract.
    #[verus_spec(result =>
        ensures
            result@ == self@.start,
    )]
    pub fn start(&self) -> T {
        self.start
    }

    /// Returns the size of the target memory region.
    #[verus_spec(result =>
        ensures
            result as int == self@.size,
    )]
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

    ///
    /// # Description
    ///
    /// Returns the MMIO cache policy of the target memory region, if set.
    ///
    /// # Returns
    ///
    /// The [`MmioCachePolicy`] if one was assigned, or `None` otherwise.
    ///
    pub fn cache_policy(&self) -> Option<MmioCachePolicy> {
        self.cache_policy
    }

    ///
    /// # Description
    ///
    /// Sets the MMIO cache policy of the target memory region.
    ///
    /// # Parameters
    ///
    /// - `policy`: The cache policy to assign.
    ///
    pub fn set_cache_policy(&mut self, policy: MmioCachePolicy) {
        self.cache_policy = Some(policy);
    }
}

impl<T: Address> PartialEq for MemoryRegion<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.start == other.start
            && self.size == other.size
            && self.typ == other.typ
            && self.perm == other.perm
            && self.cache_policy == other.cache_policy
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
        let size: usize = ::sys::mm::align_up(size, PAGE_ALIGNMENT).ok_or_else(|| {
            let reason: &str = "align_up overflow";
            error!("TruncatedMemoryRegion::new(): {reason} (name={name:?}, size={size})");
            Error::new(ErrorCode::InvalidArgument, reason)
        })?;
        Ok(Self(MemoryRegion::new(name, start, size, typ, perm)?))
    }

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
    ) -> Result<Self, Error> {
        let mut region: Self = Self::new(name, start, size, MemoryRegionType::Mmio, perm)?;
        region.0.set_cache_policy(cache_policy);
        Ok(region)
    }

    pub fn from_memory_region(region: MemoryRegion<T>) -> Result<Self, Error> {
        let cache_policy: Option<MmioCachePolicy> = region.cache_policy();
        let start: T = region.start().align_down(PAGE_ALIGNMENT)?;
        let start: PageAligned<T> = PageAligned::from_address(start)?;
        let name: String = region.name();
        let size: usize = region.size();
        let typ: MemoryRegionType = region.typ();
        let perm: AccessPermission = region.perm();
        let mut truncated: Self = Self::new(&name, start, size, typ, perm)?;
        if let Some(policy) = cache_policy {
            truncated.0.set_cache_policy(policy);
        }
        Ok(truncated)
    }

    pub fn name(&self) -> String {
        self.0.name()
    }

    /// Returns the first valid address that lies in the target memory region.
    #[verus_spec(result =>
        ensures
            result@ == self@.start,
    )]
    pub fn start(&self) -> PageAligned<T> {
        self.0.start()
    }

    /// Returns the size of the target memory region.
    #[verus_spec(result =>
        ensures
            result as int == self@.size,
    )]
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

    ///
    /// # Description
    ///
    /// Returns the MMIO cache policy of the target memory region, if set.
    ///
    /// # Returns
    ///
    /// The [`MmioCachePolicy`] if one was assigned, or `None` otherwise.
    ///
    pub fn cache_policy(&self) -> Option<MmioCachePolicy> {
        self.0.cache_policy()
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
            "TruncatedMemoryRegion {{ name: {}, start: {:?}, size: {}, typ: {:?}, perm: {:?}, \
             cache_policy: {:?} }}",
            self.name(),
            self.start(),
            self.size(),
            self.typ(),
            self.perm(),
            self.cache_policy()
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
