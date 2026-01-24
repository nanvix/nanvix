// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::RunnerConfig,
    executor::WorkloadSpec,
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
use ::nanvix::log::error;
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

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Governs the size of the intermediate buffer used while collecting stream output.
///
const CHUNK_SIZE: usize = 4096;

//==================================================================================================
// Type Definitions
//==================================================================================================

///
/// # Description
///
/// Bundles the async handles and buffers that collect Nanvix Daemon stdout and stderr streams.
///
type StreamCollectors = (
    JoinHandle<::std::io::Result<()>>,
    Arc<AsyncMutex<Vec<u8>>>,
    JoinHandle<::std::io::Result<()>>,
    Arc<AsyncMutex<Vec<u8>>>,
);

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests a program in Nanvix using the terminal executor.
///
/// # Parameters
///
/// - `runner_config`: Configuration required to spawn the Nanvix Daemon.
/// - `iterations`: Number of times to run the workflow.
/// - `workload`: Metadata that describes the workload path, arguments, and expectations.
/// - `log_layout`: Layout that defines the target directory for stdout/stderr/program logs.
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
) -> Result<()> {
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

        let nanvixd_args: NanvixdTerminalArgs = NanvixdTerminalArgs::new(
            hwloc_file_path.clone(),
            workload.program_path(),
            parsed_program_args.as_slice(),
            log_layout.test_directory(),
        )?;

        let collection_timeout: Duration =
            Duration::from_millis(runner_config.stream_collection_timeout_ms);

        let (_nanvixd, stream_collectors) = {
            let mut nanvixd: NanvixdTerminal =
                NanvixdTerminal::spawn(runner_config, &nanvixd_args).await?;

            let stdout_pipe = nanvixd.take_stdout().ok_or_else(|| {
                let reason: String =
                    "interactive mode requires capturing nanvixd stdout".to_string();
                error!("test_with_terminal_executor(): {reason}");
                ::anyhow::anyhow!(reason)
            })?;

            let stdout_buffer: Arc<AsyncMutex<Vec<u8>>> = Arc::new(AsyncMutex::new(Vec::new()));
            let stdout_handle: JoinHandle<::std::io::Result<()>> = ::tokio::spawn(
                collect_stream_to_buffer(stdout_pipe, Arc::clone(&stdout_buffer), None, None),
            );

            let stderr_pipe: ChildStderr = nanvixd.take_stderr().ok_or_else(|| {
                let reason: String =
                    "interactive mode requires capturing nanvixd stderr".to_string();
                error!("test_with_terminal_executor(): {reason}");
                ::anyhow::anyhow!(reason)
            })?;

            let stderr_buffer: Arc<AsyncMutex<Vec<u8>>> = Arc::new(AsyncMutex::new(Vec::new()));
            let stderr_handle: JoinHandle<::std::io::Result<()>> = ::tokio::spawn(
                collect_stream_to_buffer(stderr_pipe, Arc::clone(&stderr_buffer), None, None),
            );

            let stdin_pipe: ChildStdin = nanvixd.take_stdin().ok_or_else(|| {
                let reason: String = "interactive mode does not expose stdin pipe".to_string();
                error!("test_with_terminal_executor(): {reason}");
                ::anyhow::anyhow!(reason)
            })?;

            send_interactive_input(stdin_pipe, workload.input()).await?;

            (nanvixd, (stdout_handle, stdout_buffer, stderr_handle, stderr_buffer))
        };

        let (stdout_handle, stdout_buffer, stderr_handle, stderr_buffer): StreamCollectors =
            stream_collectors;

        let stdout_bytes: Vec<u8> = wait_stream_collector(
            stdout_handle,
            Arc::clone(&stdout_buffer),
            collection_timeout,
            "stdout",
            iteration,
        )
        .await?;
        let stderr_bytes: Vec<u8> = wait_stream_collector(
            stderr_handle,
            Arc::clone(&stderr_buffer),
            collection_timeout,
            "stderr",
            iteration,
        )
        .await?;

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

///
/// # Description
///
/// Consumes the Nanvix Daemon stdin handle, sends the provided payload, and shuts down the pipe.
///
/// # Parameters
///
/// - `stdin`: Writable handle to Nanvix Daemon stdin that is consumed by this function.
/// - `payload`: Optional string sent to the workload over stdin.
///
/// # Return Value
///
/// Returns `Ok(())` after stdin is written, flushed, and closed successfully; returns an error on
/// write, flush, or shutdown failures.
///
async fn send_interactive_input(mut stdin: ChildStdin, payload: Option<&str>) -> Result<()> {
    if let Some(data) = payload {
        let mut bytes: Vec<u8> = data.as_bytes().to_vec();
        if bytes.last().copied() != Some(b'\n') {
            bytes.push(b'\n');
        }

        if let Err(error) = stdin.write_all(bytes.as_slice()).await {
            error!("send_interactive_input(): failed to write payload (error={error})");
            return Err(error.into());
        }

        if let Err(error) = stdin.flush().await {
            error!("send_interactive_input(): failed to flush stdin (error={error})");
            return Err(error.into());
        }
    }

    if let Err(error) = stdin.shutdown().await {
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
    handle: JoinHandle<::std::io::Result<()>>,
    buffer: Arc<AsyncMutex<Vec<u8>>>,
    timeout: Duration,
    label: &str,
    iteration: usize,
) -> Result<Vec<u8>> {
    let join_result = match ::tokio::time::timeout(timeout, handle).await {
        Ok(result) => result,
        Err(_elapsed) => {
            let reason: String = format!(
                "timed out while waiting for interactive {label} (iteration={}, timeout_ms={})",
                iteration,
                timeout.as_millis()
            );
            error!("test_with_terminal_executor(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        },
    };

    match join_result {
        Err(join_error) => {
            let reason: String = format!(
                "failed to join {label} collector task (iteration={}, error={join_error})",
                iteration
            );
            error!("wait_stream_collector(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        },
        Ok(Err(io_error)) => {
            let reason: String = format!(
                "failed to read {label} stream from nanvixd (iteration={}, error={io_error})",
                iteration
            );
            error!("wait_stream_collector(): {reason}");
            return Err(::anyhow::anyhow!(reason));
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
