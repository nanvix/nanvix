// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::platform::bootinfo::BootInfo;
use ::core::sync::atomic::{
    AtomicPtr,
    AtomicUsize,
    Ordering,
};
use ::sys::error::Error;

//==================================================================================================
// Global State
//==================================================================================================

/// Pointer to the kernel arguments string data, set once during boot.
static KERNEL_ARGS_PTR: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());

/// Length of the kernel arguments string, set once during boot.
static KERNEL_ARGS_LEN: AtomicUsize = AtomicUsize::new(0);

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
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Stores the kernel arguments string during boot.
///
/// # Safety
///
/// Must be called exactly once during boot, before any user process is started.
/// The referenced string must have `'static` lifetime.
///
pub unsafe fn set_kernel_args(args: &'static str) {
    KERNEL_ARGS_PTR.store(args.as_ptr() as *mut u8, Ordering::Release);
    KERNEL_ARGS_LEN.store(args.len(), Ordering::Release);
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
