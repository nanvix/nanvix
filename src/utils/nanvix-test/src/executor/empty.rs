// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::RunnerConfig,
    log_layout::{
        GuestLogTracker,
        RunnerLogPaths,
        TestLogLayout,
    },
    nanvixd::{
        NanvixdHttp,
        NanvixdHttpArgs,
    },
};
use ::anyhow::Result;
use ::std::path::Path;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Runs the Empty Executor, which spawns the Nanvix Daemon with the supplied configuration and
/// immediately shuts it down, for a specified number of iterations.
///
/// # Parameters
///
/// - `runner_config`: Configuration required to spawn the Nanvix Daemon.
/// - `iterations`: Number of Empty Executor cycles to execute.
/// - `log_layout`: Layout that controls where stdout/stderr artifacts for each iteration land.
///
/// # Return Value
///
/// Returns `Ok(())` after the requested iterations succeed; returns an error if log creation,
/// timestamp computation, or daemon spawning fails.
///
pub(crate) async fn empty(
    runner_config: &RunnerConfig,
    iterations: usize,
    log_layout: &TestLogLayout,
) -> Result<()> {
    let l2_enabled: bool = runner_config.l2_enabled;
    let hwloc_file_path: Option<String> = runner_config.hwloc_file_path.clone();
    let log_root: &Path = Path::new(runner_config.log_directory.as_str());
    let guest_log_tracker: GuestLogTracker = GuestLogTracker::capture(log_root)?;

    for iteration in 0..iterations {
        let RunnerLogPaths {
            stdout: stdout_file_path,
            stderr: stderr_file_path,
        } = log_layout.allocate_runner_logs(Some(iteration));

        let nanvixd_args: NanvixdHttpArgs = NanvixdHttpArgs::new(
            stdout_file_path.as_path(),
            stderr_file_path.as_path(),
            runner_config.ipv4_addr.as_str(),
            runner_config.port_num,
            hwloc_file_path.clone(),
            l2_enabled,
            runner_config.netns_pool_size,
            log_layout.test_directory(),
        )?;

        {
            let _nanvixd_handle: NanvixdHttp =
                NanvixdHttp::spawn(runner_config, &nanvixd_args).await?;
        }

        guest_log_tracker.move_new_logs(log_layout.test_directory())?;
        log_layout.normalize_component_logs(iteration)?;
    }

    guest_log_tracker.move_new_logs(log_layout.test_directory())?;
    if iterations > 0 {
        let last_iteration: usize = iterations - 1;
        log_layout.normalize_component_logs(last_iteration)?;
    }

    Ok(())
}
