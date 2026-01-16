// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod common;
mod debug_console_spam;
mod event_registration;
mod kcall_hammer;
mod memory_mapping_storm;
mod mutex_churn;
mod parallel_spawners;
mod scoreboard_backpressure;
mod sleep_burst;
mod thread_data_area;
mod thread_fan_out;
mod thread_identity;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Runs the full suite of thread-related stress workloads.
///
/// # Returns
///
/// `Ok(())` on success or an error if any workload fails.
///
pub fn run_all() -> Result<(), Error> {
    thread_fan_out::run()?;
    mutex_churn::run()?;
    parallel_spawners::run()?;
    thread_identity::run()?;
    kcall_hammer::run()?;
    scoreboard_backpressure::run()?;
    sleep_burst::run()?;
    debug_console_spam::run()?;
    event_registration::run()?;
    memory_mapping_storm::run()?;
    thread_data_area::run()?;
    Ok(())
}
