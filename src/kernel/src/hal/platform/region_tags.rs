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

/// Tag used to identify the MicroVM RAMFS MMIO region.
#[cfg(feature = "microvm")]
pub const RAMFS_MMIO_TAG: MmioTag = MmioTag::new(*b"RAMFS   ");

/// Tag used to identify the legacy VGA-compatible video MMIO window.
#[cfg(any(
    feature = "qemu-pc",
    feature = "qemu-isapc",
    feature = "qemu-baremetal"
))]
pub const VIDEO_MMIO_TAG: MmioTag = MmioTag::new(*b"VIDEO   ");
