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
}

#[cfg(feature = "microvm")]
pub use microvm::*;

//==================================================================================================
// Hyperlight
//==================================================================================================

#[cfg(feature = "hyperlight")]
mod hyperlight {
    use super::MmioTag;

    /// Tag used to identify the Hyperlight PEB MMIO region.
    pub const PEB_MMIO_TAG: MmioTag = MmioTag::new(*b"PEB     ");

    /// Tag used to identify the Hyperlight input data buffer MMIO region.
    pub const INPUT_BUF_MMIO_TAG: MmioTag = MmioTag::new(*b"INPUTBUF");

    /// Tag used to identify the Hyperlight output data buffer MMIO region.
    pub const OUTPUT_BUF_MMIO_TAG: MmioTag = MmioTag::new(*b"OUTPUTBF");
}

#[cfg(feature = "hyperlight")]
pub use hyperlight::*;
