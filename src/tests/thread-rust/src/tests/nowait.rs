// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::KernelThread;
use ::core::time::Duration;
use ::sys::{
    error::Error,
    kcall::pm::{
        __kcall_exit,
        __kcall_sleep,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Sleep duration long enough that the worker will still be blocked when the main thread exits.
const WORKER_SLEEP: Duration = Duration::from_secs(5);

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Verifies that a process can exit while a detached worker thread is still blocked.
///
/// This test spawns a worker that sleeps for a long time, detaches it, and then exits the process.
/// The kernel must cleanly tear down the process, terminating the blocked worker. Because this test
/// calls `exit()`, it **must be the last test** executed by the binary.
pub fn run() -> Result<(), Error> {
    test_exit_with_detached_blocked_thread()
}

fn test_exit_with_detached_blocked_thread() -> Result<(), Error> {
    let handle: KernelThread = KernelThread::spawn(worker_sleep, 0)?;
    handle.detach()?;

    // Exit the process. The kernel will interrupt the sleeping worker and clean up.
    // exit() diverges on success; on failure we propagate the error.
    let Err(e) = __kcall_exit(0);
    Err(e)
}

extern "C" fn worker_sleep(_arg: usize) -> usize {
    // Block for a long time. The kernel will interrupt this sleep when the process exits.
    let _ = __kcall_sleep(WORKER_SLEEP);
    0
}
