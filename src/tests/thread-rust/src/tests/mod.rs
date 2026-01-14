// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod condvars;
mod mutexes;
mod scheduler;
mod tda;
mod threads;
mod timers;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every thread and synchronization test.
pub fn run_all() -> Result<(), Error> {
    threads::run()?;
    mutexes::run()?;
    condvars::run()?;
    timers::run()?;
    tda::run()?;
    scheduler::run()?;
    Ok(())
}
