// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ptr;
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
    sys_socket::{
        socket_address_family::{
            AF_INET,
            AF_INET6,
        },
        socklen_t,
    },
};
use ::syscall::errno::__errno_location;
use ::syslog::trace_libcall;

//==================================================================================================
// Constants
//==================================================================================================

/// Size of the buffer required to hold the longest IPv4 address string, including the terminating
/// NUL (`"255.255.255.255"`).
const INET_ADDRSTRLEN: usize = 16;

/// Size of the buffer required to hold the longest IPv6 address string, including the terminating
/// NUL (`"ffff:ffff:ffff:ffff:ffff:ffff:255.255.255.255"`).
const INET6_ADDRSTRLEN: usize = 46;

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
    // POSIX permits the result to point at static storage that subsequent calls overwrite. The
    // buffer is sized for the longest IPv4 dotted-decimal string plus its terminating NUL.
    static mut BUFFER: [u8; INET_ADDRSTRLEN] = [0; INET_ADDRSTRLEN];

    // `s_addr` is held in network byte order; recover the host-order value so the octets can be
    // extracted most-significant first.
    let host: u32 = u32::from_be(addr.s_addr);
    let octets: [u8; 4] = [
        (host >> 24) as u8,
        (host >> 16) as u8,
        (host >> 8) as u8,
        host as u8,
    ];

    // Format into a local buffer first, then publish to the static through a raw pointer.
    // Raw-pointer access avoids constructing a reference to the mutable static (see
    // `static_mut_refs`).
    let mut scratch: [u8; INET_ADDRSTRLEN] = [0; INET_ADDRSTRLEN];
    let len: usize = format_ipv4(octets, &mut scratch).unwrap_or(0);
    let buffer: *mut u8 = ptr::addr_of_mut!(BUFFER) as *mut u8;
    // SAFETY: `len < INET_ADDRSTRLEN`, so copying `len + 1` bytes (string plus NUL) stays within
    // the bounds of `BUFFER`; the source and destination do not overlap.
    ptr::copy_nonoverlapping(scratch.as_ptr(), buffer, len + 1);

    buffer as *const c_char
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
    // Format into a scratch buffer large enough for any supported family, then copy it out only
    // when the caller's buffer can hold the whole string.
    let mut tmp: [u8; INET6_ADDRSTRLEN] = [0; INET6_ADDRSTRLEN];
    let result: Option<usize> = match af {
        AF_INET => format_in_addr(src, &mut tmp),
        AF_INET6 => format_ipv6(src as *const u8, &mut tmp),
        _ => {
            *__errno_location() = ErrorCode::AddressFamilyNotSupported.get();
            return ptr::null();
        },
    };

    let len: usize = match result {
        Some(len) => len,
        None => {
            *__errno_location() = ErrorCode::NoSpaceOnDevice.get();
            return ptr::null();
        },
    };

    // The full string plus its terminating NUL must fit within the caller's buffer.
    if len + 1 > size as usize {
        *__errno_location() = ErrorCode::NoSpaceOnDevice.get();
        return ptr::null();
    }

    // Copy the formatted string, including its terminating NUL, into the caller's buffer.
    for (i, &byte) in tmp[..=len].iter().enumerate() {
        *dst.add(i) = byte as c_char;
    }

    dst
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
    match af {
        AF_INET => match parse_ipv4(src) {
            Some(octets) => {
                let out: *mut u8 = dst as *mut u8;
                for (i, &byte) in octets.iter().enumerate() {
                    *out.add(i) = byte;
                }
                1
            },
            None => 0,
        },
        AF_INET6 => match parse_ipv6(src) {
            Some(bytes) => {
                let out: *mut u8 = dst as *mut u8;
                for (i, &byte) in bytes.iter().enumerate() {
                    *out.add(i) = byte;
                }
                1
            },
            None => 0,
        },
        _ => {
            *__errno_location() = ErrorCode::AddressFamilyNotSupported.get();
            -1
        },
    }
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

//==================================================================================================
// Private Helper Functions
//==================================================================================================

///
/// # Description
///
/// Writes the dotted-decimal representation of the four IPv4 `octets` into `dst`, starting at byte
/// offset `start`. No terminating NUL is written.
///
/// # Parameters
///
/// - `octets`: The four address bytes, most-significant first.
/// - `dst`: Destination byte buffer.
/// - `start`: Offset into `dst` at which to begin writing.
///
/// # Returns
///
/// The offset just past the last byte written on success, or [`None`] if `dst` is too small.
///
fn write_ipv4(octets: [u8; 4], dst: &mut [u8], start: usize) -> Option<usize> {
    let mut len: usize = start;
    for (i, &octet) in octets.iter().enumerate() {
        if i != 0 {
            *dst.get_mut(len)? = b'.';
            len += 1;
        }
        if octet >= 100 {
            *dst.get_mut(len)? = b'0' + octet / 100;
            len += 1;
        }
        if octet >= 10 {
            *dst.get_mut(len)? = b'0' + (octet / 10) % 10;
            len += 1;
        }
        *dst.get_mut(len)? = b'0' + octet % 10;
        len += 1;
    }
    Some(len)
}

///
/// # Description
///
/// Writes the dotted-decimal representation of the four IPv4 `octets` into `dst`, followed by a
/// terminating NUL.
///
/// # Parameters
///
/// - `octets`: The four address bytes, most-significant first.
/// - `dst`: Destination byte buffer.
///
/// # Returns
///
/// The length of the string (excluding the NUL) on success, or [`None`] if `dst` is too small.
///
fn format_ipv4(octets: [u8; 4], dst: &mut [u8]) -> Option<usize> {
    let len: usize = write_ipv4(octets, dst, 0)?;
    *dst.get_mut(len)? = 0;
    Some(len)
}

///
/// # Description
///
/// Writes the lowercase hexadecimal representation of a 16-bit IPv6 group `value` into `dst`
/// starting at offset `start`, omitting leading zeros but always emitting at least one digit.
///
/// # Parameters
///
/// - `value`: The 16-bit group value.
/// - `dst`: Destination byte buffer.
/// - `start`: Offset into `dst` at which to begin writing.
///
/// # Returns
///
/// The offset just past the last byte written on success, or [`None`] if `dst` is too small.
///
fn write_hex16(value: u16, dst: &mut [u8], start: usize) -> Option<usize> {
    let mut len: usize = start;
    let mut started: bool = false;
    for shift in [12u32, 8, 4, 0] {
        let nibble: u8 = ((value >> shift) & 0xf) as u8;
        if nibble != 0 || started || shift == 0 {
            *dst.get_mut(len)? = if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + nibble - 10
            };
            len += 1;
            started = true;
        }
    }
    Some(len)
}

///
/// # Description
///
/// Finds the longest run of consecutive zero groups in an eight-group IPv6 address, which is the
/// run eligible for `::` compression. Runs shorter than two groups are not reported, matching the
/// canonical text representation rules.
///
/// # Parameters
///
/// - `words`: The eight 16-bit address groups.
///
/// # Returns
///
/// A tuple `(base, len)` giving the index and length of the longest qualifying zero run, or
/// `(-1, 0)` when there is none.
///
fn longest_zero_run(words: &[u16; 8]) -> (isize, usize) {
    let mut best_base: isize = -1;
    let mut best_len: usize = 0;
    let mut cur_base: isize = -1;
    let mut cur_len: usize = 0;

    for (i, &word) in words.iter().enumerate() {
        if word == 0 {
            if cur_base < 0 {
                cur_base = i as isize;
                cur_len = 1;
            } else {
                cur_len += 1;
            }
        } else if cur_base >= 0 {
            if best_base < 0 || cur_len > best_len {
                best_base = cur_base;
                best_len = cur_len;
            }
            cur_base = -1;
        }
    }
    if cur_base >= 0 && (best_base < 0 || cur_len > best_len) {
        best_base = cur_base;
        best_len = cur_len;
    }

    // A single zero group is written verbatim rather than compressed.
    if best_base >= 0 && best_len < 2 {
        return (-1, 0);
    }

    (best_base, best_len)
}

///
/// # Description
///
/// Formats the sixteen-byte IPv6 address at `src` into `dst` using `::` zero-run compression and
/// the trailing dotted-decimal form for IPv4-compatible and IPv4-mapped addresses, followed by a
/// terminating NUL.
///
/// # Parameters
///
/// - `src`: Pointer to the sixteen address bytes in network byte order.
/// - `dst`: Destination byte buffer.
///
/// # Returns
///
/// The length of the string (excluding the NUL) on success, or [`None`] if `dst` is too small.
///
/// # Safety
///
/// The caller must ensure that `src` points to at least sixteen readable bytes.
///
unsafe fn format_ipv6(src: *const u8, dst: &mut [u8]) -> Option<usize> {
    // Combine the sixteen bytes into eight 16-bit groups (network byte order).
    let mut words: [u16; 8] = [0; 8];
    for (i, word) in words.iter_mut().enumerate() {
        *word = ((*src.add(i * 2) as u16) << 8) | (*src.add(i * 2 + 1) as u16);
    }

    let (best_base, best_len) = longest_zero_run(&words);

    let mut len: usize = 0;
    let mut i: usize = 0;
    while i < 8 {
        // Inside the compressed run of zero groups?
        if best_base >= 0 && i >= best_base as usize && i < best_base as usize + best_len {
            if i == best_base as usize {
                *dst.get_mut(len)? = b':';
                len += 1;
            }
            i += 1;
            continue;
        }

        // Emit a separator before each group other than the first.
        if i != 0 {
            *dst.get_mut(len)? = b':';
            len += 1;
        }

        // IPv4-compatible and IPv4-mapped addresses end in dotted-decimal form.
        if i == 6 && best_base == 0 && (best_len == 6 || (best_len == 5 && words[5] == 0xffff)) {
            let octets: [u8; 4] = [*src.add(12), *src.add(13), *src.add(14), *src.add(15)];
            len = write_ipv4(octets, dst, len)?;
            break;
        }

        len = write_hex16(words[i], dst, len)?;
        i += 1;
    }

    // A zero run that extends to the final group needs a closing colon.
    if best_base >= 0 && best_base as usize + best_len == 8 {
        *dst.get_mut(len)? = b':';
        len += 1;
    }

    *dst.get_mut(len)? = 0;
    Some(len)
}

///
/// # Description
///
/// Formats the IPv4 address held in the [`in_addr`] structure at `src` into `dst`, followed by a
/// terminating NUL.
///
/// # Parameters
///
/// - `src`: Pointer to an [`in_addr`] in network byte order.
/// - `dst`: Destination byte buffer.
///
/// # Returns
///
/// The length of the string (excluding the NUL) on success, or [`None`] if `dst` is too small.
///
/// # Safety
///
/// The caller must ensure that `src` points to a readable [`in_addr`].
///
unsafe fn format_in_addr(src: *const c_void, dst: &mut [u8]) -> Option<usize> {
    let bytes: *const u8 = src as *const u8;
    let octets: [u8; 4] = [*bytes, *bytes.add(1), *bytes.add(2), *bytes.add(3)];
    format_ipv4(octets, dst)
}

///
/// # Description
///
/// Returns the value of a single hexadecimal digit, or [`None`] if `ch` is not one.
///
fn hex_value(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        b'A'..=b'F' => Some(ch - b'A' + 10),
        _ => None,
    }
}

///
/// # Description
///
/// Parses a strict dotted-decimal IPv4 address (`"a.b.c.d"`, each field in the range `0..=255`
/// with no leading zeros, octal, or hexadecimal). This is the form accepted by `inet_pton()`,
/// which is stricter than `inet_aton()`.
///
/// # Parameters
///
/// - `src`: Pointer to a null-terminated candidate string.
///
/// # Returns
///
/// The four address bytes in network order on success, or [`None`] if the input is not a valid
/// strict dotted-decimal address.
///
/// # Safety
///
/// The caller must ensure that `src` is null or points to a valid null-terminated string.
///
unsafe fn parse_ipv4(src: *const c_char) -> Option<[u8; 4]> {
    if src.is_null() {
        return None;
    }

    let mut octets: [u8; 4] = [0; 4];
    let mut noctets: usize = 0;
    let mut saw_digit: bool = false;
    let mut i: usize = 0;

    loop {
        let ch: u8 = *src.add(i) as u8;
        if ch == 0 {
            break;
        }
        i += 1;

        if ch.is_ascii_digit() {
            let digit: u8 = ch - b'0';
            if saw_digit {
                // A leading zero (for example "01") is not a valid field.
                if octets[noctets - 1] == 0 {
                    return None;
                }
                let value: u16 = octets[noctets - 1] as u16 * 10 + digit as u16;
                if value > 255 {
                    return None;
                }
                octets[noctets - 1] = value as u8;
            } else {
                if noctets >= 4 {
                    return None;
                }
                octets[noctets] = digit;
                noctets += 1;
                saw_digit = true;
            }
        } else if ch == b'.' && saw_digit {
            if noctets == 4 {
                return None;
            }
            saw_digit = false;
        } else {
            return None;
        }
    }

    if noctets != 4 {
        return None;
    }

    Some(octets)
}

///
/// # Description
///
/// Parses an IPv6 address in any of its canonical textual forms, including `::` zero compression
/// and a trailing embedded IPv4 address.
///
/// # Parameters
///
/// - `src`: Pointer to a null-terminated candidate string.
///
/// # Returns
///
/// The sixteen address bytes in network order on success, or [`None`] if the input is not a valid
/// IPv6 address.
///
/// # Safety
///
/// The caller must ensure that `src` is null or points to a valid null-terminated string.
///
unsafe fn parse_ipv6(src: *const c_char) -> Option<[u8; 16]> {
    if src.is_null() {
        return None;
    }

    let mut tmp: [u8; 16] = [0; 16];
    let mut tp: usize = 0;
    let mut colonp: Option<usize> = None;
    let mut seen_xdigits: u32 = 0;
    let mut val: u32 = 0;
    let mut i: usize = 0;

    // Only a leading "::" may begin with a colon.
    if *src.add(i) as u8 == b':' {
        i += 1;
        if *src.add(i) as u8 != b':' {
            return None;
        }
    }

    let mut curtok: usize = i;
    loop {
        let ch: u8 = *src.add(i) as u8;
        if ch == 0 {
            break;
        }
        i += 1;

        if let Some(hexval) = hex_value(ch) {
            val = (val << 4) | hexval as u32;
            seen_xdigits += 1;
            if seen_xdigits > 4 {
                return None;
            }
            continue;
        }

        if ch == b':' {
            curtok = i;
            if seen_xdigits == 0 {
                // Record the single permitted "::"; a second one is invalid.
                if colonp.is_some() {
                    return None;
                }
                colonp = Some(tp);
                continue;
            }
            // A colon may not terminate the address.
            if *src.add(i) as u8 == 0 {
                return None;
            }
            if tp + 2 > 16 {
                return None;
            }
            tmp[tp] = (val >> 8) as u8;
            tmp[tp + 1] = val as u8;
            tp += 2;
            seen_xdigits = 0;
            val = 0;
            continue;
        }

        // A trailing dotted-decimal IPv4 address fills the final four bytes.
        if ch == b'.' && tp + 4 <= 16 {
            if let Some(octets) = parse_ipv4(src.add(curtok)) {
                tmp[tp] = octets[0];
                tmp[tp + 1] = octets[1];
                tmp[tp + 2] = octets[2];
                tmp[tp + 3] = octets[3];
                tp += 4;
                seen_xdigits = 0;
                break;
            }
            return None;
        }

        return None;
    }

    // Flush a pending group of hex digits.
    if seen_xdigits > 0 {
        if tp + 2 > 16 {
            return None;
        }
        tmp[tp] = (val >> 8) as u8;
        tmp[tp + 1] = val as u8;
        tp += 2;
    }

    // Expand "::" by shifting the groups that follow it to the end of the address.
    if let Some(colon) = colonp {
        if tp == 16 {
            return None;
        }
        let n: usize = tp - colon;
        for j in 1..=n {
            tmp[16 - j] = tmp[colon + n - j];
            tmp[colon + n - j] = 0;
        }
        tp = 16;
    }

    if tp != 16 {
        return None;
    }

    Some(tmp)
}
