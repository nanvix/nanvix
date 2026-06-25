// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
        c_void,
    },
    netinet_in::{
        in_addr,
        in_addr_t,
    },
    sys_socket::socklen_t,
};
use ::syscall::errno::__errno_location;
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
    let mut addr: in_addr = in_addr { s_addr: 0 };
    if inet_aton(cp, &raw mut addr) != 0 {
        addr.s_addr
    } else {
        // INADDR_NONE.
        in_addr_t::MAX
    }
}

///
/// # Description
///
/// Converts an IPv4 address from a 32-bit binary representation to a dotted-decimal string.
///
/// # Parameters
///
/// - `addr`: Structure containing the IPv4 address in network byte order.
///
/// # Returns
///
/// The `inet_ntoa()` function returns a pointer to a statically allocated string containing the
/// dotted-decimal representation of the address. On error, it returns null.
///
/// # Safety
///
/// This function is unsafe because it may return a pointer to a static buffer and does not
/// guarantee thread safety.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn inet_ntoa(addr: in_addr) -> *const c_char {
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

///
/// # Description
///
/// Parses a single numeric component of a dotted address, honoring the historical C-library base
/// conventions: a `0x`/`0X` prefix selects hexadecimal, a leading `0` selects octal, and otherwise
/// the value is decimal.
///
/// # Returns
///
/// A tuple `(value, consumed, ok)` where `consumed` is the number of bytes accepted and `ok` is
/// `false` if no valid number was found or the value overflowed 32 bits.
///
/// # Safety
///
/// The caller must ensure `cp` points to a valid null-terminated string and `start` is within it.
///
unsafe fn parse_component(cp: *const c_char, start: usize) -> (u32, usize, bool) {
    let mut i: usize = start;
    let first: u8 = *cp.add(i) as u8;
    if !first.is_ascii_digit() {
        return (0, 0, false);
    }

    let mut base: u64 = 10;
    if first == b'0' {
        let next: u8 = *cp.add(i + 1) as u8;
        if next == b'x' || next == b'X' {
            base = 16;
            i += 2;
        } else {
            base = 8;
            i += 1;
        }
    }

    let mut value: u64 = 0;
    let mut digits: usize = 0;
    loop {
        let c: u8 = *cp.add(i) as u8;
        let digit: u64 = match c {
            b'0'..=b'9' => u64::from(c - b'0'),
            b'a'..=b'f' => u64::from(c - b'a' + 10),
            b'A'..=b'F' => u64::from(c - b'A' + 10),
            _ => break,
        };
        if digit >= base {
            break;
        }
        value = value * base + digit;
        if value > u64::from(u32::MAX) {
            return (0, 0, false);
        }
        i += 1;
        digits += 1;
    }

    // A bare `0x` prefix with no following hex digits is not a valid number.
    if base == 16 && digits == 0 {
        return (0, 0, false);
    }

    (value as u32, i - start, true)
}

///
/// # Description
///
/// Converts an IPv4 address from dotted-decimal (or the historical octal/hex and short) notation
/// to a 32-bit binary representation, storing the result in network byte order.
///
/// The accepted forms are `a.b.c.d`, `a.b.c`, `a.b`, and `a`, where each component may be decimal,
/// octal (leading `0`), or hexadecimal (leading `0x`), matching the classic `inet_aton()` contract.
///
/// # Parameters
///
/// - `cp`: Pointer to a null-terminated string containing the address.
/// - `inp`: Pointer to the [`in_addr`] that receives the parsed address; may be null to only
///   validate the input.
///
/// # Returns
///
/// `1` if the address is valid, `0` otherwise.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `cp` points to a valid null-terminated string.
/// - `inp` is null or points to a valid [`in_addr`].
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn inet_aton(cp: *const c_char, inp: *mut in_addr) -> c_int {
    if cp.is_null() {
        return 0;
    }

    let mut parts: [u32; 4] = [0; 4];
    let mut nparts: usize = 0;
    let mut i: usize = 0;

    loop {
        let (value, consumed, ok) = parse_component(cp, i);
        if !ok || nparts >= 4 {
            return 0;
        }
        parts[nparts] = value;
        nparts += 1;
        i += consumed;

        let c: u8 = *cp.add(i) as u8;
        if c == b'.' {
            i += 1;
        } else if c == 0 {
            break;
        } else {
            return 0;
        }
    }

    // Assemble the address per the classic component-count rules, validating each field's range.
    let addr: u32 = match nparts {
        1 => parts[0],
        2 => {
            if parts[0] > 0xff || parts[1] > 0x00ff_ffff {
                return 0;
            }
            (parts[0] << 24) | parts[1]
        },
        3 => {
            if parts[0] > 0xff || parts[1] > 0xff || parts[2] > 0xffff {
                return 0;
            }
            (parts[0] << 24) | (parts[1] << 16) | parts[2]
        },
        4 => {
            if parts[0] > 0xff || parts[1] > 0xff || parts[2] > 0xff || parts[3] > 0xff {
                return 0;
            }
            (parts[0] << 24) | (parts[1] << 16) | (parts[2] << 8) | parts[3]
        },
        _ => return 0,
    };

    if !inp.is_null() {
        // Store in network (big-endian) byte order.
        (*inp).s_addr = addr.to_be();
    }

    1
}
