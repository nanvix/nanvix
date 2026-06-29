// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::as_conversions)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::core::ffi::CStr;
use ::sys::error::Error;
use ::sysapi::{
    ffi::c_char,
    unistd::STDOUT_FILENO,
};
use ::syscall::unistd;

//==================================================================================================
// Constants
//==================================================================================================

/// Environment variable names probed via `getenv()`, in deterministic output order.
///
/// Each entry is NUL-terminated so it can be passed straight to `getenv()`. `NVX_GETENV_MISSING`
/// is never expected to resolve: the harness either omits it entirely (null-return path) or
/// supplies it as a bare `KEY` token with no `=` (malformed-entry-skip path). Either way
/// `getenv()` must return null for it.
const KEYS: [&[u8]; 4] = [
    b"HOME\0",
    b"NVX_GETENV_A\0",
    b"NVX_GETENV_B\0",
    b"NVX_GETENV_MISSING\0",
];

/// Sentinel written in place of a value when `getenv()` returns a null pointer.
const NULL_SENTINEL: &[u8] = b"(null)";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Probes a fixed set of environment variables through `getenv()` and writes the results to stdout
/// in a deterministic format the test harness can match:
///
/// ```text
/// GETENV:HOME=<v> NVX_GETENV_A=<v> NVX_GETENV_B=<v> NVX_GETENV_MISSING=<v>
/// ok
/// ```
///
/// Each `<v>` is the value returned by `getenv()` for that key, or `(null)` when `getenv()` returns
/// a null pointer. Because `getenv()` reads the process environment table populated at start-of-day
/// from the kernel-provided env page, a non-null result proves the kernel-delivered variable made
/// it all the way into the runtime environment API. The trailing `ok` lets the harness confirm the
/// program ran to completion.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    unistd::write(STDOUT_FILENO, b"GETENV:")?;

    for (index, key) in KEYS.iter().enumerate() {
        if index > 0 {
            unistd::write(STDOUT_FILENO, b" ")?;
        }

        // Print the key name without its trailing NUL terminator.
        let name: &[u8] = &key[..key.len() - 1];
        unistd::write(STDOUT_FILENO, name)?;
        unistd::write(STDOUT_FILENO, b"=")?;

        // Look up the value through getenv(), which consults the environment table initialized at
        // process start from the kernel-provided env page.
        let value: *mut c_char = unsafe { ::libc_stdlib::getenv(key.as_ptr().cast::<c_char>()) };
        if value.is_null() {
            unistd::write(STDOUT_FILENO, NULL_SENTINEL)?;
        } else {
            // SAFETY: a non-null `getenv()` result points to a NUL-terminated C string that remains
            // valid until the next environment mutation, and this test performs none.
            let bytes: &[u8] = unsafe { CStr::from_ptr(value) }.to_bytes();
            unistd::write(STDOUT_FILENO, bytes)?;
        }
    }

    unistd::write(STDOUT_FILENO, b"\n")?;
    unistd::write(STDOUT_FILENO, b"ok")?;

    Ok(())
}
