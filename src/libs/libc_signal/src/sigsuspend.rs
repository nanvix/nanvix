// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    set_errno,
    signal::sigset_t,
};
use ::sysapi::{
    errno::EINTR,
    ffi::c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Replaces the calling process's signal mask with `mask` and suspends the process until a signal
/// is delivered.
///
/// Nanvix does not deliver asynchronous signals, so no signal can ever resume a suspended process.
/// POSIX specifies that `sigsuspend()` always returns `-1` with `errno` set to `EINTR` once it is
/// interrupted; with no delivery this reports that outcome immediately rather than blocking
/// forever.
///
/// # Parameters
///
/// - `mask`: The signal mask to install while suspended (ignored).
///
/// # Returns
///
/// Always returns `-1` with `errno` set to `EINTR`.
///
/// # Safety
///
/// This function is unsafe because it is part of the C ABI surface; `mask` is not dereferenced.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sigsuspend.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigsuspend(_mask: *const sigset_t) -> c_int {
    set_errno(EINTR);
    -1
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn sigsuspend_always_reports_interruption() {
        let mask: sigset_t = 0;
        // SAFETY: `mask` points to a valid `sigset_t` for the duration of the call.
        assert_eq!(unsafe { sigsuspend(&mask) }, -1);
    }
}
