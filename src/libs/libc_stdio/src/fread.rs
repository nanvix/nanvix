// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads `nmemb` elements, each `size` bytes long, from the stream pointed to by `stream` into
/// the buffer pointed to by `ptr`.
///
/// # Parameters
///
/// - `ptr`: Pointer to the buffer where data will be stored.
/// - `size`: Size of each element in bytes.
/// - `nmemb`: Number of elements to read.
/// - `stream`: Pointer to the source [`FILE`] stream.
///
/// # Returns
///
/// The number of elements successfully read. If an error occurs or end-of-file is reached, the
/// return value may be less than `nmemb`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `ptr` points to at least `size * nmemb` writable bytes.
/// - `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fread.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fread(
    ptr: *mut c_void,
    size: c_size_t,
    nmemb: c_size_t,
    stream: *mut FILE,
) -> c_size_t {
    extern "C" {
        fn read(fd: c_int, buf: *mut c_void, count: c_size_t) -> c_ssize_t;
    }

    if ptr.is_null() || stream.is_null() || size == 0 || nmemb == 0 {
        return 0;
    }

    // Compute the request size with overflow checking: a wrapping multiply could yield a
    // small (or zero) byte count and cause a short or out-of-bounds transfer.
    let total: usize = match (size as usize).checked_mul(nmemb as usize) {
        Some(total) => total,
        None => {
            (*stream).error = 1;
            return 0;
        },
    };
    let buf: *mut u8 = ptr.cast::<u8>();
    let mut offset: usize = 0;

    // Drain the push-back buffer first.
    if (*stream).ungetc_buf != -1 {
        *buf = (*stream).ungetc_buf as u8;
        (*stream).ungetc_buf = -1;
        offset = 1;
    }

    if offset < total {
        let fd: c_int = (*stream).fd;
        // Loop until the request is satisfied: a single `read` may return fewer
        // bytes than requested without being at end-of-file, and `fread` must
        // only report a short count on a genuine EOF or error.
        while offset < total {
            let remaining: c_size_t = (total - offset) as c_size_t;
            // SAFETY: buf + offset is valid for remaining bytes, fd comes from a valid FILE.
            let ret: c_ssize_t = unsafe { read(fd, buf.add(offset).cast::<c_void>(), remaining) };
            if ret < 0 {
                (*stream).error = 1;
                break;
            } else if ret == 0 {
                (*stream).eof = 1;
                break;
            }
            offset += ret as usize;
        }
    }

    let size_usize: usize = size as usize;
    (offset / size_usize) as c_size_t
}

///
/// # Description
///
/// Non-locking variant of [`fread`]. Nanvix streams are single-threaded, so this is exactly
/// equivalent to [`fread`].
///
/// # Parameters
///
/// - `ptr`: Destination buffer.
/// - `size`: Size in bytes of each element.
/// - `nmemb`: Number of elements to read.
/// - `stream`: Pointer to the source [`FILE`] stream.
///
/// # Returns
///
/// The number of elements successfully read, which may be less than `nmemb` on end-of-file or
/// error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `ptr` points to a buffer of at least `size * nmemb` bytes and that `stream` is a valid [`FILE`].
///
/// # References
///
/// - <https://man7.org/linux/man-pages/man3/unlocked_stdio.3.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fread_unlocked(
    ptr: *mut c_void,
    size: c_size_t,
    nmemb: c_size_t,
    stream: *mut FILE,
) -> c_size_t {
    fread(ptr, size, nmemb, stream)
}
