// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// Name used for the RAMFS MMIO region.
#[cfg(any(feature = "microvm", feature = "hyperlight"))]
pub const RAMFS_REGION_NAME: &str = "ramfs";
