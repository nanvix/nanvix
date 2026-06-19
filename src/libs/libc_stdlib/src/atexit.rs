// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of atexit handlers that can be registered.
const MAX_ATEXIT_HANDLERS: usize = 32;

//==================================================================================================
// Global State
//==================================================================================================

/// Table of registered atexit handler functions.
static mut ATEXIT_HANDLERS: [Option<unsafe extern "C" fn()>; MAX_ATEXIT_HANDLERS] =
    [None; MAX_ATEXIT_HANDLERS];

/// Number of currently registered atexit handlers.
static mut ATEXIT_COUNT: usize = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Registers a function to be called at normal process termination.
///
/// # Parameters
///
/// - `func`: Function to be called at exit.
///
/// # Returns
///
/// `0` on success, or `-1` if the handler table is full or `func` is `None`.
///
/// # Safety
///
/// This function is unsafe because it accesses global mutable state.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/atexit.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn atexit(func: Option<unsafe extern "C" fn()>) -> c_int {
    let func = match func {
        Some(f) => f,
        None => return -1,
    };

    if ATEXIT_COUNT >= MAX_ATEXIT_HANDLERS {
        return -1;
    }

    ATEXIT_HANDLERS[ATEXIT_COUNT] = Some(func);
    ATEXIT_COUNT += 1;
    0
}

/// Invokes all registered atexit handlers in reverse order of registration.
pub(crate) unsafe fn call_atexit_handlers() {
    while ATEXIT_COUNT > 0 {
        ATEXIT_COUNT -= 1;
        if let Some(f) = ATEXIT_HANDLERS[ATEXIT_COUNT] {
            f();
            ATEXIT_HANDLERS[ATEXIT_COUNT] = None;
        }
    }
}

/// Resets the atexit handler table. Used for test isolation.
#[cfg(test)]
pub(crate) unsafe fn reset_atexit_handlers() {
    ATEXIT_COUNT = 0;
    // Use indexed assignment (a place expression) rather than `&mut ATEXIT_HANDLERS` to avoid
    // creating a reference to a mutable static. A `while` loop sidesteps `needless_range_loop`,
    // which would otherwise suggest the disallowed iterator form.
    let mut i = 0;
    while i < MAX_ATEXIT_HANDLERS {
        ATEXIT_HANDLERS[i] = None;
        i += 1;
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{
        atexit,
        reset_atexit_handlers,
    };

    unsafe extern "C" fn dummy_handler() {}

    #[test]
    fn register_handler() {
        unsafe { reset_atexit_handlers() };
        assert_eq!(unsafe { atexit(Some(dummy_handler)) }, 0);
        unsafe { reset_atexit_handlers() };
    }

    #[test]
    fn register_too_many() {
        unsafe { reset_atexit_handlers() };
        for _ in 0..32 {
            assert_eq!(unsafe { atexit(Some(dummy_handler)) }, 0);
        }
        assert_eq!(unsafe { atexit(Some(dummy_handler)) }, -1);
        unsafe { reset_atexit_handlers() };
    }
}
