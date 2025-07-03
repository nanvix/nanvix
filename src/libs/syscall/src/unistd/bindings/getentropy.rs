// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::slice;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Fills a buffer with random data. The `getentropy()` function fills the buffer pointed to by
/// `buffer` with `length` bytes of random data suitable for seeding cryptographically secure
/// random number generators. The random data is obtained from the system's entropy source and
/// should be unpredictable and suitable for cryptographic purposes. This function is designed
/// to be a simple interface for obtaining small amounts of random data without the complexity
/// of opening and reading from `/dev/random` or `/dev/urandom`.
///
/// # Parameters
///
/// - `buffer`: Pointer to the buffer where the random data will be stored. The buffer must be
///   large enough to hold `length` bytes of data.
/// - `length`: Number of bytes of random data to generate and store in the buffer. This value
///   must not exceed the maximum allowed entropy request size defined by the system.
///
/// # Returns
///
/// The `getentropy()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error. Common error conditions include invalid buffer pointer, excessive
/// length request, or system entropy source unavailable.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `buffer` points to a valid memory location of at least `length` bytes.
/// - `buffer` remains valid and writable for the duration of the function call.
/// - `buffer` is properly aligned for byte access.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getentropy(buffer: *mut c_void, length: size_t) -> c_int {
    ::syslog::trace!("getentropy(): buffer={buffer:?}, length={length:?}");

    // Check if buffer is null.
    if buffer.is_null() {
        ::syslog::error!("getentropy(): invalid buffer (buffer={buffer:?}, length={length:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // TODO: https://github.com/nanvix/nanvix/issues/670

    // Fill buffer with 1s.
    let buffer: &mut [u8] = slice::from_raw_parts_mut(buffer as *mut u8, length);
    for byte in buffer.iter_mut() {
        *byte = 1;
    }

    0
}
