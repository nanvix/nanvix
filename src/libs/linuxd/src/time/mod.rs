// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod message;

pub const CLOCK_REALTIME: clockid_t = 0;
pub const CLOCK_MONOTONIC: clockid_t = 1;

pub type time_t = i64;

#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct timespec {
    pub tv_sec: time_t,
    pub tv_nsec: i64,
}

pub type clockid_t = i32;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::{
            clock_getres,
            clock_gettime,
        };
    }
}
