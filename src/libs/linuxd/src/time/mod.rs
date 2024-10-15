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

pub struct timespec {
    pub tv_sec: time_t,
    #[cfg(all(target_arch = "x86_64", target_pointer_width = "32"))]
    pub tv_nsec: i64,
    #[cfg(not(all(target_arch = "x86_64", target_pointer_width = "32")))]
    pub tv_nsec: core::ffi::c_long,
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
