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
mod test {
    use super::*;
    use crate::signal::sigset_t;

    #[test]
    fn test_sigemptyset() {
        let mut set: sigset_t = u64::MAX;
        assert_eq!(unsafe { sigemptyset(&mut set) }, 0);
        assert_eq!(set, 0);
    }

    #[test]
    fn test_sigfillset() {
        let mut set: sigset_t = 0;
        assert_eq!(unsafe { sigfillset(&mut set) }, 0);
        assert_eq!(set, u64::MAX);
    }

    #[test]
    fn test_sigaddset_and_sigismember() {
        let mut set: sigset_t = 0;
        assert_eq!(unsafe { sigemptyset(&mut set) }, 0);

        // Add signal 1.
        assert_eq!(unsafe { sigaddset(&mut set, 1) }, 0);
        assert_eq!(unsafe { sigismember(&set, 1) }, 1);
        assert_eq!(unsafe { sigismember(&set, 2) }, 0);

        // Add signal 15.
        assert_eq!(unsafe { sigaddset(&mut set, 15) }, 0);
        assert_eq!(unsafe { sigismember(&set, 15) }, 1);
        assert_eq!(unsafe { sigismember(&set, 1) }, 1);
    }

    #[test]
    fn test_sigdelset() {
        let mut set: sigset_t = 0;
        assert_eq!(unsafe { sigfillset(&mut set) }, 0);

        // Remove signal 9.
        assert_eq!(unsafe { sigdelset(&mut set, 9) }, 0);
        assert_eq!(unsafe { sigismember(&set, 9) }, 0);
        assert_eq!(unsafe { sigismember(&set, 10) }, 1);
    }

    #[test]
    fn test_sigset_invalid_signum() {
        let mut set: sigset_t = 0;
        assert_eq!(unsafe { sigaddset(&mut set, 0) }, -1);
        assert_eq!(unsafe { sigaddset(&mut set, 65) }, -1);
        assert_eq!(unsafe { sigdelset(&mut set, -1) }, -1);
        assert_eq!(unsafe { sigismember(&set, 0) }, -1);
    }

    #[test]
    fn test_sigset_null_pointer() {
        assert_eq!(unsafe { sigemptyset(core::ptr::null_mut()) }, -1);
        assert_eq!(unsafe { sigfillset(core::ptr::null_mut()) }, -1);
        assert_eq!(unsafe { sigaddset(core::ptr::null_mut(), 1) }, -1);
        assert_eq!(unsafe { sigdelset(core::ptr::null_mut(), 1) }, -1);
        assert_eq!(unsafe { sigismember(core::ptr::null(), 1) }, -1);
    }

    #[test]
    fn test_sigset_boundary_signals() {
        let mut set: sigset_t = 0;
        assert_eq!(unsafe { sigemptyset(&mut set) }, 0);

        // Signal 1 (minimum valid).
        assert_eq!(unsafe { sigaddset(&mut set, 1) }, 0);
        assert_eq!(unsafe { sigismember(&set, 1) }, 1);

        // Signal 64 (maximum valid).
        assert_eq!(unsafe { sigaddset(&mut set, 64) }, 0);
        assert_eq!(unsafe { sigismember(&set, 64) }, 1);
    }
}
