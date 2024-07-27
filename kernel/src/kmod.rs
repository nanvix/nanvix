// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::hal::mem::PhysicalAddress;
use alloc::string::{
    String,
    ToString,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct KernelModule {
    /// Start address.
    start: PhysicalAddress,
    /// Size.
    size: usize,
    /// Command line.
    cmdline: String,
}

impl KernelModule {
    /// Creates a new kernel module.
    pub fn new(start: PhysicalAddress, size: usize, cmdline: String) -> Self {
        Self {
            start,
            size,
            cmdline,
        }
    }

    /// Gets the start address of the module.
    pub fn start(&self) -> PhysicalAddress {
        self.start
    }

    /// Gets the size of the module.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Gets the command line of the module.
    pub fn cmdline(&self) -> String {
        self.cmdline.to_string()
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
