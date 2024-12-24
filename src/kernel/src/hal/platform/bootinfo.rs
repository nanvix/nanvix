// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        mem::{
            MemoryRegion,
            TruncatedMemoryRegion,
            VirtualAddress,
        },
        platform::madt::MadtInfo,
    },
    kmod::KernelModule,
};
use ::alloc::collections::LinkedList;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents information collected by the bootloader.
///
#[derive(Default)]
pub struct BootInfo {
    /// ACPI MADT information.
    pub madt: Option<MadtInfo>,
    /// Lower memory size.
    pub mem_lower: Option<usize>,
    /// General-purpose memory regions.
    pub memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,
    /// Memory-mapped I/O regions.
    pub mmio_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    /// Kernel modules.
    pub kernel_modules: LinkedList<KernelModule>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl BootInfo {
    ///
    /// # Description
    ///
    /// Instantiates a new boot information structure.
    ///
    /// # Parameters
    ///
    /// - `madt`: ACPI MADT information.
    /// - `mem_lower`: Available Lower memory.
    /// - `memory_regions`: General-purpose memory regions.
    /// - `mmio_regions`: Memory-mapped I/O regions.
    /// - `kernel_modules`: Kernel modules.
    ///
    /// # Returns
    ///
    /// A new boot information structure.
    ///
    pub fn new(
        madt: Option<MadtInfo>,
        mem_lower: Option<usize>,
        memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,
        mmio_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
        kernel_modules: LinkedList<KernelModule>,
    ) -> Self {
        Self {
            madt,
            mem_lower,
            memory_regions,
            mmio_regions,
            kernel_modules,
        }
    }
}
