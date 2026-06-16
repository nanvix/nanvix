// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    sys_times::tms,
    time::{
        clock_ids::{
            CLOCK_MONOTONIC,
            CLOCK_PROCESS_CPUTIME_ID,
            CLOCK_REALTIME,
            CLOCK_THREAD_CPUTIME_ID,
        },
        timespec,
    },
};
use ::syscall::{
    sys::times as sys_times,
    time,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs all time-related tests.
pub fn run() -> Result<(), Error> {
    test_clock_getres()?;
    test_clock_gettime()?;
    test_nanosleep()?;
    test_times()?;
    Ok(())
}

/// Tests whether we can get the resolution of a clock with `clock_getres()`.
fn test_clock_getres() -> Result<(), Error> {
    // Get the resolution of the monotonic clock and check the result.
    {
        let mut res = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        time::clock_getres(CLOCK_MONOTONIC, &mut Some(&mut res))?;
        assert!(res.tv_sec >= 0, "clock_getres(CLOCK_MONOTONIC): tv_sec must be non-negative");
        assert!(res.tv_nsec >= 0, "clock_getres(CLOCK_MONOTONIC): tv_nsec must be non-negative");
    }

    // Get the resolution of the real-time clock and check the result.
    {
        let mut res = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        time::clock_getres(CLOCK_REALTIME, &mut Some(&mut res))?;
        assert!(res.tv_sec >= 0, "clock_getres(CLOCK_REALTIME): tv_sec must be non-negative");
        assert!(res.tv_nsec >= 0, "clock_getres(CLOCK_REALTIME): tv_nsec must be non-negative");
    }

    // Get the resolution of the process CPU-time clock and check the result.
    {
        let mut res = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let result = time::clock_getres(CLOCK_PROCESS_CPUTIME_ID, &mut Some(&mut res));
        match result {
            Err(e) => assert_eq!(
                e.code,
                ErrorCode::OperationNotSupported,
                "clock_getres(CLOCK_PROCESS_CPUTIME_ID): expected OperationNotSupported"
            ),
            Ok(()) => panic!("clock_getres(CLOCK_PROCESS_CPUTIME_ID) should have failed"),
        }
    }

    // Get the resolution of the thread CPU-time clock and check the result.
    {
        let mut res = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let result = time::clock_getres(CLOCK_THREAD_CPUTIME_ID, &mut Some(&mut res));
        match result {
            Err(e) => assert_eq!(
                e.code,
                ErrorCode::OperationNotSupported,
                "clock_getres(CLOCK_THREAD_CPUTIME_ID): expected OperationNotSupported"
            ),
            Ok(()) => panic!("clock_getres(CLOCK_THREAD_CPUTIME_ID) should have failed"),
        }
    }

    Ok(())
}

/// Tests whether we can get the current time of a clock with `clock_gettime()`.
fn test_clock_gettime() -> Result<(), Error> {
    // Get the current time of the monotonic clock and check the result.
    {
        let mut ts = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        time::clock_gettime(CLOCK_MONOTONIC, &mut Some(&mut ts))?;
        assert!(ts.tv_sec >= 0, "clock_gettime(CLOCK_MONOTONIC): tv_sec must be non-negative");
        assert!(ts.tv_nsec >= 0, "clock_gettime(CLOCK_MONOTONIC): tv_nsec must be non-negative");
    }

    // Get the current time of the real-time clock and check the result.
    {
        let mut ts = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        time::clock_gettime(CLOCK_REALTIME, &mut Some(&mut ts))?;
        assert!(ts.tv_sec >= 0, "clock_gettime(CLOCK_REALTIME): tv_sec must be non-negative");
        assert!(ts.tv_nsec >= 0, "clock_gettime(CLOCK_REALTIME): tv_nsec must be non-negative");
    }

    // Get the current time of the process CPU-time clock and check the result.
    {
        let mut ts = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let result = time::clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &mut Some(&mut ts));
        match result {
            Err(e) => assert_eq!(
                e.code,
                ErrorCode::OperationNotSupported,
                "clock_gettime(CLOCK_PROCESS_CPUTIME_ID): expected OperationNotSupported"
            ),
            Ok(()) => panic!("clock_gettime(CLOCK_PROCESS_CPUTIME_ID) should have failed"),
        }
    }

    // Get the current time of the thread CPU-time clock and check the result.
    {
        let mut ts = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let result = time::clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut Some(&mut ts));
        match result {
            Err(e) => assert_eq!(
                e.code,
                ErrorCode::OperationNotSupported,
                "clock_gettime(CLOCK_THREAD_CPUTIME_ID): expected OperationNotSupported"
            ),
            Ok(()) => panic!("clock_gettime(CLOCK_THREAD_CPUTIME_ID) should have failed"),
        }
    }

    Ok(())
}

/// Tests whether we can sleep for a given amount of time with `nanosleep()`.
fn test_nanosleep() -> Result<(), Error> {
    let req = timespec {
        tv_sec: 1,
        tv_nsec: 0,
    };
    time::nanosleep(&req, &mut None)?;
    Ok(())
}

/// Tests whether we can retrieve process times with `times()`.
fn test_times() -> Result<(), Error> {
    // Call `times()` with a buffer and check the result.
    {
        let mut buf = tms {
            tms_utime: 0,
            tms_stime: 0,
            tms_cutime: 0,
            tms_cstime: 0,
        };
        let elapsed = sys_times::times(&mut Some(&mut buf))?;
        let _ = elapsed;
    }

    // Call `times()` without a buffer and check the result.
    {
        let _elapsed = sys_times::times(&mut None)?;
    }

    Ok(())
}
