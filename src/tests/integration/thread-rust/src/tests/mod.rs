// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod alarm_starvation;
mod attributes;
mod cond_timedwait;
mod condvars;
mod identity;
mod mutex_timedlock;
mod mutexes;
mod nowait;
mod once;
mod rwlocks;
mod scheduler;
mod tda;
mod thread_local;
mod threads;
mod timers;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every thread and synchronization test.
pub fn run_all() -> Result<(), Error> {
    threads::run()?;
    identity::run()?;
    attributes::run()?;
    mutexes::run()?;
    mutex_timedlock::run()?;
    once::run()?;
    condvars::run()?;
    cond_timedwait::run()?;
    rwlocks::run()?;
    thread_local::run()?;
    timers::run()?;
    tda::run()?;
    scheduler::run()?;
    alarm_starvation::run()?;
    Ok(())
}

/// Runs the nowait test, which exits the process. Must be called after all other tests and after
/// the success marker has been emitted.
pub fn run_nowait() -> Result<(), Error> {
    nowait::run()
}
