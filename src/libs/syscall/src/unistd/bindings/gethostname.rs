// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd;
use ::alloc::{
    ffi::CString,
    string::String,
};
use ::core::slice;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the name of the current host. The `gethostname()` function retrieves the standard host
/// name for the current processor and copies it into the buffer pointed to by `name`. The host
/// name is a string that identifies the system on a network and is typically set during system
/// initialization or through administrative commands. This function provides a way for applications
/// to determine the network identity of the system they are running on, which is useful for
/// logging, network communication, and system identification purposes. The returned host name
/// is null-terminated and may be truncated if the provided buffer is too small.
///
/// # Parameters
///
/// - `name`: Pointer to a buffer where the host name will be stored. This must be a valid
///   pointer to a writable memory area of at least `namelen` bytes. The buffer will receive
///   the null-terminated host name string. The buffer must remain valid and writable for the
///   duration of the function call.
/// - `namelen`: Size of the buffer in bytes, including space for the null terminator. This value
///   must be greater than `0` and not exceed `isize::MAX`. If the host name is longer than
///   `namelen - 1` characters, it will be truncated to fit in the buffer with a null terminator.
///
/// # Returns
///
/// The `gethostname()` function returns `0` on success, indicating that the host name has been
/// successfully copied to the provided buffer. On error, it returns `-1` and the contents of
/// the buffer are undefined. Common error conditions include invalid buffer pointer, invalid
/// buffer size, or internal system errors during host name retrieval.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `name` points to a valid memory location with space for at least `namelen` bytes.
/// - `name` remains valid and writable for the duration of the function call.
/// - `name` is properly aligned for `c_char` access.
/// - `namelen` is greater than `0` and does not exceed `isize::MAX`.
/// - The memory referenced by `name` is not concurrently accessed by other threads.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethostname(name: *mut c_char, namelen: c_size_t) -> c_int {
    let buf: &mut [u8] = {
        // Check if the buffer is invalid.
        if name.is_null() {
            ::syslog::error!("gethostname(): invalid buffer (name={name:?}, namelen={namelen:?})");
            return -1;
        }

        // Check if `namelen` is invalid.
        if namelen == 0 || namelen as usize >= isize::MAX as usize {
            ::syslog::error!(
                "gethostname(): invalid buffer size (name={name:?}, namelen={namelen:?})"
            );
            return -1;
        }
        slice::from_raw_parts_mut(name as *mut u8, namelen as usize)
    };

    // Get the host name.
    let hostname: String = unistd::gethostname();

    // Attempt to convert Rust string to C string and check for errors.
    let c_string: CString = match CString::new(hostname) {
        // Success.
        Ok(s) => s,
        // Failure.
        Err(_error) => {
            ::syslog::error!(
                "gethostname(): failed to convert string (name={name:?}, namelen={namelen:?})",
            );
            return -1;
        },
    };

    let bytes: &[u8] = c_string.as_bytes_with_nul();

    // Copy hostname to buffer.
    if bytes.len() > buf.len() {
        // Hostname is too long, truncate it.
        let bytes_len_truncated: usize = buf.len() - 1;

        ::syslog::warn!(
            "gethostname(): hostname is too long, truncating (name={name:?}, namelen={namelen:?})"
        );
        buf[..bytes_len_truncated].copy_from_slice(&bytes[..bytes_len_truncated]);
        buf[bytes_len_truncated] = b'\0';
    } else {
        // Hostname fits in the buffer, copy it as is.
        buf[..bytes.len()].copy_from_slice(bytes);
    }

    0
}
