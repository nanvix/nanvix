// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::StressError;
use ::core::{
    convert::TryFrom,
    time::Duration,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        pm::__kcall_sleep,
        sched::__kcall_sched_yield,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

const SLEEP_BURST_ROUNDS: usize = 16;
const SLEEP_BASE_MICROS: u64 = 128;
const SLEEP_JITTER_MICROS: u64 = 32;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Issues a short burst of sleeps with jittered durations to mimic timer-wheel pressure from many
/// fine-grained delays (e.g., backoff loops or tick schedulers) that can starve runnable threads if
/// scheduling latency grows.
///
/// # Returns
///
/// `Ok(())` on success or an error if sleep or duration conversion fails.
///
pub fn run() -> Result<(), StressError> {
    for round in 0..SLEEP_BURST_ROUNDS {
        let round_u64: u64 = u64::try_from(round)
            .map_err(|_| Error::new(ErrorCode::ValueOutOfRange, "round overflow"))?;
        let delay_micros: u64 = SLEEP_BASE_MICROS + (round_u64 & 0xF) * SLEEP_JITTER_MICROS;
        let duration: Duration = Duration::from_micros(delay_micros);
        __kcall_sleep(duration)?;

        if round & 0x3 == 0 {
            __kcall_sched_yield()?;
        }
    }

    Ok(())
}
