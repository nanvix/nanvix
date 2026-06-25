// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fgetc::fgetc,
    streams::FILE,
};
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
        c_void,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

extern "C" {
    fn malloc(size: c_size_t) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: c_size_t) -> *mut c_void;
}

/// Minimum capacity used when a fresh buffer has to be allocated.
const INITIAL_CAPACITY: usize = 120;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads a line from `stream`, up to and including the next `delim` byte (or end-of-file),
/// storing it in a dynamically sized buffer. The buffer pointed to by `*lineptr` is grown with
/// [`realloc`] as needed and `*n` is updated to reflect its capacity; passing a null `*lineptr`
/// (with `*n` ignored) requests a freshly allocated buffer. The resulting line is always
/// null-terminated.
///
/// # Parameters
///
/// - `lineptr`: Address of the caller's buffer pointer; updated when the buffer is (re)allocated.
/// - `n`: Address of the caller's capacity counter; updated when the buffer is (re)allocated.
/// - `delim`: Delimiter byte that terminates the line.
/// - `stream`: Pointer to the source [`FILE`] stream.
///
/// # Returns
///
/// The number of bytes read (including the delimiter but excluding the terminating null byte), or
/// `-1` on end-of-file with no bytes read or on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `lineptr`, `n`, and `stream` point to valid objects, and that any non-null `*lineptr` was
/// obtained from the C allocator with a capacity of `*n` bytes.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getdelim.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getdelim(
    lineptr: *mut *mut c_char,
    n: *mut c_size_t,
    delim: c_int,
    stream: *mut FILE,
) -> c_ssize_t {
    if lineptr.is_null() || n.is_null() || stream.is_null() {
        return -1;
    }

    let mut buf: *mut c_char = *lineptr;
    let mut cap: usize = *n as usize;

    // Allocate an initial buffer if the caller did not provide one.
    if buf.is_null() || cap == 0 {
        cap = INITIAL_CAPACITY;
        buf = malloc(cap as c_size_t).cast::<c_char>();
        if buf.is_null() {
            return -1;
        }
        *lineptr = buf;
        *n = cap as c_size_t;
    }

    let delim_byte: c_int = delim & 0xff;
    let mut pos: usize = 0;

    loop {
        let c: c_int = fgetc(stream);
        if c == -1 {
            // End-of-file or read error: report failure only when nothing was read.
            if pos == 0 {
                return -1;
            }
            break;
        }

        // Ensure room for the byte just read plus the terminating null byte.
        if pos + 1 >= cap {
            let new_cap: usize = match cap.checked_mul(2) {
                Some(value) => value,
                None => return -1,
            };
            let new_buf: *mut c_char =
                realloc(buf.cast::<c_void>(), new_cap as c_size_t).cast::<c_char>();
            if new_buf.is_null() {
                return -1;
            }
            buf = new_buf;
            cap = new_cap;
            *lineptr = buf;
            *n = cap as c_size_t;
        }

        *buf.add(pos).cast::<u8>() = (c & 0xff) as u8;
        pos += 1;

        if c == delim_byte {
            break;
        }
    }

    *buf.add(pos) = 0;
    pos as c_ssize_t
}

///
/// # Description
///
/// Reads a line from `stream`, up to and including the next newline byte (or end-of-file). This is
/// equivalent to [`getdelim`] with a delimiter of `'\n'`.
///
/// # Parameters
///
/// - `lineptr`: Address of the caller's buffer pointer; updated when the buffer is (re)allocated.
/// - `n`: Address of the caller's capacity counter; updated when the buffer is (re)allocated.
/// - `stream`: Pointer to the source [`FILE`] stream.
///
/// # Returns
///
/// The number of bytes read (including the newline but excluding the terminating null byte), or
/// `-1` on end-of-file with no bytes read or on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `lineptr`, `n`, and `stream` point to valid objects, and that any non-null `*lineptr` was
/// obtained from the C allocator with a capacity of `*n` bytes.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getline.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getline(
    lineptr: *mut *mut c_char,
    n: *mut c_size_t,
    stream: *mut FILE,
) -> c_ssize_t {
    getdelim(lineptr, n, c_int::from(b'\n'), stream)
}
