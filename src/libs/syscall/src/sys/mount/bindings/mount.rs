// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::ffi;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_ulong,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Mounts a filesystem at the specified target path. The `mount()` function attaches the
/// filesystem specified by `source` to the directory specified by `target`, using the filesystem
/// type given by `fstype`.
///
/// # Parameters
///
/// - `source`: Pointer to a null-terminated string specifying the source device or path. May be
///   empty for virtual filesystems.
/// - `target`: Pointer to a null-terminated string specifying the target mount point in the guest
///   VFS namespace.
/// - `fstype`: Pointer to a null-terminated string specifying the filesystem type (e.g.,
///   "hostfs").
/// - `flags`: Mount flags (reserved, pass 0).
///
/// # Returns
///
/// The `mount()` function returns `0` on success. On error, it returns `-1` and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `source`, `target`, and `fstype` each point to a valid null-terminated string.
/// - All pointers remain valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
// `flags as u64` widens `c_ulong` (u32) on x86 but is an identity cast on x86_64 (c_ulong = u64).
#[cfg_attr(target_arch = "x86_64", allow(clippy::unnecessary_cast))]
pub unsafe extern "C" fn mount(
    source: *const c_char,
    target: *const c_char,
    fstype: *const c_char,
    flags: c_ulong,
) -> c_int {
    // Check if `source` is invalid.
    if source.is_null() {
        ::syslog::warn!("mount(): source is null");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `target` is invalid.
    if target.is_null() {
        ::syslog::warn!("mount(): target is null");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `fstype` is invalid.
    if fstype.is_null() {
        ::syslog::warn!("mount(): fstype is null");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `source`.
    let source_str: &str = match ffi::CStr::from_ptr(source).to_str() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::warn!("mount(): invalid source string");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `target`.
    let target_str: &str = match ffi::CStr::from_ptr(target).to_str() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::warn!("mount(): invalid target string");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert `fstype`.
    let fstype_str: &str = match ffi::CStr::from_ptr(fstype).to_str() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::warn!("mount(): invalid fstype string");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to mount and check for errors.
    match crate::sys::mount::mount(source_str, target_str, fstype_str, flags as u64) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::warn!(
                "mount(): {error:?} (source={source_str:?}, target={target_str:?}, \
                 fstype={fstype_str:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
