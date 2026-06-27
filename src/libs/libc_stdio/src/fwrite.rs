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
/// Writes `nmemb` elements, each `size` bytes long, from the buffer pointed to by `ptr` to the
/// given file stream.
///
/// # Parameters
///
/// - `ptr`: Pointer to the data to be written.
/// - `size`: Size of each element in bytes.
/// - `nmemb`: Number of elements to write.
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Returns
///
/// The number of elements successfully written. If an error occurs or `size`/`nmemb` is zero,
/// zero is returned.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `ptr` points to at least `size * nmemb` readable bytes.
/// - `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fwrite.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fwrite(
    ptr: *const c_void,
    size: c_size_t,
    nmemb: c_size_t,
    stream: *mut FILE,
) -> c_size_t {
    extern "C" {
        fn write(fd: c_int, buf: *const c_void, count: c_size_t) -> c_ssize_t;
    }

    if ptr.is_null() || stream.is_null() || size == 0 || nmemb == 0 {
        return 0;
    }

    let fd: c_int = (*stream).fd;
    // Compute the request size with overflow checking: a wrapping multiply could silently
    // change the number of bytes written.
    let total_bytes: usize = match (size as usize).checked_mul(nmemb as usize) {
        Some(total_bytes) => total_bytes,
        None => {
            (*stream).error = 1;
            return 0;
        },
    };

    // Loop until every byte is written: a single `write` may transfer fewer
    // bytes than requested (a short write) without an error, and `fwrite` must
    // not report a short count in that case.
    let mut written: usize = 0;
    while written < total_bytes {
        // SAFETY: ptr is valid for total_bytes, fd comes from a valid FILE.
        let ret: c_ssize_t = unsafe {
            write(
                fd,
                ptr.cast::<u8>().add(written).cast::<c_void>(),
                (total_bytes - written) as c_size_t,
            )
        };
        if ret <= 0 {
            // A negative result is a write error; a zero result means no progress can be
            // made. Either way, flag the stream's error indicator and stop to keep the
            // stream state consistent and avoid an infinite loop.
            (*stream).error = 1;
            break;
        }
        written += ret as usize;
    }

    let size_usize: usize = size as usize;
    (written / size_usize) as c_size_t
}

///
/// # Description
///
/// Non-locking variant of [`fwrite`]. Nanvix streams are single-threaded, so this is exactly
/// equivalent to [`fwrite`].
///
/// # Parameters
///
/// - `ptr`: Source buffer.
/// - `size`: Size in bytes of each element.
/// - `nmemb`: Number of elements to write.
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Returns
///
/// The number of elements successfully written, which may be less than `nmemb` on error.
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
pub unsafe extern "C" fn fwrite_unlocked(
    ptr: *const c_void,
    size: c_size_t,
    nmemb: c_size_t,
    stream: *mut FILE,
) -> c_size_t {
    fwrite(ptr, size, nmemb, stream)
}
