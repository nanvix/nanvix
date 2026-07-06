// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    DEFAULT_TENANT_ID,
    config::RunnerConfig,
    executor::WorkloadSpec,
    log_layout::{
        GuestLogTracker,
        RunnerLogPaths,
        TestLogLayout,
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
use ::anyhow::Result;
use ::log::{
    error,
    trace,
    warn,
};
use ::std::{
    io::ErrorKind,
    path::Path,
    time::{
        Duration,
        SystemTime,
        UNIX_EPOCH,
    },
};
use ::tokio::time::timeout;
use ::tokio_util::sync::CancellationToken;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests a program in Nanvix using the HTTP executor.
///
/// # Parameters
///
/// - `runner_config`: Configuration required to spawn the Nanvix Daemon and User VMs.
/// - `iterations`: Number of times the start/run/stop cycle should execute.
/// - `workload`: Metadata that describes the workload path, arguments, and expectations.
/// - `log_layout`: Layout that controls how runner/program logs are timestamped and stored.
/// - `extra_nanvixd_args`: Command-line arguments passed directly to nanvixd.
///
/// # Return Value
///
/// Returns `Ok(())` after each iteration successfully spawns the Nanvix Daemon, launches the User
/// VM, optionally injects the custom payload, validates the received bytes, and persists the
/// sanitized stdout capture; returns an error when any lifecycle step fails.
///
pub(crate) async fn test_with_http_executor(
    runner_config: &RunnerConfig,
    iterations: usize,
    workload: WorkloadSpec<'_>,
    log_layout: &TestLogLayout,
    extra_nanvixd_args: &[String],
    cancellation_token: CancellationToken,
) -> Result<()> {
    tokio::select! {
        result = async {
            let hwloc_file_path: Option<String> = runner_config.hwloc_file_path.clone();
            let program_path: String = workload.program_path().to_string();
            let request_payload: Option<Vec<u8>> = workload.input().map(|value| value.as_bytes().to_vec());
            let response_timeout: Duration =
                Duration::from_millis(runner_config.stream_collection_timeout_ms);
            let log_root: &Path = Path::new(runner_config.log_directory.as_str());
            let guest_log_tracker: GuestLogTracker = GuestLogTracker::capture(log_root)?;
            let execution_epoch_ms: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_millis(),
                Err(error) => {
                    let reason: String = format!(
                        "failed to compute execution timestamp (program_path={}, error={error})",
                        program_path
                    );
                    error!("test_with_http_executor(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                },
            };

            let RunnerLogPaths {
                stdout: stdout_file_path,
                stderr: stderr_file_path,
            } = log_layout.allocate_runner_logs(None);

            // Resolve an available port, searching for alternatives if the configured port is in use.
            // Run in a blocking task to avoid stalling the Tokio runtime with synchronous bind calls.
            let ipv4_addr_clone: String = runner_config.ipv4_addr.clone();
            let port_num: u16 = runner_config.port_num;
            let resolved_port: u16 = ::tokio::task::spawn_blocking(move || {
                resolve_http_port(ipv4_addr_clone.as_str(), port_num)
            })
            .await
            .map_err(|e| {
                let reason: String = format!("port resolution task failed (error={e})");
                error!("test_with_http_executor(): {reason}");
                ::anyhow::anyhow!(reason)
            })??;

            // Propagate the resolved port so that the control-plane endpoint used by `UserVm::spawn`
            // matches the port `nanvixd` actually binds to when a fallback port is selected.
            let mut runner_config: RunnerConfig = runner_config.clone();
            runner_config.port_num = resolved_port;
            let runner_config: &RunnerConfig = &runner_config;

            let nanvixd_args: NanvixdHttpArgs = NanvixdHttpArgs::new(
                (stdout_file_path.as_path(), stderr_file_path.as_path()),
                (runner_config.ipv4_addr.as_str(), resolved_port),
                hwloc_file_path.clone(),
                log_layout.test_directory(),
                extra_nanvixd_args,
            )?;

            // Run tests within a scoped block to ensure logs are captured before moving them.
            {
                let _nanvixd_handle: NanvixdHttp = NanvixdHttp::spawn(runner_config, &nanvixd_args).await?;

                for iteration in 0..iterations {
                    let app_name: String = format!("{execution_epoch_ms}-{iteration}");
                    let uservm_args: UserVmArgs = UserVmArgs::new(
                        DEFAULT_TENANT_ID,
                        app_name.as_str(),
                        program_path.as_str(),
                        workload.program_args(),
                        workload.program_env(),
                    )?;

                    let mut user_vm: UserVm = UserVm::spawn(runner_config, &uservm_args).await?;

                    if let Some(payload) = request_payload.as_ref() {
                        send_payload(&mut user_vm, payload.as_slice()).await?;
                    }

                    close_gateway_input(&mut user_vm).await?;

                    let expected_pattern: Option<&[u8]> = workload.expected_output().and_then(|value| {
                        if value.is_empty() {
                            None
                        } else {
                            Some(value.as_bytes())
                        }
                    });

                    let payload: Vec<u8> = receive_payload(
                            &mut user_vm,
                            expected_pattern,
                            response_timeout,
                            workload.expect_empty_output(),
                        ).await?;
                    if workload.expect_empty_output() && !payload.is_empty() {
                        let reason: String = format!(
                            "uservm produced unexpected stdout (bytes={:?}, len={})",
                            payload,
                            payload.len()
                        );
                        error!("test_with_http_executor(): {reason}");
                        return Err(::anyhow::anyhow!(reason));
                    }
                    log_layout.persist_program_output(iteration, payload.as_slice())?;

                    // Explicitly terminate the User VM to get the exit code.
                    let exit_code: i32 = user_vm.terminate().await?;

                    // Validate exit code.
                    if exit_code != workload.expected_exit_code() {
                        let expected: i32 = workload.expected_exit_code();
                        let reason: String = format!(
                            "exit code mismatch (expected={}, actual={}, program={}, iteration={})",
                            expected, exit_code, program_path, iteration
                        );
                        error!("test_with_http_executor(): {reason}");
                        return Err(::anyhow::anyhow!(reason));
                    }
                }
            }

            let last_iteration: usize = iterations.saturating_sub(1);
            guest_log_tracker.move_new_logs(log_layout.test_directory())?;
            log_layout.normalize_component_logs(last_iteration)?;

            Ok(())
        } => result,
        _ = cancellation_token.cancelled() => {
            error!("test_with_http_executor(): cancellation requested");
            Err(::anyhow::anyhow!("cancelled"))
        }
    }
}

///
/// # Description
///
/// Writes the provided payload to the User VM gateway stream.
///
/// # Parameters
///
/// - `user_vm`: Handle to the running User VM gateway stream.
/// - `payload`: Bytes forwarded to the workload over the gateway stream.
///
/// # Return Value
///
/// Returns `Ok(())` after the bytes are written successfully. A `BrokenPipe` or `ConnectionReset`
/// error is treated as success because it means the guest closed the gateway before consuming its
/// input (for example, a workload that exits or faults before reading stdin); the workload's real
/// expectations are validated later via the collected output and exit code. Any other write
/// failure is returned as an error.
///
async fn send_payload(user_vm: &mut UserVm, payload: &[u8]) -> Result<()> {
    trace!("send_payload(): payload_len={}, payload={:?}", payload.len(), payload);
    if let Err(error) = user_vm.gateway_stream().write_all(payload).await {
        match error.kind() {
            // The guest closed the gateway before consuming its input. Mirror the read path
            // (collect_uservm_payload) and the terminal executor (send_interactive_input),
            // which also tolerate a peer that closes early.
            ErrorKind::BrokenPipe | ErrorKind::ConnectionReset => {
                warn!(
                    "send_payload(): gateway closed before payload was sent (error_kind={:?}, \
                     error={error})",
                    error.kind()
                );
            },
            _ => {
                error!("send_payload(): failed to send payload (error={error})");
                return Err(error.into());
            },
        }
    }

    Ok(())
}

///
/// # Description
///
/// Signals end-of-input to the running User VM by shutting down the gateway write half.
///
/// # Return Value
///
/// Returns `Ok(())` when the shutdown succeeds. A `BrokenPipe` or `ConnectionReset` error is
/// treated as success because the guest already closed the gateway (for example, a workload that
/// exits or faults before reading stdin). Any other shutdown failure is returned as an error.
///
async fn close_gateway_input(user_vm: &mut UserVm) -> Result<()> {
    if let Err(error) = user_vm.gateway_stream().shutdown_write().await {
        match error.kind() {
            // The guest already closed the gateway, so shutting down the write half is a no-op
            // from the test's perspective. Tolerate it like send_payload and the read path.
            ErrorKind::BrokenPipe | ErrorKind::ConnectionReset => {
                warn!(
                    "close_gateway_input(): gateway already closed (error_kind={:?}, \
                     error={error})",
                    error.kind()
                );
            },
            _ => {
                let reason: String =
                    format!("failed to shutdown uservm gateway write half (error={error})");
                error!("close_gateway_input(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
        }
    }

    Ok(())
}

///
/// # Description
///
/// Reads a payload from the User VM gateway stream and returns the captured bytes.
///
/// # Parameters
///
/// - `user_vm`: Handle to the running User VM gateway stream.
/// - `expected_pattern`: Byte pattern that must appear in the payload.
/// - `timeout_duration`: Maximum time allowed while waiting for the expected payload.
/// - `expect_empty_output`: Indicates whether EOF/connection reset should map to an empty payload.
///
/// # Return Value
///
/// Returns the captured bytes when the read succeeds; returns an error on socket read failures,
/// pattern mismatches, or timeout expiration.
///
async fn receive_payload(
    user_vm: &mut UserVm,
    expected_pattern: Option<&[u8]>,
    timeout_duration: Duration,
    expect_empty_output: bool,
) -> Result<Vec<u8>> {
    match timeout(
        timeout_duration,
        collect_uservm_payload(user_vm, expected_pattern, expect_empty_output),
    )
    .await
    {
        Ok(result) => result,
        Err(_elapsed) => {
            let reason: String = match expected_pattern {
                Some(pattern) => format!(
                    "uservm payload timed out (timeout_ms={}, expected_pattern={:?})",
                    timeout_duration.as_millis(),
                    pattern
                ),
                None => format!(
                    "uservm payload timed out (timeout_ms={}, expectation=none)",
                    timeout_duration.as_millis()
                ),
            };
            error!("receive_payload(): {reason}");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Collects bytes from the User VM gateway stream until the expected pattern is observed.
///
/// # Parameters
///
/// - `user_vm`: Handle to the running User VM gateway stream.
/// - `expected_pattern`: Byte sequence that must appear in the collected payload.
/// - `expect_empty_output`: Allows EOF/connection-reset to be treated as empty output when set.
///
/// # Return Value
///
/// Returns the captured bytes when the expected pattern is found or the stream closes; returns an
/// error on socket failures or when the pattern never appears.
///
async fn collect_uservm_payload(
    user_vm: &mut UserVm,
    expected_pattern: Option<&[u8]>,
    expect_empty_output: bool,
) -> Result<Vec<u8>> {
    trace!(
        "collect_uservm_payload(): expected_len={}, expected_pattern={:?}",
        expected_pattern.map_or(0, |pattern| pattern.len()),
        expected_pattern
    );
    let mut response_payload: Vec<u8> = Vec::new();
    let mut pattern_found: bool = expected_pattern.is_none();

    loop {
        let mut byte: [u8; 1] = [0u8; 1];
        match user_vm.gateway_stream().read_exact(&mut byte).await {
            Ok(_) => {
                response_payload.push(byte[0]);
            },
            Err(error) => match error.kind() {
                ErrorKind::UnexpectedEof | ErrorKind::ConnectionReset | ErrorKind::BrokenPipe => {
                    if expect_empty_output {
                        if response_payload.is_empty() && expected_pattern.is_none() {
                            warn!(
                                "collect_uservm_payload(): gateway closed before emitting payload \
                                 (error_kind={:?})",
                                error.kind()
                            );
                        } else if !response_payload.is_empty() {
                            warn!(
                                "collect_uservm_payload(): gateway closed after emitting \
                                 unexpected payload (error_kind={:?}, response_len={})",
                                error.kind(),
                                response_payload.len()
                            );
                        }
                    }
                    break;
                },
                _ => {
                    error!("collect_uservm_payload(): failed to read payload (error={error})");
                    return Err(error.into());
                },
            },
        }

        if let Some(pattern) = expected_pattern
            && response_payload.ends_with(pattern)
        {
            pattern_found = true;
            break;
        }
    }

    if let Some(pattern) = expected_pattern
        && !pattern_found
    {
        let reason: String = format!(
            "echo payload mismatch (expected_pattern={:?}, received={:?})",
            pattern, response_payload
        );
        error!("collect_uservm_payload(): {reason}");
        return Err(::anyhow::anyhow!(reason));
    }

    trace!(
        "collect_uservm_payload(): response_len={}, response={:?}",
        response_payload.len(),
        response_payload
    );

    Ok(response_payload)
}
