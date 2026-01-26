// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
        c_void,
    },
    netinet_in::in_addr_t,
    sys_socket::socklen_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts an IPv4 address from the dotted-decimal string format to a 32-bit binary representation.
///
/// # Parameters
///
/// - `cp`: Pointer to a null-terminated string containing the IPv4 address in dotted-decimal notation.
///
/// # Returns
///
/// The `inet_addr()` function returns the address in network byte order as an `in_addr_t` on
/// success.  On error, it returns `-1` cast to `in_addr_t`.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `cp` points to a valid null-terminated string.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn inet_addr(cp: *const c_char) -> in_addr_t {
    // TODO: https://github.com/nanvix/nanvix/issues/594.
    ::syslog::debug!("inet_addr(): not implemented");
    in_addr_t::MAX
}

///
/// # Description
///
/// Converts an IPv4 address from a 32-bit binary representation to a dotted-decimal string.
///
/// # Parameters
///
/// - `in_addr`: Structure containing the IPv4 address in network byte order.
///
/// # Returns
///
/// The `inet_ntoa()` function returns a pointer to a statically allocated string containing the
/// dotted-decimal representation of the address. On error, it returns null.
///
/// # Safety
///
/// This function is unsafe because it may return a pointer to a static buffer and does not guarantee thread safety.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn inet_ntoa(in_addr: in_addr_t) -> *const c_char {
    // TODO: https://github.com/nanvix/nanvix/issues/595.
    ::syslog::debug!("inet_ntoa(): not implemented");
    core::ptr::null()
}

///
/// # Description
///
/// Converts an IP address from binary form to text form.
///
/// # Parameters
///
/// - `af`: Address family (e.g., AF_INET or AF_INET6).
/// - `src`: Pointer to the binary address.
/// - `dst`: Pointer to the buffer where the text representation will be stored.
/// - `size`: Size of the buffer.
///
/// # Returns
///
/// The `inet_ntop()` function returns a pointer to the buffer containing the text representation of the address on success.
/// On error, it returns null and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `src` points to a valid address structure.
/// - `dst` points to a valid buffer of at least `size` bytes.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn inet_ntop(
    af: c_int,
    src: *const c_void,
    dst: *mut c_char,
    size: socklen_t,
) -> *const c_char {
    // TODO: https://github.com/nanvix/nanvix/issues/592.
    ::syslog::debug!("inet_ntop(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    core::ptr::null()
}

///
/// # Description
///
/// Converts an IP address from text form to binary form.
///
/// # Parameters
///
/// - `af`: Address family (e.g., AF_INET or AF_INET6).
/// - `src`: Pointer to the null-terminated string containing the IP address in text form.
/// - `dst`: Pointer to the buffer where the binary address will be stored.
///
/// # Returns
///
/// The `inet_pton()` function returns `1` on success, `0` if the input is not a valid address, and `-1` on error
/// (setting `errno` to indicate the error).
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `src` points to a valid null-terminated string.
/// - `dst` points to a valid buffer for the binary address.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn inet_pton(af: c_int, src: *const c_char, dst: *mut c_void) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/593.
    ::syslog::debug!("inet_pton(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}
