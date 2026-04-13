// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod common;
mod debug_console_spam;
mod event_registration;
mod heap_max_capacity;
mod heap_reclaim;
mod heap_shrink;
mod kcall_hammer;
#[cfg_attr(feature = "hyperlight", allow(dead_code))]
mod memory_mapping_storm;
mod mmap_rapid_cycle;
mod mutex_churn;
mod parallel_spawners;
mod scheduler_pressure;
#[cfg_attr(feature = "hyperlight", allow(dead_code))]
mod scoreboard_backpressure;
mod sleep_burst;
mod thread_data_area;
mod thread_fan_out;
mod thread_identity;
mod zombie_join_pressure;

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
    macro_rules! run_test {
        ($name:expr, $test:expr) => {
            if let Err(e) = $test {
                let _ = ::sys::kcall::debug::debug(
                    concat!("FAIL: ", $name, "\n").as_ptr(),
                    concat!("FAIL: ", $name, "\n").len(),
                );
                return Err(e);
            }
            let _ = ::sys::kcall::debug::debug(
                concat!("PASS: ", $name, "\n").as_ptr(),
                concat!("PASS: ", $name, "\n").len(),
            );
        };
    }
    run_test!("zombie_join_pressure", zombie_join_pressure::run());
    run_test!("thread_fan_out", thread_fan_out::run());
    run_test!("mutex_churn", mutex_churn::run());
    run_test!("parallel_spawners", parallel_spawners::run());
    run_test!("thread_identity", thread_identity::run());
    run_test!("kcall_hammer", kcall_hammer::run());
    run_test!("scheduler_pressure", scheduler_pressure::run());
    #[cfg(not(feature = "hyperlight"))]
    run_test!("scoreboard_backpressure", scoreboard_backpressure::run());
    run_test!("heap_reclaim", heap_reclaim::run());
    run_test!("heap_max_capacity", heap_max_capacity::run());
    run_test!("heap_shrink", heap_shrink::run());
    run_test!("sleep_burst", sleep_burst::run());
    run_test!("debug_console_spam", debug_console_spam::run());
    run_test!("event_registration", event_registration::run());
    #[cfg(not(feature = "hyperlight"))]
    run_test!("memory_mapping_storm", memory_mapping_storm::run());
    run_test!("thread_data_area", thread_data_area::run());
    run_test!("mmap_rapid_cycle", mmap_rapid_cycle::run());
    Ok(())
}
