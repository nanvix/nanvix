// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::mmio_tag::MmioTag;

//==================================================================================================
// Constants
//==================================================================================================

/// Tag used to identify the I/O APIC MMIO region.
pub const IOAPIC_MMIO_TAG: MmioTag = MmioTag::new(*b"IOAPIC  ");

/// Tag used to identify the Local APIC MMIO region.
pub const LAPIC_MMIO_TAG: MmioTag = MmioTag::new(*b"LAPIC   ");

/// Tag used to identify the RAMFS MMIO region.
pub const RAMFS_MMIO_TAG: MmioTag = MmioTag::new(*b"RAMFS   ");

/// Tag used to identify the root filesystem image.
pub const ROOTFS_MMIO_TAG: MmioTag = MmioTag::new(*b"ROOTFS  ");

//==================================================================================================
// MicroVM
//==================================================================================================

#[cfg(feature = "microvm")]
mod microvm {
    use super::MmioTag;

    /// Tag used to identify the MicroVM control registers MMIO region.
    pub const MICROVM_CTRL_MMIO_TAG: MmioTag = MmioTag::new(*b"MVMCTRL ");

    /// Tag used to identify the pvclock MMIO region.
    pub const PVCLOCK_MMIO_TAG: MmioTag = MmioTag::new(*b"PVCLOCK ");

    /// Tag used to identify the AArch64 GIC distributor MMIO region.
    pub const GICD_MMIO_TAG: MmioTag = MmioTag::new(*b"GICD    ");

    /// Tag used to identify the AArch64 GIC redistributor MMIO region.
    pub const GICR_MMIO_TAG: MmioTag = MmioTag::new(*b"GICR    ");

    /// Tag used to identify the AArch64 GIC ITS MMIO region.
    pub const GITS_MMIO_TAG: MmioTag = MmioTag::new(*b"GITS    ");

    /// Tag used to identify the AArch64 host-service doorbell MMIO region.
    pub const DOORBELL_MMIO_TAG: MmioTag = MmioTag::new(*b"DOORBELL");
}

#[cfg(feature = "microvm")]
pub use microvm::*;
