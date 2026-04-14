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
// Extern Crates
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
        c_long,
        c_longlong,
        c_short,
        c_uchar,
        c_uint,
        c_ulong,
        c_ulonglong,
        c_ushort,
    },
    sched::sched_param,
    sys_types::{
        blkcnt_t,
        blksize_t,
        c_size_t,
        c_ssize_t,
        clock_t,
        clockid_t,
        dev_t,
        gid_t,
        ino_t,
        mode_t,
        nlink_t,
        off_t,
        pid_t,
        reclen_t,
        time_t,
        uid_t,
    },
    time::timespec,
    unistd::STDOUT_FILENO,
};
use ::syscall::unistd;
use core::mem::size_of;

//==================================================================================================
// Static Assertions: Signed Primitive Types
//==================================================================================================

const _: () = assert!(size_of::<c_char>() == 1);
const _: () = assert!(size_of::<c_short>() == 2);
const _: () = assert!(size_of::<c_int>() == 4);
#[cfg(target_arch = "x86")]
const _: () = assert!(size_of::<c_long>() == 4);
#[cfg(target_arch = "x86_64")]
const _: () = assert!(size_of::<c_long>() == 8);
const _: () = assert!(size_of::<c_longlong>() == 8);
const _: () = assert!(size_of::<f32>() == 4);
const _: () = assert!(size_of::<f64>() == 8);

//==================================================================================================
// Static Assertions: Unsigned Primitive Types
//==================================================================================================

const _: () = assert!(size_of::<c_uchar>() == 1);
const _: () = assert!(size_of::<c_ushort>() == 2);
const _: () = assert!(size_of::<c_uint>() == 4);
#[cfg(target_arch = "x86")]
const _: () = assert!(size_of::<c_ulong>() == 4);
#[cfg(target_arch = "x86_64")]
const _: () = assert!(size_of::<c_ulong>() == 8);
const _: () = assert!(size_of::<c_ulonglong>() == 8);

//==================================================================================================
// Static Assertions: Fixed-Width Integer Types (<stdint.h>)
//==================================================================================================

const _: () = assert!(size_of::<i8>() == 1);
const _: () = assert!(size_of::<i16>() == 2);
const _: () = assert!(size_of::<i32>() == 4);
const _: () = assert!(size_of::<i64>() == 8);
const _: () = assert!(size_of::<u8>() == 1);
const _: () = assert!(size_of::<u16>() == 2);
const _: () = assert!(size_of::<u32>() == 4);
const _: () = assert!(size_of::<u64>() == 8);

//==================================================================================================
// Static Assertions: System Types (<sys/types.h>)
//==================================================================================================

const _: () = assert!(size_of::<blkcnt_t>() == size_of::<c_longlong>());
const _: () = assert!(size_of::<blksize_t>() == size_of::<c_longlong>());
const _: () = assert!(size_of::<clock_t>() == size_of::<c_longlong>());
const _: () = assert!(size_of::<clockid_t>() == size_of::<c_int>());
const _: () = assert!(size_of::<dev_t>() == size_of::<c_ulonglong>());
const _: () = assert!(size_of::<gid_t>() == size_of::<c_uint>());
const _: () = assert!(size_of::<ino_t>() == size_of::<c_ulonglong>());
const _: () = assert!(size_of::<mode_t>() == size_of::<c_uint>());
const _: () = assert!(size_of::<nlink_t>() == size_of::<c_ulonglong>());
const _: () = assert!(size_of::<off_t>() == size_of::<c_longlong>());
const _: () = assert!(size_of::<pid_t>() == size_of::<c_int>());
const _: () = assert!(size_of::<reclen_t>() == size_of::<c_ushort>());
const _: () = assert!(size_of::<c_size_t>() == size_of::<c_uint>());
const _: () = assert!(size_of::<c_ssize_t>() == size_of::<c_int>());
const _: () = assert!(size_of::<time_t>() == size_of::<c_longlong>());
const _: () = assert!(size_of::<uid_t>() == size_of::<c_uint>());

//==================================================================================================
// Static Assertions: Time Types (<time.h>)
//==================================================================================================

const _: () = assert!(size_of::<timespec>() == size_of::<time_t>() + size_of::<c_long>());

//==================================================================================================
// Static Assertions: Scheduling Types (<sched.h>)
//==================================================================================================

const _: () = assert!(size_of::<sched_param>() == size_of::<c_int>());

//==================================================================================================
// Main Function
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Magic string for CI harness.
    {
        let magic_string: &[u8] = b"ok";
        unistd::write(STDOUT_FILENO, magic_string)?;
    }

    Ok(())
}
