// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::error::Error;

//==================================================================================================
// Modules
//==================================================================================================

mod attributes;
mod cond_timedwait;
mod condvars;
mod identity;
mod mutex_timedlock;
mod mutexes;
mod rwlocks;
mod scheduler;
mod tda;
mod thread_local;
mod threads;
mod timers;

// TODO: Port nowait.c (tests that a process can exit while worker threads are blocked). The C test
// relies on process-exit semantics that cannot be safely reproduced with the current KernelThread
// abstraction (dropping without joining panics).

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
    condvars::run()?;
    cond_timedwait::run()?;
    rwlocks::run()?;
    thread_local::run()?;
    timers::run()?;
    tda::run()?;
    scheduler::run()?;
    Ok(())
}
