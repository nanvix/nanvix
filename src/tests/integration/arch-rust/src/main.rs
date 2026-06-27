// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Modules
//==================================================================================================

mod sse;
mod sse2;

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use crate::{
    sse::test_sse,
    sse2::test_sse2,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::unistd::STDOUT_FILENO;
use ::syscall::unistd;

//==================================================================================================
// Macros
//==================================================================================================

///
/// **Description**
///
/// Runs test and prints whether it passed or failed on the standard output.
///
#[macro_export]
macro_rules! test {
    ($expr:expr $(,)?) => {{
        match $expr {
            true => {
                ::syslog::info!("{} {}", "passed", stringify!($expr));
                true
            },
            false => {
                ::syslog::error!("{} {}", "FAILED", stringify!($expr));
                false
            },
        }
    }};
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let mut all_passed: bool = true;

    all_passed &= test!(test_sse());
    all_passed &= test!(test_sse2());

    if all_passed {
        let magic_string: &[u8] = "ok".as_bytes();
        unistd::write(STDOUT_FILENO, magic_string)?;

        Ok(())
    } else {
        let reason: &str = "some tests failed";
        let magic_string: &[u8] = "failed".as_bytes();
        unistd::write(STDOUT_FILENO, magic_string)?;
        ::syslog::error!("main(): {reason}");
        Err(Error::new(ErrorCode::TryAgain, reason))
    }
}
