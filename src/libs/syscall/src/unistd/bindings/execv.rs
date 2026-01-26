// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Executes a program by replacing the current process image. The `execv()` function replaces the
/// current process image with a new process image specified by `path`. The new process image is
/// constructed from the executable file whose pathname is pointed to by `path`. The `argv` argument
/// is an array of character pointers to null-terminated strings that represent the argument list
/// available to the new program. The first argument, by convention, points to the filename associated
/// with the file being executed. The array of pointers must be terminated by a null pointer.
/// This function is one of the exec family of functions that provide different interfaces for
/// program execution and process replacement.
///
/// # Parameters
///
/// - `path`: Pathname of the executable file to execute. This must be a valid null-terminated
///   string specifying either an absolute or relative path to an executable file. The file must
///   have appropriate execute permissions for the calling process. The path is resolved relative
///   to the current working directory if it is not an absolute path.
/// - `argv`: Argument vector for the new program. This is an array of pointers to null-terminated
///   strings that represent the command-line arguments to be passed to the new program. By
///   convention, `argv[0]` should point to the filename of the program being executed. The array
///   must be terminated by a null pointer (`NULL`). Each string in the array represents a separate
///   argument that will be available to the new program through its main function parameters.
///
/// # Returns
///
/// Upon successful completion, `execv()` does not return to the calling program because the process
/// image is completely replaced. If the function fails, it returns `-1` and sets `errno` to indicate
/// the error. The calling process continues execution at the point of the failed `execv()` call.
/// Common error conditions include file not found, permission denied, invalid executable format,
/// insufficient memory, or invalid argument pointers.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - `path` remains valid for the duration of the function call.
/// - `argv` points to a valid array of character pointers.
/// - Each non-null pointer in `argv` points to a valid null-terminated string.
/// - The `argv` array is properly terminated with a null pointer.
/// - All strings referenced by `argv` remain valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execv(path: *const c_char, argv: *const *const c_char) -> c_int {
    // TODO:https://github.com/nanvix/nanvix/issues/588
    ::syslog::debug!("execv(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
