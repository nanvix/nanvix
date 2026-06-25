// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::{
    fcntl::{
        file_access_mode::{
            O_RDONLY,
            O_RDWR,
            O_WRONLY,
        },
        file_creation_flags::{
            O_CREAT,
            O_TRUNC,
        },
        file_status_flags::O_APPEND,
    },
    ffi::{
        c_char,
        c_int,
    },
    sys_types::mode_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reopens the stream `stream`, associating it with the file named by `pathname`. The original
/// stream is flushed and its underlying file descriptor is retargeted to the newly opened file so
/// that existing `FILE *` references (such as `stdout`) remain valid. If `pathname` is null the
/// stream is left associated with its current descriptor (changing the access mode of an already
/// open descriptor is not supported).
///
/// # Parameters
///
/// - `pathname`: Pointer to a null-terminated path string, or null to keep the current file.
/// - `mode`: Pointer to a null-terminated mode string (`"r"`, `"w"`, `"a"`, `"r+"`, `"w+"`, or
///   `"a+"`).
/// - `stream`: Pointer to the [`FILE`] stream to reopen.
///
/// # Returns
///
/// `stream` on success, or a null pointer on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `pathname`, when non-null, points to a valid null-terminated string.
/// - `mode` points to a valid, null-terminated mode string.
/// - `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/freopen.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn freopen(
    pathname: *const c_char,
    mode: *const c_char,
    stream: *mut FILE,
) -> *mut FILE {
    extern "C" {
        fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int;
        fn close(fd: c_int) -> c_int;
        fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    }

    if mode.is_null() || stream.is_null() {
        return core::ptr::null_mut();
    }

    // Parse the mode string, matching fopen(). Guard against an empty mode string so that the second
    // and third bytes are only read when they lie within the null-terminated string.
    let first: u8 = *mode as u8;
    if first == 0 {
        return core::ptr::null_mut();
    }
    let second: u8 = *mode.add(1) as u8;
    let third: u8 = if second != 0 { *mode.add(2) as u8 } else { 0 };
    let has_plus: bool = second == b'+' || third == b'+';

    let flags: c_int = match (first, has_plus) {
        (b'r', false) => O_RDONLY,
        (b'r', true) => O_RDWR,
        (b'w', false) => O_WRONLY | O_CREAT | O_TRUNC,
        (b'w', true) => O_RDWR | O_CREAT | O_TRUNC,
        (b'a', false) => O_WRONLY | O_CREAT | O_APPEND,
        (b'a', true) => O_RDWR | O_CREAT | O_APPEND,
        _ => return core::ptr::null_mut(),
    };

    // Flush any pending output before detaching from the old file.
    // SAFETY: stream is a valid FILE pointer.
    unsafe { crate::fflush::fflush(stream) };

    // With no pathname, leave the stream attached to its current descriptor.
    if pathname.is_null() {
        return stream;
    }

    let perm: mode_t = 0o666;
    // SAFETY: pathname is non-null, flags and permissions are valid.
    let newfd: c_int = unsafe { open(pathname, flags, perm) };
    if newfd < 0 {
        return core::ptr::null_mut();
    }

    let oldfd: c_int = (*stream).fd;
    if oldfd < 0 {
        // SAFETY: newfd is a valid open descriptor.
        unsafe { close(newfd) };
        return core::ptr::null_mut();
    }

    // Retarget the stream's descriptor at the freshly opened file.
    if newfd != oldfd {
        // SAFETY: both descriptors are valid; dup2 closes oldfd if necessary.
        if unsafe { dup2(newfd, oldfd) } < 0 {
            // SAFETY: newfd is a valid open descriptor.
            unsafe { close(newfd) };
            return core::ptr::null_mut();
        }
        // SAFETY: newfd has been duplicated onto oldfd and is no longer needed.
        unsafe { close(newfd) };
    }

    // Reset the stream's status indicators for the new file.
    (*stream).eof = 0;
    (*stream).error = 0;
    (*stream).ungetc_buf = -1;

    stream
}
