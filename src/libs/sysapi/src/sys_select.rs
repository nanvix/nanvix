// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys_types::{
    suseconds_t,
    time_t,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct timeval {
    /// Seconds.
    pub tv_sec: time_t,
    /// Nano-seconds.
    pub tv_nsec: suseconds_t,
}

impl From<timeval> for crate::time::timespec {
    fn from(tv: timeval) -> Self {
        Self {
            tv_sec: tv.tv_sec,
            tv_nsec: tv.tv_nsec,
        }
    }
}
