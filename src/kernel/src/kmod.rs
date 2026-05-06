// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::hal::mem::PhysicalAddress;

//==================================================================================================
// Structures
//==================================================================================================

pub struct KernelModule {
    /// Start address of the ELF binary data.
    start: PhysicalAddress,
    /// Size of the ELF binary data.
    size: usize,
    /// Base address of the memory region that must be mapped for this module.
    /// For multibinary images this covers the full image (including the header
    /// where cmdline strings reside); for single-binary modules it equals `start`.
    region_base: PhysicalAddress,
    /// Total size of the memory region from `region_base` that must be mapped.
    region_size: usize,
    /// Command line.
    cmdline: &'static str,
}

impl KernelModule {
    /// Creates a new kernel module whose mapped region matches the ELF extent.
    pub fn new(start: PhysicalAddress, size: usize, cmdline: &'static str) -> Self {
        Self {
            start,
            size,
            region_base: start,
            region_size: size,
            cmdline,
        }
    }

    /// Creates a new kernel module with an explicit memory region base and size.
    pub fn new_with_region(
        start: PhysicalAddress,
        size: usize,
        region_base: PhysicalAddress,
        region_size: usize,
        cmdline: &'static str,
    ) -> Self {
        Self {
            start,
            size,
            region_base,
            region_size,
            cmdline,
        }
    }

    /// Gets the start address of the ELF binary.
    pub fn start(&self) -> PhysicalAddress {
        self.start
    }

    /// Gets the size of the ELF binary.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Gets the base address of the memory region to map.
    pub fn region_base(&self) -> PhysicalAddress {
        self.region_base
    }

    /// Gets the size of the memory region to map.
    pub fn region_size(&self) -> usize {
        self.region_size
    }

    /// Gets the command line of the module.
    pub fn cmdline(&self) -> &str {
        self.cmdline
    }
}

impl core::fmt::Debug for KernelModule {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "kernel_module {{ start: {:?}, size: {:?}, cmdline: {:?} }}",
            self.start, self.size, self.cmdline
        )
    }
}
