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

use ::core::ffi::{
    CStr,
    c_char,
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
    static mut environ: *mut *mut c_char;
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads all environment variables from the `environ` global and writes them to stdout, one per
/// line. Each entry is printed as `KEY=VALUE\n`. After all entries are printed, the magic string
/// `"ok"` is written so the test harness can validate success.
///
/// The test harness forwards environment variables by combining them with the command-line
/// arguments using the `<args>;<env>` format in the `program_args` field (both in the HTTP API
/// and the terminal executor). The kernel splits the command line at the `;` separator and
/// populates `environ` accordingly.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let env_ptr: *mut *mut c_char = unsafe { environ };

    if env_ptr.is_null() {
        syslog::error!("main(): environ pointer is null");
        unistd::write(STDOUT_FILENO, b"ok")?;
        return Ok(());
    }

    let mut index: usize = 0;
    loop {
        let entry: *mut c_char = unsafe { *env_ptr.add(index) };
        if entry.is_null() {
            break;
        }

        let c_str: &CStr = unsafe { CStr::from_ptr(entry) };
        let entry_bytes: &[u8] = c_str.to_bytes();

        syslog::info!("main(): env[{}]: {:?}", index, c_str);

        unistd::write(STDOUT_FILENO, entry_bytes)?;
        unistd::write(STDOUT_FILENO, b"\n")?;

        index += 1;
    }

    unistd::write(STDOUT_FILENO, b"ok")?;

    Ok(())
}
