// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! This program prints numbers sequentially for a long time. It can be used for pausing and
//! resuming an application and checking that the count continues from where it left off, and also
//! for taking a snapshot of the VM state when paused, then checking that the count continues from
//! where it left off when loading the snapshot.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate nvx;

use sys::{
    error::Error,
    kcall::debug,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    const ITERATIONS_STEP: u64 = 500_000_000;
    let mut buffer = [0u8; 32];

    for i in 0..u64::MAX {
        if i % ITERATIONS_STEP == 0 {
            let string: &str = count_to_string(i / ITERATIONS_STEP, &mut buffer);
            let _ = debug::debug(string.as_ptr(), string.len());
        }
    }

    Ok(())
}

fn count_to_string(mut num: u64, buf: &mut [u8]) -> &str {
    if num == 0 {
        buf[0] = b'0';
        buf[1] = b'\n';
        return unsafe { core::str::from_utf8_unchecked(&buf[0..2]) };
    }

    let mut pos = buf.len() - 1;
    buf[pos] = b'\n';
    while num > 0 {
        pos -= 1;
        buf[pos] = (num % 10) as u8 + b'0';
        num /= 10;
    }

    unsafe { core::str::from_utf8_unchecked(&buf[pos..]) }
}
