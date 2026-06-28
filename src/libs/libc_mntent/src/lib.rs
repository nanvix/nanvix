// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

// Attributes
#![cfg_attr(not(feature = "std"), no_std)]
// Lints
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::cast_sign_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Modules
//==================================================================================================

mod parse;

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Types
//==================================================================================================

/// Opaque standard I/O stream, defined by `<stdio.h>`. Only ever handled through a pointer.
#[repr(C)]
pub struct CFile {
    _opaque: [u8; 0],
}

/// A filesystem table entry, mirroring the C `struct mntent`.
#[repr(C)]
pub struct Mntent {
    /// Device or remote filesystem.
    pub mnt_fsname: *mut c_char,
    /// Mount point.
    pub mnt_dir: *mut c_char,
    /// Filesystem type.
    pub mnt_type: *mut c_char,
    /// Comma-separated mount options.
    pub mnt_opts: *mut c_char,
    /// Dump frequency in days.
    pub mnt_freq: c_int,
    /// Pass number on parallel fsck.
    pub mnt_passno: c_int,
}

//==================================================================================================
// `<stdio.h>` Backend
//==================================================================================================

// These are provided by `libc_stdio` and resolved when the libc archive is linked.
#[cfg(not(test))]
extern "C" {
    fn fopen(pathname: *const c_char, mode: *const c_char) -> *mut CFile;
    fn fclose(stream: *mut CFile) -> c_int;
    fn fgets(s: *mut c_char, size: c_int, stream: *mut CFile) -> *mut c_char;
    fn fputc(c: c_int, stream: *mut CFile) -> c_int;
}

//==================================================================================================
// Constants
//==================================================================================================

/// Capacity of the static line buffer used by the non-reentrant [`getmntent`].
#[cfg(not(test))]
const LINE_CAPACITY: usize = 4096;
/// Capacity of the static line buffer expressed as a C `int`.
#[cfg(not(test))]
const LINE_CAPACITY_INT: c_int = 4096;

/// ASCII space, as a C `int` for [`fputc`].
#[cfg(not(test))]
const SPACE: c_int = b' ' as c_int;
/// ASCII newline, as a C `int` for [`fputc`].
#[cfg(not(test))]
const NEWLINE: c_int = b'\n' as c_int;
/// ASCII minus sign, as a C `int` for [`fputc`].
#[cfg(not(test))]
const MINUS: c_int = b'-' as c_int;
/// ASCII digit zero, as a C `int` for [`fputc`].
#[cfg(not(test))]
const DIGIT_ZERO: c_int = b'0' as c_int;
/// ASCII backslash, as a C `int` for [`fputc`].
#[cfg(not(test))]
const BACKSLASH: c_int = b'\\' as c_int;

//==================================================================================================
// Non-Reentrant State
//==================================================================================================

/// Line buffer backing the non-reentrant [`getmntent`].
#[cfg(not(test))]
static mut LINE_BUFFER: [c_char; LINE_CAPACITY] = [0; LINE_CAPACITY];

/// Entry storage backing the non-reentrant [`getmntent`].
#[cfg(not(test))]
static mut STATIC_ENTRY: Mntent = Mntent {
    mnt_fsname: core::ptr::null_mut(),
    mnt_dir: core::ptr::null_mut(),
    mnt_type: core::ptr::null_mut(),
    mnt_opts: core::ptr::null_mut(),
    mnt_freq: 0,
    mnt_passno: 0,
};

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Opens the filesystem description file `filename` for reading or writing, returning a stream
/// suitable for use with [`getmntent`], [`addmntent`], and [`endmntent`].
///
/// # Parameters
///
/// - `filename`: Path to the filesystem description file (e.g. `/etc/fstab`).
/// - `type`: Access mode, as understood by `fopen()` (e.g. `"r"` or `"a"`).
///
/// # Return Value
///
/// A stream pointer on success, or a null pointer on error.
///
/// # Safety
///
/// `filename` and `type` must be valid null-terminated C strings.
///
#[cfg(not(test))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn setmntent(filename: *const c_char, r#type: *const c_char) -> *mut CFile {
    fopen(filename, r#type)
}

///
/// # Description
///
/// Closes the filesystem description file stream opened by [`setmntent`].
///
/// # Parameters
///
/// - `stream`: Stream returned by [`setmntent`].
///
/// # Return Value
///
/// Always returns `1`, as mandated by the interface.
///
/// # Safety
///
/// `stream` must be null or a stream previously returned by [`setmntent`].
///
#[cfg(not(test))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn endmntent(stream: *mut CFile) -> c_int {
    if !stream.is_null() {
        fclose(stream);
    }
    1
}

///
/// # Description
///
/// Reads the next entry from a filesystem description file stream, storing the result in a
/// statically allocated structure that is overwritten on each call.
///
/// # Parameters
///
/// - `stream`: Stream returned by [`setmntent`].
///
/// # Return Value
///
/// A pointer to a [`Mntent`] on success, or a null pointer at end of file.
///
/// # Safety
///
/// `stream` must be a stream previously returned by [`setmntent`]. The returned pointer aliases
/// shared static storage and must not be used across threads or concurrent calls.
///
#[cfg(not(test))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getmntent(stream: *mut CFile) -> *mut Mntent {
    let entry: *mut Mntent = core::ptr::addr_of_mut!(STATIC_ENTRY);
    let buffer: *mut c_char = core::ptr::addr_of_mut!(LINE_BUFFER).cast::<c_char>();
    getmntent_r(stream, entry, buffer, LINE_CAPACITY_INT)
}

///
/// # Description
///
/// Reentrant variant of [`getmntent`]: reads the next entry into the caller-provided structure,
/// using `buffer` to hold the parsed field strings.
///
/// # Parameters
///
/// - `stream`: Stream returned by [`setmntent`].
/// - `result`: Destination structure.
/// - `buffer`: Scratch buffer that backs the field strings in `result`.
/// - `bufsize`: Size of `buffer`, in bytes.
///
/// # Return Value
///
/// `result` on success, or a null pointer at end of file or on invalid arguments.
///
/// # Safety
///
/// All pointers must be valid, and `buffer` must be writable for `bufsize` bytes.
///
#[cfg(not(test))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getmntent_r(
    stream: *mut CFile,
    result: *mut Mntent,
    buffer: *mut c_char,
    bufsize: c_int,
) -> *mut Mntent {
    if stream.is_null() || result.is_null() || buffer.is_null() || bufsize <= 1 {
        return core::ptr::null_mut();
    }

    loop {
        if fgets(buffer, bufsize, stream).is_null() {
            return core::ptr::null_mut();
        }
        if parse::parse_line(buffer, result) {
            return result;
        }
    }
}

///
/// # Description
///
/// Appends a filesystem table entry to a stream opened for writing by [`setmntent`].
///
/// # Parameters
///
/// - `stream`: Stream opened for appending.
/// - `mnt`: Entry to write.
///
/// # Return Value
///
/// `0` on success, or `1` on error. It is an error for any of the four mandatory string fields
/// (`mnt_fsname`, `mnt_dir`, `mnt_type`, `mnt_opts`) to be null or empty, as such an entry could not
/// be read back by [`getmntent`].
///
/// # Safety
///
/// `stream` must be writable, and `mnt` must point to a valid [`Mntent`] whose string fields are
/// valid null-terminated C strings (or null).
///
#[cfg(not(test))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn addmntent(stream: *mut CFile, mnt: *const Mntent) -> c_int {
    if stream.is_null() || mnt.is_null() {
        return 1;
    }
    let entry: &Mntent = &*mnt;
    if field_is_empty(entry.mnt_fsname)
        || field_is_empty(entry.mnt_dir)
        || field_is_empty(entry.mnt_type)
        || field_is_empty(entry.mnt_opts)
    {
        return 1;
    }
    if write_field(stream, entry.mnt_fsname) != 0
        || fputc(SPACE, stream) < 0
        || write_field(stream, entry.mnt_dir) != 0
        || fputc(SPACE, stream) < 0
        || write_field(stream, entry.mnt_type) != 0
        || fputc(SPACE, stream) < 0
        || write_field(stream, entry.mnt_opts) != 0
        || fputc(SPACE, stream) < 0
        || write_int(stream, entry.mnt_freq) != 0
        || fputc(SPACE, stream) < 0
        || write_int(stream, entry.mnt_passno) != 0
        || fputc(NEWLINE, stream) < 0
    {
        return 1;
    }
    0
}

///
/// # Description
///
/// Searches the option list of `mnt` for the option named `opt`.
///
/// # Parameters
///
/// - `mnt`: Entry whose `mnt_opts` field is searched.
/// - `opt`: Null-terminated option name to look for.
///
/// # Return Value
///
/// A pointer to the matching option within `mnt_opts`, or a null pointer when absent.
///
/// # Safety
///
/// `mnt` must be null or valid, and `opt` must be a valid null-terminated C string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn hasmntopt(mnt: *const Mntent, opt: *const c_char) -> *mut c_char {
    if mnt.is_null() {
        return core::ptr::null_mut();
    }
    parse::option_search((*mnt).mnt_opts, opt)
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Returns `true` when `field` is null or points to an empty string, i.e. when it is not a valid
/// mandatory field value for [`addmntent`].
#[cfg(not(test))]
unsafe fn field_is_empty(field: *const c_char) -> bool {
    field.is_null() || *field == 0
}

/// Writes a single (possibly null) field to `stream`, octal-escaping the characters that are
/// significant to the line format (space, tab, newline, and backslash) as glibc does, so that the
/// entry can be read back by [`getmntent`]. Returns `0` on success or `1` on error.
#[cfg(not(test))]
unsafe fn write_field(stream: *mut CFile, field: *const c_char) -> c_int {
    if field.is_null() {
        return 0;
    }
    let mut i: usize = 0;
    loop {
        let c: c_char = *field.add(i);
        if c == 0 {
            return 0;
        }
        let byte: u8 = u8::from_ne_bytes(c.to_ne_bytes());
        if matches!(byte, b' ' | b'\t' | b'\n' | b'\\') {
            if write_octal_escape(stream, byte) != 0 {
                return 1;
            }
        } else if fputc(c_int::from(byte), stream) < 0 {
            return 1;
        }
        i += 1;
    }
}

/// Writes `byte` as a three-digit octal escape sequence (`\NNN`) to `stream`, returning `0` on
/// success or `1` on error.
#[cfg(not(test))]
unsafe fn write_octal_escape(stream: *mut CFile, byte: u8) -> c_int {
    let d0: c_int = c_int::from((byte >> 6) & 0o7);
    let d1: c_int = c_int::from((byte >> 3) & 0o7);
    let d2: c_int = c_int::from(byte & 0o7);
    if fputc(BACKSLASH, stream) < 0
        || fputc(DIGIT_ZERO + d0, stream) < 0
        || fputc(DIGIT_ZERO + d1, stream) < 0
        || fputc(DIGIT_ZERO + d2, stream) < 0
    {
        return 1;
    }
    0
}

/// Writes the decimal representation of `value` to `stream`, returning `0` on success or `1` on
/// error.
#[cfg(not(test))]
unsafe fn write_int(stream: *mut CFile, value: c_int) -> c_int {
    // i32 has at most 10 decimal digits; the sign is emitted separately.
    let mut digits: [c_int; 10] = [0; 10];
    // Widen to i64 so that negating i32::MIN cannot overflow.
    let mut remaining: i64 = i64::from(value);
    if remaining < 0 {
        if fputc(MINUS, stream) < 0 {
            return 1;
        }
        remaining = -remaining;
    }

    let mut count: usize = 0;
    if remaining == 0 {
        digits[0] = 0;
        count = 1;
    } else {
        while remaining > 0 && count < digits.len() {
            // `remaining % 10` is in `0..=9`, which always fits in a C `int`.
            digits[count] = c_int::try_from(remaining % 10).unwrap_or(0);
            count += 1;
            remaining /= 10;
        }
    }

    // Digits were collected least-significant first; emit them in reverse.
    let mut index: usize = count;
    while index > 0 {
        index -= 1;
        if fputc(DIGIT_ZERO + digits[index], stream) < 0 {
            return 1;
        }
    }
    0
}
