// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::platform::bootinfo::BootInfo;
use ::sys::error::Error;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Stores arguments passed to the kernel.
///
#[repr(C)]
pub struct KernelArguments {
    /// Boot magic value.
    boot_magic: u32,
    /// Address of boot information structure.
    boot_info: usize,
}

// `KernelArguments` must be 8 bytes long. This must match low-level startup code.
::static_assert::assert_eq_size!(KernelArguments, 8);

// `KernelArguments` must be aligned to 4 bytes. This must match low-level startup code.
::static_assert::assert_eq_align!(KernelArguments, 4);

//==================================================================================================
// Implementations
//==================================================================================================

impl KernelArguments {
    /// Parses kernel arguments.
    pub fn parse(&self) -> Result<BootInfo, Error> {
        crate::hal::platform::parse_bootinfo(self.boot_magic, self.boot_info)
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl core::fmt::Debug for KernelArguments {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "kernel_arguments {{ boot_magic: {:#010x}, boot_info: {:#010x} }}",
            self.boot_magic, self.boot_info
        )
    }
}
