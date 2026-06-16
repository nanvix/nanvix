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

use ::core::{
    ffi::CStr,
    sync::atomic::Ordering,
};
use ::sys::error::Error;
use ::sysapi::unistd::STDOUT_FILENO;
use ::syscall::unistd;

//==================================================================================================
// External Symbols
//==================================================================================================

// The `environ` pointer is set by the nvx runtime (_start) and contains a null-terminated array of
// pointers to "KEY=VALUE\0" C strings.
unsafe extern "C" {
    static mut environ: *mut *mut i8;
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Prints command-line arguments and environment variables to stdout in a deterministic format
/// that the test harness can match:
///
/// ```text
/// ARGS:<argv[1]> <argv[2]> ...
/// ENV:<KEY1=VAL1> <KEY2=VAL2> ...
/// ok
/// ```
///
/// - `ARGS:` is followed by space-separated arguments (argv[0] is skipped as it is the program
///   name). If there are no arguments beyond argv[0], the line is `ARGS:` with nothing after.
/// - `ENV:` is followed by space-separated `KEY=VALUE` entries. If there are no environment
///   variables, the line is `ENV:` with nothing after.
/// - `ok` is always written last so the test harness can confirm the program completed.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // -- Print arguments --
    unistd::write(STDOUT_FILENO, b"ARGS:")?;

    let argc: i32 = nvx_crt0::ARGC.load(Ordering::SeqCst);
    let argv: *mut *const u8 = nvx_crt0::ARGV.load(Ordering::SeqCst);

    if !argv.is_null() && argc > 1 {
        for i in 1..argc {
            if i > 1 {
                unistd::write(STDOUT_FILENO, b" ")?;
            }
            #[allow(clippy::as_conversions)]
            let ptr: *const u8 = unsafe { *argv.add(i as usize) };
            if ptr.is_null() {
                break;
            }
            let mut len: usize = 0;
            unsafe {
                while *ptr.add(len) != 0 {
                    len += 1;
                }
            }
            let bytes: &[u8] = unsafe { ::core::slice::from_raw_parts(ptr, len) };
            unistd::write(STDOUT_FILENO, bytes)?;
        }
    }
    unistd::write(STDOUT_FILENO, b"\n")?;

    // -- Print environment variables --
    unistd::write(STDOUT_FILENO, b"ENV:")?;

    let env_ptr: *mut *mut i8 = unsafe { environ };
    if !env_ptr.is_null() {
        let mut index: usize = 0;
        loop {
            let entry: *mut i8 = unsafe { *env_ptr.add(index) };
            if entry.is_null() {
                break;
            }
            if index > 0 {
                unistd::write(STDOUT_FILENO, b" ")?;
            }
            let c_str: &CStr = unsafe { CStr::from_ptr(entry) };
            unistd::write(STDOUT_FILENO, c_str.to_bytes())?;
            index += 1;
        }
    }
    unistd::write(STDOUT_FILENO, b"\n")?;

    unistd::write(STDOUT_FILENO, b"ok")?;

    Ok(())
}
