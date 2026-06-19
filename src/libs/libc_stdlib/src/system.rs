// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::set_errno;
use ::sysapi::{
    errno::ENOSYS,
    ffi::{
        c_char,
        c_int,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Executes a shell command.
///
/// # Parameters
///
/// - `command`: The command to execute, or null to query for a command processor.
///
/// # Returns
///
/// Non-zero when `command` is null; otherwise -1 with `errno` set to `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it may dereference `command`; the pointer is, however, not
/// dereferenced by this stub.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/system.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn system(command: *const c_char) -> c_int {
    if command.is_null() {
        return 1;
    }
    set_errno(ENOSYS);
    -1
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::system;
    use ::sysapi::ffi::c_char;

    #[test]
    fn null_command_reports_processor() {
        assert_ne!(unsafe { system(::core::ptr::null()) }, 0);
    }

    #[test]
    fn non_null_command_returns_error() {
        let command = b"true\0";
        assert_eq!(unsafe { system(command.as_ptr().cast::<c_char>()) }, -1);
    }
}
