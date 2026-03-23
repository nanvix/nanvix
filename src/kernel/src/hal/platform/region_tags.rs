// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::io::MmioTag;

//==================================================================================================
// Constants
//==================================================================================================

/// Tag used to identify the I/O APIC MMIO region.
pub const IOAPIC_MMIO_TAG: MmioTag = MmioTag::new(*b"IOAPIC  ");

/// Tag used to identify the Local APIC MMIO region.
pub const LAPIC_MMIO_TAG: MmioTag = MmioTag::new(*b"LAPIC   ");

/// Tag used to identify the RAMFS MMIO region.
#[cfg(any(feature = "microvm", feature = "hyperlight"))]
pub const RAMFS_MMIO_TAG: MmioTag = MmioTag::new(*b"RAMFS   ");
