// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    set_errno,
    signal::{
        sigset_t,
        SIG_MAX,
    },
};
use ::sysapi::{
    errno::{
        EFAULT,
        EINVAL,
    },
    ffi::c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes a signal set to be empty (all signals excluded).
///
/// # Parameters
///
/// - `set`: Pointer to the signal set to initialize.
///
/// # Returns
///
/// Zero on success, or -1 if `set` is null.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `set`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigemptyset(set: *mut sigset_t) -> c_int {
    if set.is_null() {
        // POSIX: EFAULT when `set` points to invalid memory.
        set_errno(EFAULT);
        return -1;
    }
    *set = 0;
    0
}

///
/// # Description
///
/// Initializes a signal set to be full (all signals included).
///
/// # Parameters
///
/// - `set`: Pointer to the signal set to initialize.
///
/// # Returns
///
/// Zero on success, or -1 if `set` is null.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `set`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigfillset(set: *mut sigset_t) -> c_int {
    if set.is_null() {
        // POSIX: EFAULT when `set` points to invalid memory.
        set_errno(EFAULT);
        return -1;
    }
    *set = u64::MAX;
    0
}

///
/// # Description
///
/// Tests whether a signal set is empty (GNU extension).
///
/// # Parameters
///
/// - `set`: Pointer to the signal set to test.
///
/// # Returns
///
/// `1` if the set is empty, `0` if it contains at least one signal, or `-1` if `set` is null.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `set`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigisemptyset(set: *const sigset_t) -> c_int {
    if set.is_null() {
        // POSIX: EFAULT when `set` points to invalid memory.
        set_errno(EFAULT);
        return -1;
    }
    if *set == 0 {
        1
    } else {
        0
    }
}

///
/// # Description
///
/// Adds the specified signal to the signal set.
///
/// # Parameters
///
/// - `set`: Pointer to the signal set.
/// - `signum`: The signal number to add (1-based).
///
/// # Returns
///
/// Zero on success, or -1 if `set` is null or `signum` is out of range.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `set`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigaddset(set: *mut sigset_t, signum: c_int) -> c_int {
    if set.is_null() {
        // POSIX: EFAULT when `set` points to invalid memory.
        set_errno(EFAULT);
        return -1;
    }
    if signum <= 0 || signum > SIG_MAX {
        // POSIX: EINVAL when `signum` is not a valid signal number.
        set_errno(EINVAL);
        return -1;
    }
    // `signum` is validated to be in `1..=SIG_MAX`, so `(signum - 1)` is a non-negative shift
    // amount that fits in `u32`.
    *set |= 1u64 << ((signum - 1) as u32);
    0
}

///
/// # Description
///
/// Removes the specified signal from the signal set.
///
/// # Parameters
///
/// - `set`: Pointer to the signal set.
/// - `signum`: The signal number to remove (1-based).
///
/// # Returns
///
/// Zero on success, or -1 if `set` is null or `signum` is out of range.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `set`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigdelset(set: *mut sigset_t, signum: c_int) -> c_int {
    if set.is_null() {
        // POSIX: EFAULT when `set` points to invalid memory.
        set_errno(EFAULT);
        return -1;
    }
    if signum <= 0 || signum > SIG_MAX {
        // POSIX: EINVAL when `signum` is not a valid signal number.
        set_errno(EINVAL);
        return -1;
    }
    // `signum` is validated to be in `1..=SIG_MAX`, so `(signum - 1)` is a non-negative shift
    // amount that fits in `u32`.
    *set &= !(1u64 << ((signum - 1) as u32));
    0
}

///
/// # Description
///
/// Tests whether the specified signal is a member of the signal set.
///
/// # Parameters
///
/// - `set`: Pointer to the signal set.
/// - `signum`: The signal number to test (1-based).
///
/// # Returns
///
/// 1 if `signum` is a member of the set, 0 if it is not, or -1 on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `set`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigismember(set: *const sigset_t, signum: c_int) -> c_int {
    if set.is_null() {
        // POSIX: EFAULT when `set` points to invalid memory.
        set_errno(EFAULT);
        return -1;
    }
    if signum <= 0 || signum > SIG_MAX {
        // POSIX: EINVAL when `signum` is not a valid signal number.
        set_errno(EINVAL);
        return -1;
    }
    // `signum` is validated to be in `1..=SIG_MAX`, so `(signum - 1)` is a non-negative shift
    // amount that fits in `u32`.
    if *set & (1u64 << ((signum - 1) as u32)) != 0 {
        1
    } else {
        0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
