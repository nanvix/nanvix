// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Signal executor.
//!
//! This executor checks that the Nanvix Daemon shuts down gracefully when it receives a shutdown
//! signal while a workload is running: the daemon must exit successfully, and it must remove the
//! gateway socket of the workload instead of leaking it.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(unix)]
use crate::{
    DEFAULT_TENANT_ID,
    executor::http::{
        receive_payload,
        send_payload,
    },
    log_layout::{
        GuestLogTracker,
        RunnerLogPaths,
    },
    nanvixd::{
        NanvixdHttp,
        NanvixdHttpArgs,
    },
    port::resolve_http_port,
    uservm::{
        UserVm,
        UserVmArgs,
    },
};
use crate::{
    config::RunnerConfig,
    executor::WorkloadSpec,
    log_layout::TestLogLayout,
};
use ::anyhow::Result;
use ::log::error;
#[cfg(unix)]
use ::log::trace;
#[cfg(unix)]
use ::std::{
    path::Path,
    time::{
        Duration,
        SystemTime,
        UNIX_EPOCH,
    },
};
#[cfg(unix)]
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::TcpStream,
    task::JoinHandle,
    time::{
        sleep,
        timeout,
    },
};
use ::tokio_util::sync::CancellationToken;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum time to wait for the Nanvix Daemon to exit after it is signaled, and for an in-flight
/// KILL request to complete after that.
///
/// The Nanvix Daemon tears the workload down and exits within a couple of seconds. This limit is
/// far larger, to absorb slow hosts, while still catching a daemon that ignores the signal or that
/// hangs on exit.
#[cfg(unix)]
const NANVIXD_EXIT_TIMEOUT: Duration = Duration::from_secs(30);

/// Readiness probe of the Nanvix Daemon, sent on a connection of its own.
#[cfg(unix)]
const READINESS_PROBE: &str = "GET /ready HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";

/// Maximum time to wait for the answer to a readiness probe. A Nanvix Daemon that does not answer
/// in time is deemed busy serving another request.
#[cfg(unix)]
const READINESS_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// Maximum number of readiness probes sent while waiting for the Nanvix Daemon to serve a KILL
/// request.
#[cfg(unix)]
const READINESS_PROBE_ATTEMPTS_MAX: usize = 100;

/// Delay between readiness probes.
#[cfg(unix)]
const READINESS_PROBE_INTERVAL: Duration = Duration::from_millis(10);

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Describes how the signal executor signals the Nanvix Daemon.
///
#[derive(Clone, Copy)]
pub(crate) struct SignalSpec<'a> {
    /// Name of the signal sent to the Nanvix Daemon (e.g., `SIGTERM`).
    name: &'a str,
    /// Whether a KILL request is in flight when the Nanvix Daemon is signaled.
    #[cfg_attr(not(unix), allow(dead_code))]
    kill_in_flight: bool,
}

impl<'a> SignalSpec<'a> {
    ///
    /// # Description
    ///
    /// Creates a new description of how the signal executor signals the Nanvix Daemon.
    ///
    /// # Parameters
    ///
    /// - `name`: Name of the signal sent to the Nanvix Daemon (e.g., `SIGTERM`).
    /// - `kill_in_flight`: Whether a KILL request is in flight when the Nanvix Daemon is signaled.
    ///
    /// # Return Value
    ///
    /// Returns the description of how the Nanvix Daemon is signaled.
    ///
    pub(crate) const fn new(name: &'a str, kill_in_flight: bool) -> Self {
        Self {
            name,
            kill_in_flight,
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests that the Nanvix Daemon shuts down gracefully when it receives a signal while a workload
/// is running.
///
/// Each iteration spawns the Nanvix Daemon in HTTP mode, starts the workload through a NEW
/// request, and exchanges the input and expected output of the workload with it, if any, to make
/// sure that it is up and running. The input of the workload is never closed, so that a workload
/// that reads its input until the end keeps running. If requested, the iteration then sends a KILL
/// request, which waits for the workload to exit, as the containerd shim does to learn the exit
/// code of the workload. Finally, the iteration sends the signal to the Nanvix Daemon, and checks
/// that the daemon exits successfully, that it cancels the in-flight KILL request, if any, and
/// that it removes the gateway socket of the workload.
///
/// # Parameters
///
/// - `runner_config`: Configuration required to spawn the Nanvix Daemon and User VMs.
/// - `iterations`: Number of times the start/signal cycle should execute.
/// - `workload`: Metadata that describes the workload path, arguments, and expectations.
/// - `signal`: Description of how the Nanvix Daemon is signaled.
/// - `log_layout`: Layout that controls how runner/program logs are timestamped and stored.
/// - `extra_nanvixd_args`: Command-line arguments passed directly to nanvixd.
/// - `cancellation_token`: Token that cancels the test when the runner is interrupted.
///
/// # Return Value
///
/// Returns `Ok(())` after every iteration observes a graceful shutdown of the Nanvix Daemon;
/// returns an error when a lifecycle step fails, when the Nanvix Daemon does not exit
/// successfully in time, or when it leaks the gateway socket of the workload.
///
#[cfg(unix)]
pub(crate) async fn test_with_signal_executor(
    runner_config: &RunnerConfig,
    iterations: usize,
    workload: WorkloadSpec<'_>,
    signal: SignalSpec<'_>,
    log_layout: &TestLogLayout,
    extra_nanvixd_args: &[String],
    cancellation_token: CancellationToken,
) -> Result<()> {
    let signal_number: ::libc::c_int = signal_number(signal.name)?;

    let result: Option<Result<()>> = cancellation_token
        .run_until_cancelled(async {
            let log_root: &Path = Path::new(runner_config.log_directory.as_str());
            let guest_log_tracker: GuestLogTracker = GuestLogTracker::capture(log_root)?;
            let execution_epoch_ms: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_millis(),
                Err(error) => {
                    let reason: String =
                        format!("failed to compute execution timestamp (error={error})");
                    error!("test_with_signal_executor(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                },
            };

            for iteration in 0..iterations {
                let app_name: String = format!("{execution_epoch_ms}-{iteration}");
                signal_nanvixd_while_running(
                    runner_config,
                    workload,
                    (signal_number, signal),
                    log_layout,
                    extra_nanvixd_args,
                    (app_name.as_str(), iteration),
                )
                .await?;

                // Every iteration runs its own Nanvix Daemon, so file its logs right away.
                guest_log_tracker.move_new_logs(log_layout.test_directory())?;
                log_layout.normalize_component_logs(iteration)?;
            }

            Ok(())
        })
        .await;

    match result {
        Some(result) => result,
        None => {
            error!("test_with_signal_executor(): cancellation requested");
            Err(::anyhow::anyhow!("cancelled"))
        },
    }
}

///
/// # Description
///
/// Fails, as signals are only supported on Unix.
///
/// # Return Value
///
/// Returns an error.
///
#[cfg(not(unix))]
pub(crate) async fn test_with_signal_executor(
    _runner_config: &RunnerConfig,
    _iterations: usize,
    _workload: WorkloadSpec<'_>,
    signal: SignalSpec<'_>,
    _log_layout: &TestLogLayout,
    _extra_nanvixd_args: &[String],
    _cancellation_token: CancellationToken,
) -> Result<()> {
    let reason: String =
        format!("cannot send {} to nanvixd: signals are only supported on Unix", signal.name);
    error!("test_with_signal_executor(): {reason}");
    Err(::anyhow::anyhow!(reason))
}

///
/// # Description
///
/// Runs one iteration of the signal executor: spawns the Nanvix Daemon, starts the workload, and
/// signals the Nanvix Daemon while the workload is running.
///
/// # Parameters
///
/// - `runner_config`: Configuration required to spawn the Nanvix Daemon and User VMs.
/// - `workload`: Metadata that describes the workload path, arguments, and expectations.
/// - `signal`: Number of the signal sent to the Nanvix Daemon, and description of how the Nanvix
///   Daemon is signaled.
/// - `log_layout`: Layout that controls how runner/program logs are timestamped and stored.
/// - `extra_nanvixd_args`: Command-line arguments passed directly to nanvixd.
/// - `run`: Application name of the workload, and index of the iteration.
///
/// # Return Value
///
/// Returns `Ok(())` when the Nanvix Daemon shuts down gracefully; returns an error when a
/// lifecycle step fails, when the Nanvix Daemon does not exit successfully in time, when it does
/// not cancel the in-flight KILL request, or when it leaks the gateway socket of the workload.
///
#[cfg(unix)]
async fn signal_nanvixd_while_running(
    runner_config: &RunnerConfig,
    workload: WorkloadSpec<'_>,
    signal: (::libc::c_int, SignalSpec<'_>),
    log_layout: &TestLogLayout,
    extra_nanvixd_args: &[String],
    run: (&str, usize),
) -> Result<()> {
    let (signal_number, signal): (::libc::c_int, SignalSpec<'_>) = signal;
    let signal_name: &str = signal.name;
    let (app_name, iteration): (&str, usize) = run;
    let response_timeout: Duration =
        Duration::from_millis(runner_config.stream_collection_timeout_ms);

    let RunnerLogPaths {
        stdout: stdout_file_path,
        stderr: stderr_file_path,
    } = log_layout.allocate_runner_logs(Some(iteration));

    // Each iteration stops its Nanvix Daemon, so the port is resolved anew every time. Run in a
    // blocking task to avoid stalling the Tokio runtime with synchronous bind calls.
    let ipv4_addr: String = runner_config.ipv4_addr.clone();
    let port_num: u16 = runner_config.port_num;
    let resolved_port: u16 =
        ::tokio::task::spawn_blocking(move || resolve_http_port(ipv4_addr.as_str(), port_num))
            .await
            .map_err(|e| {
                let reason: String = format!("port resolution task failed (error={e})");
                error!("signal_nanvixd_while_running(): {reason}");
                ::anyhow::anyhow!(reason)
            })??;

    // Propagate the resolved port so that the control-plane endpoint used by `UserVm::spawn`
    // matches the port that the Nanvix Daemon binds to.
    let mut runner_config: RunnerConfig = runner_config.clone();
    runner_config.port_num = resolved_port;

    let nanvixd_args: NanvixdHttpArgs = NanvixdHttpArgs::new(
        (stdout_file_path.as_path(), stderr_file_path.as_path()),
        (runner_config.ipv4_addr.as_str(), resolved_port),
        log_layout.test_directory(),
        extra_nanvixd_args,
    )?;
    let mut nanvixd: NanvixdHttp = NanvixdHttp::spawn(&runner_config, &nanvixd_args).await?;

    let uservm_args: UserVmArgs = UserVmArgs::new(
        DEFAULT_TENANT_ID,
        app_name,
        workload.program_path(),
        workload.program_args(),
        workload.program_env(),
    )?;
    let mut user_vm: UserVm = UserVm::spawn(&runner_config, &uservm_args).await?;
    // The Nanvix Daemon takes the User VM down when it shuts down, so the handle must not
    // terminate it, which would wait for a workload that never exits on its own.
    user_vm.disown();

    // Exchange a payload with the workload, to make sure that it is up and running before the
    // Nanvix Daemon is signaled.
    if let Some(input) = workload.input() {
        send_payload(&mut user_vm, input.as_bytes()).await?;
    }
    if let Some(expected_output) = workload
        .expected_output()
        .filter(|output| !output.is_empty())
    {
        receive_payload(&mut user_vm, Some(expected_output.as_bytes()), response_timeout, false)
            .await?;
    }

    // Keep a KILL request in flight, if requested. The request waits for a workload that never
    // exits on its own, so the Nanvix Daemon has to cancel it to shut down.
    let kill_task: Option<JoinHandle<Result<i32>>> = if signal.kill_in_flight {
        let kill_task: JoinHandle<Result<i32>> = user_vm.spawn_kill();
        wait_until_serving_kill(runner_config.http_endpoint().as_str(), &kill_task).await?;
        Some(kill_task)
    } else {
        None
    };

    let exit_code: i32 = nanvixd
        .signal_and_wait_exit(signal_number, NANVIXD_EXIT_TIMEOUT)
        .await?;
    if exit_code != 0 {
        let reason: String = format!(
            "nanvixd exited with a non-zero exit code after {signal_name} (exit_code={exit_code}, \
             iteration={iteration})"
        );
        error!("signal_nanvixd_while_running(): {reason}");
        return Err(::anyhow::anyhow!(reason));
    }

    if let Some(kill_task) = kill_task {
        check_kill_cancelled(kill_task, signal_name, iteration).await?;
    }

    // Check the gateway socket before the Nanvix Daemon handle is dropped, as dropping it
    // sanitizes the temporary directory where the socket lives.
    let gateway_sockaddr: &str = user_vm.gateway_sockaddr();
    match Path::new(gateway_sockaddr).try_exists() {
        Ok(false) => {
            trace!(
                "signal_nanvixd_while_running(): gateway socket removed (path={gateway_sockaddr}, \
                 signal={signal_name}, iteration={iteration})"
            );
            Ok(())
        },
        Ok(true) => {
            let reason: String = format!(
                "nanvixd leaked the gateway socket after {signal_name} (path={gateway_sockaddr}, \
                 iteration={iteration})"
            );
            error!("signal_nanvixd_while_running(): {reason}");
            Err(::anyhow::anyhow!(reason))
        },
        Err(error) => {
            let reason: String = format!(
                "failed to check the gateway socket (path={gateway_sockaddr}, error={error})"
            );
            error!("signal_nanvixd_while_running(): {reason}");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Waits until the Nanvix Daemon serves an in-flight KILL request.
///
/// The Nanvix Daemon serves connections one at a time, and a KILL request waits for the workload
/// to exit, so the daemon stops answering readiness probes once it serves the KILL request.
///
/// # Parameters
///
/// - `http_endpoint`: Address of the HTTP endpoint of the Nanvix Daemon.
/// - `kill_task`: Task that sends the KILL request.
///
/// # Return Value
///
/// Returns `Ok(())` once the Nanvix Daemon stops answering readiness probes; returns an error if
/// the KILL request completes first, or if the Nanvix Daemon keeps answering readiness probes.
///
#[cfg(unix)]
async fn wait_until_serving_kill(
    http_endpoint: &str,
    kill_task: &JoinHandle<Result<i32>>,
) -> Result<()> {
    for _ in 0..READINESS_PROBE_ATTEMPTS_MAX {
        if kill_task.is_finished() {
            let reason: String =
                "KILL request completed before nanvixd was signaled (workload exited?)".to_string();
            error!("wait_until_serving_kill(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        }
        if timeout(READINESS_PROBE_TIMEOUT, probe_readiness(http_endpoint))
            .await
            .is_err()
        {
            trace!("wait_until_serving_kill(): nanvixd is serving the KILL request");
            return Ok(());
        }
        sleep(READINESS_PROBE_INTERVAL).await;
    }

    let reason: String = format!(
        "nanvixd kept answering readiness probes while a KILL request was pending \
         (attempts={READINESS_PROBE_ATTEMPTS_MAX})"
    );
    error!("wait_until_serving_kill(): {reason}");
    Err(::anyhow::anyhow!(reason))
}

///
/// # Description
///
/// Sends a readiness probe to the Nanvix Daemon, and reads its answer.
///
/// # Parameters
///
/// - `http_endpoint`: Address of the HTTP endpoint of the Nanvix Daemon.
///
/// # Return Value
///
/// Returns `Ok(())` once the Nanvix Daemon answers and closes the connection; returns an error if
/// the probe cannot be sent or its answer cannot be read.
///
#[cfg(unix)]
async fn probe_readiness(http_endpoint: &str) -> ::std::io::Result<()> {
    let mut stream: TcpStream = TcpStream::connect(http_endpoint).await?;
    stream.write_all(READINESS_PROBE.as_bytes()).await?;
    let mut answer: Vec<u8> = Vec::new();
    stream.read_to_end(&mut answer).await?;
    Ok(())
}

///
/// # Description
///
/// Checks that the Nanvix Daemon cancelled an in-flight KILL request when it shut down.
///
/// # Parameters
///
/// - `kill_task`: Task that sent the KILL request.
/// - `signal_name`: Name of the signal sent to the Nanvix Daemon.
/// - `iteration`: Index of the iteration.
///
/// # Return Value
///
/// Returns `Ok(())` when the KILL request failed, as it does when the Nanvix Daemon closes its
/// connection; returns an error when it does not complete in time, or when it reports the exit
/// code of the workload, which means that the workload exited on its own.
///
#[cfg(unix)]
async fn check_kill_cancelled(
    kill_task: JoinHandle<Result<i32>>,
    signal_name: &str,
    iteration: usize,
) -> Result<()> {
    match timeout(NANVIXD_EXIT_TIMEOUT, kill_task).await {
        Ok(Ok(Err(error))) => {
            trace!(
                "check_kill_cancelled(): KILL request cancelled (signal={signal_name}, \
                 iteration={iteration}, error={error})"
            );
            Ok(())
        },
        Ok(Ok(Ok(exit_code))) => {
            let reason: String = format!(
                "KILL request completed although the workload was still running \
                 (signal={signal_name}, exit_code={exit_code}, iteration={iteration})"
            );
            error!("check_kill_cancelled(): {reason}");
            Err(::anyhow::anyhow!(reason))
        },
        Ok(Err(error)) => {
            let reason: String = format!("KILL request task failed (error={error})");
            error!("check_kill_cancelled(): {reason}");
            Err(::anyhow::anyhow!(reason))
        },
        Err(_elapsed) => {
            let reason: String = format!(
                "KILL request still in flight after nanvixd exited (signal={signal_name}, \
                 iteration={iteration})"
            );
            error!("check_kill_cancelled(): {reason}");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Converts the name of a signal into its number.
///
/// # Parameters
///
/// - `signal_name`: Name of the signal (e.g., `SIGTERM`).
///
/// # Return Value
///
/// Returns the number of the signal; returns an error when the signal is not one of the signals
/// that the Nanvix Daemon handles to shut down gracefully.
///
#[cfg(unix)]
fn signal_number(signal_name: &str) -> Result<::libc::c_int> {
    match signal_name {
        "SIGINT" => Ok(::libc::SIGINT),
        "SIGTERM" => Ok(::libc::SIGTERM),
        "SIGHUP" => Ok(::libc::SIGHUP),
        "SIGQUIT" => Ok(::libc::SIGQUIT),
        _ => {
            let reason: String = format!("unsupported signal '{signal_name}'");
            error!("signal_number(): {reason}");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::config::TestCaseConfig;

    #[test]
    fn signal_number_converts_known_signals() -> Result<()> {
        for signal_name in TestCaseConfig::KNOWN_SIGNALS {
            signal_number(signal_name)?;
        }
        assert_eq!(signal_number("SIGTERM")?, ::libc::SIGTERM);
        Ok(())
    }

    #[test]
    fn signal_number_rejects_unknown_signals() {
        assert!(signal_number("SIGKILL").is_err(), "SIGKILL cannot be handled by nanvixd");
        assert!(signal_number("sigterm").is_err(), "signal names are case-sensitive");
    }
}
