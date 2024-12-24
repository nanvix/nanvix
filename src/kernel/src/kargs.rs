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
    /// Magic value multiboot.
    mboot_magic: u32,
    /// Address of multiboot information.
    mboot_info: usize,
}

// `KernelArguments` must be 8 bytes long. This must match low-level startup code.
sys::static_assert_size!(KernelArguments, 8);

// `KernelArguments` must be aligned to 4 bytes. This must match low-level startup code.
sys::static_assert_alignment!(KernelArguments, 4);

//==================================================================================================
// Implementations
//==================================================================================================

impl KernelArguments {
    /// Parses kernel arguments.
    #[cfg(feature = "mboot")]
    pub fn parse(&self) -> Result<BootInfo, Error> {
        crate::hal::platform::mboot::parse(self.mboot_magic, self.mboot_info)
    }

    /// Parses kernel arguments.
    #[cfg(not(feature = "mboot"))]
    pub fn parse(&self) -> Result<BootInfo, Error> {
        crate::hal::platform::parse_bootinfo(self.mboot_magic, self.mboot_info)
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl core::fmt::Debug for KernelArguments {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "kernel_arguments {{ mboot_magic: {:#010x}, mboot_info: {:#010x} }}",
            self.mboot_magic, self.mboot_info
        )
    }
}
