// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::posix::{
    sys::times,
    time::{
        self,
        timespec,
        CLOCK_MONOTONIC,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn test_times() {
    match times::times(None) {
        Ok(clock) => {
            ::nvx::info!("times() returned {}", clock);
        },
        Err(e) => {
            panic!("times() failed: {:?}", e);
        },
    }
}

pub fn test_clock_getres() {
    let mut res: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    match time::clock_getres(CLOCK_MONOTONIC, Some(&mut res)) {
        Ok(()) => {
            ::nvx::info!("clock resolution: {}s {}ns", { res.tv_sec }, { res.tv_nsec });
        },
        Err(error) => {
            panic!("failed to get clock resolution: {:?}", error);
        },
    }
}

pub fn test_clock_gettime() {
    let mut tp: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    match time::clock_gettime(CLOCK_MONOTONIC, Some(&mut tp)) {
        Ok(()) => {
            ::nvx::info!("clock time: {}s {}ns", { tp.tv_sec }, { tp.tv_nsec });
        },
        e => {
            panic!("failed to get clock time: {:?}", e);
        },
    }
}

pub fn test() {
    test_times();
    test_clock_getres();
    test_clock_gettime();
}
