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

// `KernelArguments` must match low-level startup code.
#[cfg(target_arch = "x86")]
::static_assert::assert_eq_size!(KernelArguments, 8);
#[cfg(target_arch = "x86_64")]
::static_assert::assert_eq_size!(KernelArguments, 16);

// `KernelArguments` must be aligned to match low-level startup code.
#[cfg(target_arch = "x86")]
::static_assert::assert_eq_align!(KernelArguments, 4);
#[cfg(target_arch = "x86_64")]
::static_assert::assert_eq_align!(KernelArguments, 8);

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

///
/// # Description
///
/// Returns the kernel arguments string that was stored during boot.
///
/// # Returns
///
/// The kernel arguments string, or an empty string if none were set.
///
#[cfg_attr(not(feature = "test"), allow(dead_code))]
pub fn get_kernel_args() -> &'static str {
    let ptr = KERNEL_ARGS_PTR.load(Ordering::Acquire);
    let len = KERNEL_ARGS_LEN.load(Ordering::Acquire);
    if ptr.is_null() || len == 0 {
        return "";
    }
    // SAFETY: the pointer and length were set from a valid `&'static str` during boot.
    unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len)) }
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
