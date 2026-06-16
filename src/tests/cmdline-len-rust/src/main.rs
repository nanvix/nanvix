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

use ::config::system::MAX_CMDLINE_ARGS_LEN;
use ::core::sync::atomic::Ordering;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::unistd::STDOUT_FILENO;
use ::syscall::unistd;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Validates that the guest received command-line arguments whose total wire length equals
/// [`MAX_CMDLINE_ARGS_LEN`]. This proves that the u16 `CmdlineArgsLen` wire format is functional
/// end-to-end at the maximum supported size.
///
/// The test reconstructs the original argument string from `ARGC`/`ARGV` (joining entries with
/// spaces) and checks that its byte length matches exactly.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let argc: i32 = nvx_crt0::ARGC.load(Ordering::SeqCst);
    let argv: *mut *const u8 = nvx_crt0::ARGV.load(Ordering::SeqCst);

    syslog::info!("main(): argc={argc}");

    if argc < 2 {
        let reason: &str = "expected at least two arguments (program name + payload)";
        syslog::error!("main(): {reason}");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Reconstruct the total argument string length.
    // The wire format encodes `"<argv[0]> <argv[1]> ... <argv[n]>"`.
    let mut total_len: usize = 0;
    for i in 0..argc {
        #[allow(clippy::as_conversions)]
        let ptr: *const u8 = unsafe { *argv.add(i as usize) };
        if ptr.is_null() {
            break;
        }

        // Compute length of this argument (null-terminated C string).
        let mut len: usize = 0;
        unsafe {
            while *ptr.add(len) != 0 {
                len += 1;
            }
        }

        if i > 0 {
            // Account for the space separator between arguments.
            total_len += 1;
        }
        total_len += len;
    }

    syslog::info!("main(): total argument string length = {total_len}");

    if total_len != MAX_CMDLINE_ARGS_LEN {
        syslog::error!(
            "main(): unexpected argument string length (len={total_len}, \
             expected={MAX_CMDLINE_ARGS_LEN})"
        );
        return Err(Error::new(ErrorCode::InvalidArgument, "argument string length mismatch"));
    }

    // Success: the u16 wire format delivered arguments at the maximum supported size.
    let magic_string: &[u8] = b"ok";
    unistd::write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
