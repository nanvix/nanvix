// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "syscall")]
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Unlocks all pages mapped into the address space of the calling process, and stops locking the
/// pages that become mapped later on, undoing the effects of `mlockall()` and `mlock()`.
///
/// Nanvix never swaps or pages out memory, so every page is resident from the moment it becomes
/// mapped until it is unmapped, whether it is locked or not. Unlocking therefore has no effect, and
/// this function always succeeds.
///
/// # Returns
///
/// Always returns `0`.
///
/// # Known Limitations (Nanvix)
///
/// - Unlocking is a no-op: mapped pages are always resident.
///
#[cfg(feature = "syscall")]
#[unsafe(no_mangle)]
pub extern "C" fn munlockall() -> c_int {
    0
}
