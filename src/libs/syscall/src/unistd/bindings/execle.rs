// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::ffi::VaList;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of argument pointers, including the terminating null. Longer argument lists fail
/// with `E2BIG`.
const MAX_ARGV: usize = 256;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Replaces the current process image with a new one, passing the program arguments as an explicit
/// variadic list terminated by a null pointer. A pointer to the environment array follows the
/// terminating null pointer.
///
/// # Parameters
///
/// - `path`: Null-terminated path to the executable image.
/// - `arg`: First argument (conventionally the program name).
/// - Variadic: remaining null-terminated arguments, a `(char *)NULL` terminator, then a
///   `char *const envp[]` environment array.
///
/// # Returns
///
/// Does not return on success; returns `-1` with `errno` set on failure.
///
/// # Safety
///
/// This function is unsafe because it reads a variadic argument list and dereferences raw pointers.
/// The caller must terminate the argument list with a null pointer followed by a valid environment
/// array, and ensure every argument points to a valid null-terminated string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/execle.html>
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execle(path: *const c_char, arg: *const c_char, args: ...) -> c_int {
    extern "C" {
        fn execve(
            path: *const c_char,
            argv: *const *const c_char,
            envp: *const *const c_char,
        ) -> c_int;
    }

    let mut ap: VaList<'_> = args;
    let mut argv: [*const c_char; MAX_ARGV] = [::core::ptr::null(); MAX_ARGV];

    // `arg` is argv[0] (conventionally the program name). The argument vector is terminated by the
    // first null pointer in the variadic list, so termination is tracked solely from the variadic
    // arguments. The environment pointer immediately follows that null terminator.
    argv[0] = arg;
    let mut n: usize = 1;
    let mut terminated: bool = false;
    while n < MAX_ARGV {
        let next: *const c_char = unsafe { ap.next_arg::<*const c_char>() };
        argv[n] = next;
        n += 1;
        if next.is_null() {
            terminated = true;
            break;
        }
    }
    if !terminated {
        unsafe {
            *__errno_location() = ErrorCode::TooBig.get();
        }
        return -1;
    }

    let envp: *const *const c_char = unsafe { ap.next_arg::<*const *const c_char>() };

    unsafe { execve(path, argv.as_ptr(), envp) }
}
