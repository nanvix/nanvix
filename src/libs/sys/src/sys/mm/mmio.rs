// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        AccessPermission,
        Address,
        VirtualAddress,
    },
};
use ::core::convert::TryFrom;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Describes the attributes of a memory-mapped I/O region that is attached to a process.
///
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MmioRegionInfo {
    /// Base virtual address of the MMIO mapping (32-bit).
    base: u32,
    /// Size of the MMIO mapping in bytes.
    size: u32,
    /// Encoded access permissions (bitmask compatible with [`AccessPermission`]).
    permissions: u32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl MmioRegionInfo {
    ///
    /// # Description
    ///
    /// Creates a new [`MmioRegionInfo`] from strongly-typed components.
    ///
    /// # Parameters
    ///
    /// - `base`: Page-aligned virtual address of the mapping.
    /// - `size`: Size of the mapping in bytes.
    /// - `permissions`: Access permissions granted to the mapping.
    ///
    /// # Returns
    ///
    /// Upon success, a populated [`MmioRegionInfo`] is returned. Upon failure, an error is returned
    /// instead.
    ///
    pub fn new(
        base: VirtualAddress,
        size: usize,
        permissions: AccessPermission,
    ) -> Result<Self, Error> {
        let base_raw: u32 = u32::try_from(base.into_raw_value()).map_err(|_| {
            Error::new(ErrorCode::ValueOutOfRange, "mmio base address exceeds 32 bits")
        })?;
        let size_raw: u32 = u32::try_from(size)
            .map_err(|_| Error::new(ErrorCode::ValueOutOfRange, "mmio size exceeds 32 bits"))?;

        Ok(Self {
            base: base_raw,
            size: size_raw,
            permissions: permissions.into(),
        })
    }

    ///
    /// # Description
    ///
    /// Instantiates a new [`MmioRegionInfo`] directly from raw primitives.
    ///
    /// # Parameters
    ///
    /// - `base`: Raw base virtual address.
    /// - `size`: Size of the region in bytes.
    /// - `permissions`: Encoded access permissions bitmask.
    ///
    /// # Returns
    ///
    /// A new [`MmioRegionInfo`] initialized with the provided values.
    ///
    pub const fn from_raw_parts(base: u32, size: u32, permissions: u32) -> Self {
        Self {
            base,
            size,
            permissions,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the raw base virtual address encoded in the structure.
    ///
    /// # Returns
    ///
    /// The base address as a 32-bit unsigned integer.
    ///
    pub const fn base_raw(&self) -> u32 {
        self.base
    }

    ///
    /// # Description
    ///
    /// Returns the base address as a [`VirtualAddress`].
    ///
    /// # Returns
    ///
    /// The base address of the MMIO region.
    ///
    pub fn base(&self) -> VirtualAddress {
        VirtualAddress::from_raw_value(self.base as usize)
    }

    ///
    /// # Description
    ///
    /// Returns the raw size in bytes.
    ///
    /// # Returns
    ///
    /// The size of the MMIO region as a 32-bit unsigned integer.
    ///
    pub const fn size_raw(&self) -> u32 {
        self.size
    }

    ///
    /// # Description
    ///
    /// Returns the size of the region as a host `usize`.
    ///
    /// # Returns
    ///
    /// The size of the MMIO region in bytes.
    ///
    pub fn size(&self) -> usize {
        self.size as usize
    }

    ///
    /// # Description
    ///
    /// Returns the raw permission field.
    ///
    /// # Returns
    ///
    /// The encoded permissions as a 32-bit unsigned integer.
    ///
    pub const fn permissions_raw(&self) -> u32 {
        self.permissions
    }

    ///
    /// # Description
    ///
    /// Returns the decoded permissions as an [`AccessPermission`].
    ///
    /// # Returns
    ///
    /// Upon success, the access permissions are returned. Upon failure, an error is returned
    /// instead.
    ///
    pub fn permissions(&self) -> Result<AccessPermission, Error> {
        AccessPermission::try_from(self.permissions)
    }
}
