// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(unix)]
use crate::nanvixd::{
    NanvixdHttp,
    NanvixdHttpArgs,
};
use crate::{
    config::RunnerConfig,
    log_layout::{
        GuestLogTracker,
        RunnerLogPaths,
        TestLogLayout,
    },
};
use ::anyhow::Result;
use ::log::error;
use ::std::path::Path;
use ::tokio_util::sync::CancellationToken;

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
/// - `extra_nanvixd_args`: Command-line arguments passed directly to nanvixd.
///
/// # Return Value
///
/// Returns `Ok(())` after the requested iterations succeed; returns an error if log creation,
/// timestamp computation, or daemon spawning fails.
///
#[cfg(unix)]
pub(crate) async fn empty(
    runner_config: &RunnerConfig,
    iterations: usize,
    log_layout: &TestLogLayout,
    extra_nanvixd_args: &[String],
    cancellation_token: CancellationToken,
) -> Result<()> {
    tokio::select! {
        result = async {
            let hwloc_file_path: Option<String> = runner_config.hwloc_file_path.clone();
            let log_root: &Path = Path::new(runner_config.log_directory.as_str());
            let guest_log_tracker: GuestLogTracker = GuestLogTracker::capture(log_root)?;

            for iteration in 0..iterations {
                let RunnerLogPaths {
                    stdout: stdout_file_path,
                    stderr: stderr_file_path,
                } = log_layout.allocate_runner_logs(Some(iteration));

                let nanvixd_http_args: NanvixdHttpArgs = NanvixdHttpArgs::new(
                    (stdout_file_path.as_path(), stderr_file_path.as_path()),
                    (runner_config.ipv4_addr.as_str(), runner_config.port_num),
                    hwloc_file_path.clone(),
                    log_layout.test_directory(),
                    extra_nanvixd_args,
                )?;

                {
                    let _nanvixd_handle: NanvixdHttp =
                        NanvixdHttp::spawn(runner_config, &nanvixd_http_args).await?;
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
    } => result,
        _ = cancellation_token.cancelled() => {
            error!("empty(): cancellation requested");
            Err(::anyhow::anyhow!("cancelled"))
        },
    }
}

///
/// # Description
///
/// Runs the Empty Executor on non-Unix platforms. Since the HTTP-based daemon mode is not
/// available, this variant preserves log-management bookkeeping without spawning the daemon.
///
/// # Parameters
///
/// - `runner_config`: Configuration required by the runner (used for log paths).
/// - `iterations`: Number of Empty Executor cycles to execute.
/// - `log_layout`: Layout that controls where stdout/stderr artifacts for each iteration land.
/// - `_extra_nanvixd_args`: Unused on non-Unix platforms.
///
/// # Return Value
///
/// Returns `Ok(())` after log bookkeeping completes for all iterations.
///
#[cfg(not(unix))]
pub(crate) async fn empty(
    runner_config: &RunnerConfig,
    iterations: usize,
    log_layout: &TestLogLayout,
    _extra_nanvixd_args: &[String],
    cancellation_token: CancellationToken,
) -> Result<()> {
    tokio::select! {
        result = async {
            let log_root: &Path = Path::new(runner_config.log_directory.as_str());
            let guest_log_tracker: GuestLogTracker = GuestLogTracker::capture(log_root)?;

            for iteration in 0..iterations {
                let RunnerLogPaths {
                    stdout: _stdout_file_path,
                    stderr: _stderr_file_path,
                } = log_layout.allocate_runner_logs(Some(iteration));

                ::log::info!("empty(): no-op iteration on non-Unix platform (iteration={iteration})");

                guest_log_tracker.move_new_logs(log_layout.test_directory())?;
                log_layout.normalize_component_logs(iteration)?;
            }

            guest_log_tracker.move_new_logs(log_layout.test_directory())?;
            if iterations > 0 {
                let last_iteration: usize = iterations - 1;
                log_layout.normalize_component_logs(last_iteration)?;
            }

            Ok(())
    } => result,
        _ = cancellation_token.cancelled() => {
            error!("empty(): cancellation requested");
            Err(::anyhow::anyhow!("cancelled"))
        },
    }
}
