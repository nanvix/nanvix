// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::monotonic_now;
use ::core::time::Duration;
use ::sys::{
    error::Error,
    kcall::pm::__kcall_sleep,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Ensures monotonic timekeeping and sleeping primitives behave as expected.
pub fn run() -> Result<(), Error> {
    test_monotonic_clock()?;
    test_sleep_elapsed_time()?;
    Ok(())
}

fn test_monotonic_clock() -> Result<(), Error> {
    let first = monotonic_now()?;
    let second = monotonic_now()?;
    assert!(second >= first, "monotonic clock regressed");
    Ok(())
}

fn test_sleep_elapsed_time() -> Result<(), Error> {
    let before = monotonic_now()?;
    let target = Duration::from_millis(5);
    __kcall_sleep(target)?;
    let after = monotonic_now()?;

    match after.checked_sub(&before) {
        Ok(elapsed) => {
            assert!(
                elapsed >= target,
                "sleep() returned too early (elapsed={:?}, target={:?})",
                elapsed,
                target
            );
            Ok(())
        },
        Err(regressed) => {
            panic!("monotonic clock regressed by {:?}", regressed);
        },
    }
}
