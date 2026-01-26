// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::slice;
use ::spin::{
    Mutex,
    MutexGuard,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::size_t,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Seed value for the pseudo-random number generator.
const SEED: i32 = 12345;
static_assert::assert_eq!(SEED > 0);

/// Next value for the pseudo-random number generator.
static NEXT: Mutex<i32> = Mutex::new((SEED % 0x7ffffffe) + 1);

/// Computes a pseudo-random number using a Park-Miller minimal standard generator.
fn prng() -> i32 {
    let mut state: MutexGuard<'_, i32> = NEXT.lock();

    const A: i32 = 16807;
    const M: i32 = 2147483647;
    const Q: i32 = 127773; // m / a
    const R: i32 = 2836; // m % a

    let hi: i32 = *state / Q;
    let lo: i32 = *state % Q;
    let mut x: i32 = A.wrapping_mul(lo).wrapping_sub(R.wrapping_mul(hi));

    if x <= 0 {
        x = x.wrapping_add(M);
    }
    *state = x;

    x
}

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
#[trace_libcall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getentropy(buffer: *mut c_void, length: size_t) -> c_int {
    // Check if buffer is null.
    if buffer.is_null() {
        ::syslog::error!("getentropy(): invalid buffer (buffer={buffer:?}, length={length:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // TODO: https://github.com/nanvix/nanvix/issues/670

    // Fill the buffer.
    let buffer: &mut [u8] = slice::from_raw_parts_mut(buffer as *mut u8, length);
    for byte in buffer.iter_mut() {
        *byte = prng() as u8;
    }

    0
}
