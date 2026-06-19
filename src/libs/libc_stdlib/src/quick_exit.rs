// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of quick-exit handlers that can be registered.
const MAX_AT_QUICK_EXIT_HANDLERS: usize = 32;

//==================================================================================================
// Global State
//==================================================================================================

/// Table of registered quick-exit handler functions.
static mut AT_QUICK_EXIT_HANDLERS: [Option<unsafe extern "C" fn()>; MAX_AT_QUICK_EXIT_HANDLERS] =
    [None; MAX_AT_QUICK_EXIT_HANDLERS];

/// Number of currently registered quick-exit handlers.
static mut AT_QUICK_EXIT_COUNT: usize = 0;

//==================================================================================================
// External Declarations
//==================================================================================================

extern "C" {
    fn _exit(status: c_int) -> !;
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Registers a function to be called by [`quick_exit`].
///
/// # Parameters
///
/// - `func`: Function to be called at quick exit.
///
/// # Returns
///
/// `0` on success, or a non-zero value on failure.
///
/// # Safety
///
/// This function is unsafe because it accesses global mutable state.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/at_quick_exit.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn at_quick_exit(func: Option<unsafe extern "C" fn()>) -> c_int {
    let func = match func {
        Some(func) => func,
        None => return -1,
    };

    if AT_QUICK_EXIT_COUNT >= MAX_AT_QUICK_EXIT_HANDLERS {
        return -1;
    }

    AT_QUICK_EXIT_HANDLERS[AT_QUICK_EXIT_COUNT] = Some(func);
    AT_QUICK_EXIT_COUNT += 1;
    0
}

///
/// # Description
///
/// Terminates the process after calling handlers registered by [`at_quick_exit`].
///
/// # Parameters
///
/// - `status`: Exit status code.
///
/// # Safety
///
/// This function is unsafe because it terminates the process and calls registered handlers.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/quick_exit.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn quick_exit(status: c_int) -> ! {
    while AT_QUICK_EXIT_COUNT > 0 {
        AT_QUICK_EXIT_COUNT -= 1;
        if let Some(func) = AT_QUICK_EXIT_HANDLERS[AT_QUICK_EXIT_COUNT] {
            func();
            AT_QUICK_EXIT_HANDLERS[AT_QUICK_EXIT_COUNT] = None;
        }
    }
    _exit(status)
}

/// Resets the quick-exit handler table. Used for test isolation.
#[cfg(test)]
pub(crate) unsafe fn reset_at_quick_exit_handlers() {
    AT_QUICK_EXIT_COUNT = 0;
    let mut i: usize = 0;
    while i < MAX_AT_QUICK_EXIT_HANDLERS {
        AT_QUICK_EXIT_HANDLERS[i] = None;
        i += 1;
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{
        at_quick_exit,
        reset_at_quick_exit_handlers,
    };

    unsafe extern "C" fn dummy_handler() {}

    #[test]
    fn register_handler() {
        unsafe { reset_at_quick_exit_handlers() };
        assert_eq!(unsafe { at_quick_exit(Some(dummy_handler)) }, 0);
        unsafe { reset_at_quick_exit_handlers() };
    }
}
