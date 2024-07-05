/*
 * Copyright(c) 2011-2024 The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==============================================================================
// Imports
//==============================================================================

use nanvix::misc;

//==============================================================================
// Private Standalone Functions
//==============================================================================

/// Checks if sizes are conformant.
fn check_sizes() -> bool {
    if core::mem::size_of::<misc::KernelModule>() != 72 {
        nanvix::log!("unexpected size for KernelModule");
        return false;
    }

    true
}

/// Attempts to retrieve information of a valid kernel module.
fn get_kmod_info() -> bool {
    // Attempt to get information of a kernel module.
    let mut kmod: misc::KernelModule = misc::KernelModule::default();
    let result: i32 = misc::kmod_get(&mut kmod, 0);

    // Check if we failed to get information of a kernel module.
    if result != 0 {
        nanvix::log!("failed to get information of a kernel module");
        return false;
    }

    true
}

/// Attempts to retrieve information of an invalid kernel module.
fn get_invalid_kmod_info() -> bool {
    // Attempt to get information of a kernel module.
    let mut kmod: misc::KernelModule = misc::KernelModule::default();
    let result: i32 = misc::kmod_get(&mut kmod, u32::MAX);

    // Check if we succeeded to get information of a kernel module.
    if result == 0 {
        nanvix::log!("succeded to get information of an invalid kernel module");
        return false;
    }

    true
}

//==============================================================================
// Public Standalone Functions
//==============================================================================

///
/// **Description**
///
/// Tests kernel calls in the misc facility.
///
pub fn test() {
    crate::test!(check_sizes());
    crate::test!(get_kmod_info());
    crate::test!(get_invalid_kmod_info());
}
