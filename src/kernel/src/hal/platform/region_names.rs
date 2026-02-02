// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// Name used for the I/O APIC MMIO region.
#[cfg(any(
    feature = "qemu-pc",
    feature = "qemu-isapc",
    feature = "qemu-baremetal"
))]
pub const IOAPIC_REGION_NAME: &str = "ioapic";

/// Name used for the Local APIC MMIO region.
#[cfg(any(
    feature = "qemu-pc",
    feature = "qemu-isapc",
    feature = "qemu-baremetal"
))]
pub const LAPIC_REGION_NAME: &str = "local_apic";

/// Name used for the VGA-compatible video MMIO window.
#[cfg(any(
    feature = "qemu-pc",
    feature = "qemu-isapc",
    feature = "qemu-baremetal"
))]
pub const VIDEO_MMIO_REGION_NAME: &str = "video display memory";

/// Name used for the RAMFS MMIO region.
#[cfg(feature = "microvm")]
pub const RAMFS_REGION_NAME: &str = "ramfs";
