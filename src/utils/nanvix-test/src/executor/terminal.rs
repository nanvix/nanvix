// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::RunnerConfig,
    executor::{
        WorkloadSpec,
        combine_args_env,
    },
    log_layout::{
        GuestLogTracker,
        RunnerLogPaths,
        TestLogLayout,
    },
    nanvixd::{
        NanvixdTerminal,
        NanvixdTerminalArgs,
    },
};
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    warn,
};
use ::std::{
    fs::write,
    path::Path,
    sync::{
        Arc,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    time::Duration,
};
use ::tokio::{
    io::{
        AsyncRead,
        AsyncReadExt,
        AsyncWriteExt,
    },
    process::{
        ChildStderr,
        ChildStdin,
    },
    sync::{
        Mutex as AsyncMutex,
        Notify,
    },
    task::JoinHandle,
};
use ::tokio_util::sync::CancellationToken;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Governs the size of the intermediate buffer used while collecting stream output.
///
const CHUNK_SIZE: usize = 4096;

///
/// # Description
///
/// Number of times a terminal-executor iteration is retried after a stream-collection timeout
/// before the test is failed. The standalone interactive launch can rarely wedge during VM
/// boot/teardown so that nanvixd never emits stdout and never exits; retrying a bounded number
/// of times absorbs that transient flake without masking a genuinely broken workload (which
/// times out on every attempt).
///
const TERMINAL_STREAM_TIMEOUT_RETRIES: usize = 2;

//==================================================================================================
// Type Definitions
//==================================================================================================

///
/// # Description
///
/// Error returned while collecting a Nanvix Daemon stream, distinguishing a recoverable timeout
/// from a fatal failure.
///
enum StreamCollectError {
    /// Collection exceeded the configured timeout. Recoverable: the caller may retry.
    Timeout(String),
    /// Collection failed for a non-recoverable reason (task join failure or stream I/O error).
    Failed(::anyhow::Error),
}

///
/// # Description
///
/// Outcome of a single terminal-executor attempt that did not produce a result.
///
enum TerminalAttemptError {
    /// A stdout/stderr collection timed out. Recoverable: the iteration may be retried.
    StreamTimeout(String),
    /// A non-recoverable failure occurred; the test must fail.
    Fatal(::anyhow::Error),
}

impl StreamCollectError {
    ///
    /// # Description
    ///
    /// Converts a stream-collection error into the corresponding terminal-attempt error, mapping
    /// a timeout to the recoverable variant and any other failure to the fatal variant.
    ///
    fn into_attempt_error(self) -> TerminalAttemptError {
        match self {
            StreamCollectError::Timeout(reason) => TerminalAttemptError::StreamTimeout(reason),
            StreamCollectError::Failed(error) => TerminalAttemptError::Fatal(error),
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests a program in Nanvix using the terminal executor, aborting when cancellation is requested.
///
/// # Parameters
///
/// - `runner_config`: Configuration required to spawn the Nanvix Daemon.
/// - `iterations`: Number of times to run the workflow.
/// - `workload`: Metadata that describes the workload path, arguments, and expectations.
/// - `log_layout`: Layout that defines the target directory for stdout/stderr/program logs.
/// - `extra_nanvixd_args`: Command-line arguments passed directly to nanvixd.
/// - `cancellation_token`: Token to abort test execution when cancellation is requested.
///
/// # Return Value
///
/// Returns `Ok(())` after all iterations succeed; returns an error if spawning the Nanvix Daemon,
/// wiring stdio pipes, sending input, collecting output, validating stdout, or persisting logs
/// fails.
///
pub async fn test_with_terminal_executor(
    runner_config: &RunnerConfig,
    iterations: usize,
    workload: WorkloadSpec<'_>,
    log_layout: &TestLogLayout,
    extra_nanvixd_args: &[String],
    cancellation_token: CancellationToken,
) -> Result<()> {
    tokio::select! {
        result = async {
            if runner_config.l2_enabled {
                let reason: String = "terminal executor does not support L2 deployment".to_string();
                error!("test_with_terminal_executor(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            }

            let hwloc_file_path: Option<String> = runner_config.hwloc_file_path.clone();
            let parsed_program_args: Vec<String> = match workload.program_args() {
                Some(args) => match shell_words::split(args) {
                    Ok(values) => values,
                    Err(error) => {
                        let reason: String = format!(
                            "failed to parse terminal executor program_args (args='{}', error={error})",
                            args
                        );
                        error!("test_with_terminal_executor(): {reason}");
                        return Err(::anyhow::anyhow!(reason));
                    },
                },
                None => Vec::new(),
            };
            let log_root: &Path = Path::new(runner_config.log_directory.as_str());
            let guest_log_tracker: GuestLogTracker = GuestLogTracker::capture(log_root)?;

            for iteration in 0..iterations {
                let RunnerLogPaths {
                    stdout: stdout_file_path,
                    stderr: stderr_file_path,
                } = log_layout.allocate_runner_logs(Some(iteration));

                // Build a single combined argument string using the shared
                // `combine_args_env()` helper, which implements the documented
                // `<args>;<env>` format with proper `;` escaping.
                let joined_args: String = parsed_program_args.join(" ");
                let combined: String = combine_args_env(
                    if joined_args.is_empty() { None } else { Some(joined_args.as_str()) },
                    workload.program_env(),
                );
                let combined_program_args: Vec<String> = if combined.is_empty() {
                    Vec::new()
                } else {
                    vec![combined]
                };

                let nanvixd_terminal_args: NanvixdTerminalArgs = NanvixdTerminalArgs::new(
                    hwloc_file_path.clone(),
                    workload.program_path(),
                    combined_program_args.as_slice(),
                    log_layout.test_directory(),
                    extra_nanvixd_args,
                )?;

                let collection_timeout: Duration =
                    Duration::from_millis(runner_config.stream_collection_timeout_ms);

                // Defensive retry: the standalone interactive (terminal) launch can,
                // rarely and non-deterministically, wedge during VM boot/teardown such that
                // nanvixd never emits stdout and never exits. Rather than fail the whole suite on
                // a single transient hang, re-spawn nanvixd and retry the iteration a bounded
                // number of times. Each timed-out attempt drops (and thereby kills) its nanvixd
                // before the next attempt, and a genuinely broken workload still fails because
                // every attempt times out identically. The in-process uservm shutdown watchdog
                // covers the common case; this retry is the outer safety net.
                let max_attempts: usize = TERMINAL_STREAM_TIMEOUT_RETRIES + 1;
                let (stdout_bytes, stderr_bytes, exit_code): (Vec<u8>, Vec<u8>, i32) = {
                    let mut attempt: usize = 0;
                    loop {
                        attempt += 1;
                        match run_terminal_attempt(
                            runner_config,
                            &nanvixd_terminal_args,
                            workload.input(),
                            collection_timeout,
                            iteration,
                        )
                        .await
                        {
                            Ok(result) => break result,
                            Err(TerminalAttemptError::StreamTimeout(reason)) => {
                                if attempt >= max_attempts {
                                    error!(
                                        "test_with_terminal_executor(): giving up after \
                                         {attempt} attempt(s) ({reason})"
                                    );
                                    return Err(::anyhow::anyhow!(reason));
                                }
                                warn!(
                                    "test_with_terminal_executor(): {reason}; re-spawning \
                                     nanvixd and retrying (attempt={attempt}/{max_attempts})"
                                );
                            },
                            Err(TerminalAttemptError::Fatal(error)) => return Err(error),
                        }
                    }
                };

                if let Err(error) = write(&stdout_file_path, &stdout_bytes) {
                    let reason: String = format!(
                        "failed to write interactive stdout log (path={}, error={error})",
                        stdout_file_path.display()
                    );
                    error!("test_with_terminal_executor(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                }

                if let Err(error) = write(&stderr_file_path, &stderr_bytes) {
                    let reason: String = format!(
                        "failed to write interactive stderr log (path={}, error={error})",
                        stderr_file_path.display()
                    );
                    error!("test_with_terminal_executor(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                }

                log_layout.persist_program_output(iteration, stdout_bytes.as_slice())?;

                if let Some(expected) = workload.expected_output()
                    && !buffer_contains_pattern(stdout_bytes.as_slice(), expected.as_bytes())
                {
                    let reason: String = format!(
                        "interactive output mismatch (expected='{}', log={}, iteration={iteration})",
                        expected,
                        stdout_file_path.display()
                    );
                    error!("test_with_terminal_executor(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                }

                if workload.expect_empty_output() && !stdout_bytes.is_empty() {
                    let reason: String = format!(
                        "interactive output is not empty as required (bytes={:?}, iteration={iteration})",
                        stdout_bytes
                    );
                    error!("test_with_terminal_executor(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                }

                // Validate exit code.
                 if exit_code != workload.expected_exit_code() {
                    let expected: i32 = workload.expected_exit_code();
                    let reason: String = format!(
                        "exit code mismatch (expected={}, actual={}, program={}, iteration={})",
                        expected,
                        exit_code,
                        workload.program_path(),
                        iteration
                    );
                    error!("test_with_terminal_executor(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
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
            error!("test_with_terminal_executor(): cancellation requested");
            Err(::anyhow::anyhow!("cancelled"))
        }
    }
}

///
/// # Description
///
/// Runs a single terminal-executor attempt: spawns nanvixd in interactive mode, wires its
/// stdin/stdout/stderr, forwards the workload input, and collects the guest output and exit code.
///
/// The spawned nanvixd is owned by this function, so it is dropped -- and thereby killed via its
/// `Drop` implementation -- whenever this function returns early with an error (including a
/// stream-collection timeout). This guarantees the previous nanvixd is gone before the caller
/// re-spawns for a retry.
///
/// # Parameters
///
/// - `runner_config`: Configuration required to spawn the Nanvix Daemon.
/// - `nanvixd_terminal_args`: Arguments describing the interactive workload to launch.
/// - `input`: Optional payload forwarded to the workload over stdin.
/// - `collection_timeout`: Maximum time to wait for each of stdout and stderr to be collected.
/// - `iteration`: Index of the current iteration, used for diagnostics.
///
/// # Return Value
///
/// On success, returns the collected stdout bytes, stderr bytes, and the nanvixd exit code.
/// Returns [`TerminalAttemptError::StreamTimeout`] when stream collection times out (recoverable)
/// or [`TerminalAttemptError::Fatal`] for any other failure.
///
async fn run_terminal_attempt(
    runner_config: &RunnerConfig,
    nanvixd_terminal_args: &NanvixdTerminalArgs,
    input: Option<&str>,
    collection_timeout: Duration,
    iteration: usize,
) -> ::std::result::Result<(Vec<u8>, Vec<u8>, i32), TerminalAttemptError> {
    let mut nanvixd: NanvixdTerminal = NanvixdTerminal::spawn(runner_config, nanvixd_terminal_args)
        .await
        .map_err(TerminalAttemptError::Fatal)?;

    let stdout_pipe = nanvixd.take_stdout().ok_or_else(|| {
        let reason: String = "interactive mode requires capturing nanvixd stdout".to_string();
        error!("run_terminal_attempt(): {reason}");
        TerminalAttemptError::Fatal(::anyhow::anyhow!(reason))
    })?;
    let stdout_buffer: Arc<AsyncMutex<Vec<u8>>> = Arc::new(AsyncMutex::new(Vec::new()));
    let stdout_handle: JoinHandle<::std::io::Result<()>> = ::tokio::spawn(
        collect_stream_to_buffer(stdout_pipe, Arc::clone(&stdout_buffer), None, None),
    );

    let stderr_pipe: ChildStderr = match nanvixd.take_stderr() {
        Some(pipe) => pipe,
        None => {
            stdout_handle.abort();
            let reason: String = "interactive mode requires capturing nanvixd stderr".to_string();
            error!("run_terminal_attempt(): {reason}");
            return Err(TerminalAttemptError::Fatal(::anyhow::anyhow!(reason)));
        },
    };
    let stderr_buffer: Arc<AsyncMutex<Vec<u8>>> = Arc::new(AsyncMutex::new(Vec::new()));
    let stderr_handle: JoinHandle<::std::io::Result<()>> = ::tokio::spawn(
        collect_stream_to_buffer(stderr_pipe, Arc::clone(&stderr_buffer), None, None),
    );

    let stdin_pipe: ChildStdin = match nanvixd.take_stdin() {
        Some(pipe) => pipe,
        None => {
            stdout_handle.abort();
            stderr_handle.abort();
            let reason: String = "interactive mode does not expose stdin pipe".to_string();
            error!("run_terminal_attempt(): {reason}");
            return Err(TerminalAttemptError::Fatal(::anyhow::anyhow!(reason)));
        },
    };
    if let Err(error) = send_interactive_input(stdin_pipe, input).await {
        stdout_handle.abort();
        stderr_handle.abort();
        return Err(TerminalAttemptError::Fatal(error));
    }

    let stdout_bytes: Vec<u8> = match wait_stream_collector(
        stdout_handle,
        Arc::clone(&stdout_buffer),
        collection_timeout,
        "stdout",
        iteration,
    )
    .await
    {
        Ok(bytes) => bytes,
        Err(error) => {
            // Abort the still-running stderr collector before bailing so a retry does not
            // accumulate a detached task holding pipe fds / buffers.
            stderr_handle.abort();
            return Err(error.into_attempt_error());
        },
    };

    let stderr_bytes: Vec<u8> = wait_stream_collector(
        stderr_handle,
        Arc::clone(&stderr_buffer),
        collection_timeout,
        "stderr",
        iteration,
    )
    .await
    .map_err(StreamCollectError::into_attempt_error)?;

    // Wait for the nanvixd process to exit and get its exit code.
    let exit_code: i32 = nanvixd
        .wait_exit_code()
        .await
        .map_err(TerminalAttemptError::Fatal)?;

    Ok((stdout_bytes, stderr_bytes, exit_code))
}

///
/// # Description
///
/// Consumes the Nanvix Daemon stdin handle, sends the provided payload, and shuts down the pipe.
///
/// When the nanvixd process exits before all stdin operations complete, the OS closes the read end
/// of the pipe. On Windows this produces OS error 232 (`ERROR_NO_DATA`), which Rust maps to
/// [`std::io::ErrorKind::BrokenPipe`]. Because the exit-code validation that runs later will catch
/// any unexpected termination, broken-pipe errors are demoted to warnings and do not fail the test.
///
/// # Parameters
///
/// - `stdin`: Writable handle to Nanvix Daemon stdin that is consumed by this function.
/// - `payload`: Optional string sent to the workload over stdin.
///
/// # Return Value
///
/// Returns `Ok(())` after stdin is written, flushed, and closed successfully, or when the pipe is
/// broken because nanvixd already exited. Returns an error for non-broken-pipe I/O failures.
///
async fn send_interactive_input(mut stdin: ChildStdin, payload: Option<&str>) -> Result<()> {
    if let Some(data) = payload {
        let mut bytes: Vec<u8> = data.as_bytes().to_vec();
        if bytes.last().copied() != Some(b'\n') {
            bytes.push(b'\n');
        }

        if let Err(error) = stdin.write_all(bytes.as_slice()).await {
            if error.kind() == ::std::io::ErrorKind::BrokenPipe {
                warn!(
                    "send_interactive_input(): nanvixd stdin pipe closed during write \
                     (error={error})"
                );
                return Ok(());
            }
            error!("send_interactive_input(): failed to write payload (error={error})");
            return Err(error.into());
        }

        if let Err(error) = stdin.flush().await {
            if error.kind() == ::std::io::ErrorKind::BrokenPipe {
                warn!(
                    "send_interactive_input(): nanvixd stdin pipe closed during flush \
                     (error={error})"
                );
                return Ok(());
            }
            error!("send_interactive_input(): failed to flush stdin (error={error})");
            return Err(error.into());
        }
    }

    if let Err(error) = stdin.shutdown().await {
        if error.kind() == ::std::io::ErrorKind::BrokenPipe {
            warn!(
                "send_interactive_input(): nanvixd stdin pipe closed during shutdown \
                 (error={error})"
            );
            return Ok(());
        }
        error!("send_interactive_input(): failed to shutdown stdin (error={error})");
        return Err(error.into());
    }

    Ok(())
}

///
/// # Description
///
/// Reads bytes from a Nanvix Daemon stream, stores them in a shared buffer, and notifies consumers when
/// new data arrives or the stream closes.
///
/// # Parameters
///
/// - `reader`: Async stream reader (stdout or stderr).
/// - `buffer`: Shared buffer that accumulates the captured bytes.
/// - `notify`: Optional notifier used to wake tasks waiting for new data.
/// - `closed_flag`: Optional flag set when the stream reaches EOF.
///
/// # Return Value
///
/// Returns `Ok(())` after the stream closes; returns an error if any I/O operation fails during
/// collection.
///
async fn collect_stream_to_buffer<R>(
    mut reader: R,
    buffer: Arc<AsyncMutex<Vec<u8>>>,
    notify: Option<Arc<Notify>>,
    closed_flag: Option<Arc<AtomicBool>>,
) -> ::std::io::Result<()>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut chunk: [u8; CHUNK_SIZE] = [0u8; CHUNK_SIZE];
    loop {
        let bytes_read: usize = reader.read(&mut chunk).await?;
        if bytes_read == 0 {
            if let Some(flag) = &closed_flag {
                flag.store(true, Ordering::SeqCst);
            }
            if let Some(notifier) = &notify {
                notifier.notify_waiters();
            }
            break;
        }

        {
            let mut guard = buffer.lock().await;
            guard.extend_from_slice(&chunk[..bytes_read]);
        }

        if let Some(notifier) = &notify {
            notifier.notify_waiters();
        }
    }

    Ok(())
}

///
/// # Description
///
/// Waits for a stream collector to finish, returning the accumulated bytes or surfacing errors.
///
/// # Parameters
///
/// - `handle`: Join handle for the collector task.
/// - `buffer`: Shared buffer populated by the collector.
/// - `timeout`: Maximum time allowed while waiting for stream shutdown.
/// - `label`: Textual label (stdout/stderr) used in error messages.
/// - `iteration`: Iteration index for logging context.
///
/// # Return Value
///
/// Returns the buffered bytes when the collector finishes successfully; returns an error on join
/// failures, I/O errors, or timeouts.
///
async fn wait_stream_collector(
    mut handle: JoinHandle<::std::io::Result<()>>,
    buffer: Arc<AsyncMutex<Vec<u8>>>,
    timeout: Duration,
    label: &str,
    iteration: usize,
) -> ::std::result::Result<Vec<u8>, StreamCollectError> {
    let join_result = match ::tokio::time::timeout(timeout, &mut handle).await {
        Ok(result) => result,
        Err(_elapsed) => {
            let reason: String = format!(
                "timed out while waiting for interactive {label} (iteration={}, timeout_ms={})",
                iteration,
                timeout.as_millis()
            );
            // Abort the still-running collector task so its Arc buffer / pipe reader is
            // released now instead of being detached to run until the process exits.
            handle.abort();
            // Logged at debug here because the caller decides whether this is recoverable
            // (retryable) or final, and emits the user-facing warning/error accordingly.
            debug!("wait_stream_collector(): {reason}");
            return Err(StreamCollectError::Timeout(reason));
        },
    };

    match join_result {
        Err(join_error) => {
            let reason: String = format!(
                "failed to join {label} collector task (iteration={}, error={join_error})",
                iteration
            );
            error!("wait_stream_collector(): {reason}");
            return Err(StreamCollectError::Failed(::anyhow::anyhow!(reason)));
        },
        Ok(Err(io_error)) => {
            let reason: String = format!(
                "failed to read {label} stream from nanvixd (iteration={}, error={io_error})",
                iteration
            );
            error!("wait_stream_collector(): {reason}");
            return Err(StreamCollectError::Failed(::anyhow::anyhow!(reason)));
        },
        Ok(Ok(())) => {},
    }

    let bytes: Vec<u8> = buffer.lock().await.clone();
    Ok(bytes)
}

///
/// # Description
///
/// Checks whether a byte buffer contains the desired pattern.
///
/// # Parameters
///
/// - `buffer`: Captured byte sequence examined for the desired pattern.
/// - `pattern`: Target byte sequence that must appear in `buffer`.
///
/// # Return Value
///
/// Returns `true` when the pattern is found or when the expected pattern is empty. Returns
/// `false` when the pattern is non-empty but the buffer is empty.
///
fn buffer_contains_pattern(buffer: &[u8], pattern: &[u8]) -> bool {
    if pattern.is_empty() && buffer.is_empty() {
        true
    } else if pattern.is_empty() ^ buffer.is_empty() {
        false
    } else {
        buffer
            .windows(pattern.len())
            .any(|window| window == pattern)
    }
}
