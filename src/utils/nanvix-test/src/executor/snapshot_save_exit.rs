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
        drain_stream,
    },
    log_layout::{
        GuestLogTracker,
        TestLogLayout,
    },
    nanvixd::{
        NanvixdTerminal,
        NanvixdTerminalArgs,
    },
};
use ::anyhow::{
    Result,
    anyhow,
};
use ::log::{
    debug,
    error,
};
use ::std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
    pin::Pin,
    time::Duration,
};
use ::tokio::{
    task::JoinHandle,
    time::{
        error::Elapsed,
        timeout,
    },
};
use ::tokio_util::sync::CancellationToken;

//==================================================================================================
// Constants
//==================================================================================================

/// Path (relative to nanvixd's working directory) at which the VMM stores snapshot artifacts.
const SNAPSHOTS_DIR: &str = "snapshots";
/// Filename of the snapshot virtual-memory dump produced by the KVM backend.
const SNAPSHOT_VMEM_FILENAME: &str = "kernel.vmem";
/// Filename of the snapshot KVM metadata produced by the KVM backend.
const SNAPSHOT_KVM_JSON_FILENAME: &str = "kernel.kvm.json";
/// Flags appended to the workload's command line during phase 1.
const SNAPSHOT_FLAG: &str = "--snapshot";
const NO_EXIT_FLAG: &str = "--no-exit";
/// Upper bound on how long the guest may take to request the snapshot and the VMM to write the
/// snapshot artifacts to disk.
///
/// The VMM writes the whole guest memory and flushes it to disk, which usually takes about a
/// second, but may take minutes on hosts with a degraded disk. This limit is far larger, so that a
/// slow disk is not mistaken for a hang, while still catching a guest or VMM that never saves the
/// snapshot.
const SNAPSHOT_SAVE_TIMEOUT: Duration = Duration::from_secs(300);
/// Delay between checks for the snapshot artifacts.
const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_millis(100);
/// Upper bound on how long the host nanvixd process may take to exit after the snapshot
/// artifacts are flushed to disk.
const NANVIXD_EXIT_TIMEOUT: Duration = Duration::from_secs(60);

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Drives a single snapshot-save cycle and asserts that the host `nanvixd` process exits on
/// its own within `NANVIXD_EXIT_TIMEOUT` after the snapshot artifacts
/// (`snapshots/kernel.vmem`, `snapshots/kernel.kvm.json`) are written to disk.
///
/// To isolate the daemon-exit behaviour from the workload-exit behaviour, the guest workload
/// is invoked with both `--snapshot` and `--no-exit`. The workload calls `pm::snapshot()`
/// and then spins forever, so any clean shutdown observed by this executor must originate
/// from the host-side snapshot-completion path.
///
/// The executor fails when:
///   * the snapshot artifacts are not written within `SNAPSHOT_SAVE_TIMEOUT`,
///   * `nanvixd` does not exit within `NANVIXD_EXIT_TIMEOUT` after the snapshot artifacts are
///     written,
///   * `nanvixd` exits with a non-`expected_exit_code` status, or
///   * the snapshot artifacts are missing after `nanvixd` exits.
///
/// The executor does not support workloads that configure `input`, `expected_output`, or
/// `expect_empty_output`; the daemon's stdio is drained but not inspected.
///
pub async fn test_with_snapshot_save_exit_executor(
    runner_config: &RunnerConfig,
    iterations: usize,
    workload: WorkloadSpec<'_>,
    log_layout: &TestLogLayout,
    extra_nanvixd_args: &[String],
    cancellation_token: CancellationToken,
) -> Result<()> {
    tokio::select! {
        result = run_iterations(
            runner_config,
            iterations,
            workload,
            log_layout,
            extra_nanvixd_args,
        ) => result,
        _ = cancellation_token.cancelled() => {
            error!("test_with_snapshot_save_exit_executor(): cancellation requested");
            Err(anyhow!("cancelled"))
        },
    }
}

async fn run_iterations(
    runner_config: &RunnerConfig,
    iterations: usize,
    workload: WorkloadSpec<'_>,
    log_layout: &TestLogLayout,
    extra_nanvixd_args: &[String],
) -> Result<()> {
    if workload.input().is_some()
        || workload.expected_output().is_some()
        || workload.expect_empty_output()
    {
        let reason: String = "snapshot-save-exit executor does not support workloads with \
                              'input', 'expected_output', or 'expect_empty_output' configured"
            .to_string();
        error!("run_iterations(): {reason}");
        return Err(anyhow!(reason));
    }

    let working_directory: PathBuf = PathBuf::from(runner_config.working_directory.as_str());
    let snapshots_dir: PathBuf = working_directory.join(SNAPSHOTS_DIR);
    let snapshot_program: String = workload.program_path().to_string();
    if !Path::new(snapshot_program.as_str()).exists() {
        let reason: String = format!("snapshot workload not found (path={snapshot_program})");
        error!("run_iterations(): {reason}");
        return Err(anyhow!(reason));
    }

    let expected_exit_code: i32 = workload.expected_exit_code();

    // Combine workload argv with the two test-harness flags and re-encode for nanvixd's
    // interactive mode (matches the snapshot-restore executor's behaviour).
    let parsed_program_args: Vec<String> = match workload.program_args() {
        Some(args) => ::shell_words::split(args).map_err(|error| {
            let reason: String = format!(
                "failed to parse snapshot-save-exit program_args (args='{args}', error={error})"
            );
            error!("run_iterations(): {reason}");
            anyhow!(reason)
        })?,
        None => Vec::new(),
    };
    let combined_args: Vec<String> =
        build_combined_program_args(&parsed_program_args, workload.program_env());

    let extra_nanvixd_args_with_kargs: Vec<String> = {
        let mut combined: Vec<String> = Vec::with_capacity(extra_nanvixd_args.len() + 2);
        combined.extend_from_slice(extra_nanvixd_args);
        combined.push("-kernel-args".to_string());
        combined.push(::koptions::SNAPSHOT_TOKEN.to_string());
        combined
    };

    let log_directory: PathBuf = log_layout.test_directory().to_path_buf();
    let log_root: &Path = Path::new(runner_config.log_directory.as_str());
    let guest_log_tracker: GuestLogTracker = GuestLogTracker::capture(log_root)?;

    for iteration in 0..iterations {
        debug!(
            "run_iterations(): starting iteration {iteration} of {iterations} \
             (program={snapshot_program})"
        );

        // Pre-create the snapshots directory and remove any previous artifacts so the
        // post-exit existence checks are unambiguous.
        prepare_snapshots_dir(&snapshots_dir)?;

        let nanvixd_args: NanvixdTerminalArgs = NanvixdTerminalArgs::new(
            snapshot_program.as_str(),
            combined_args.as_slice(),
            log_directory.as_path(),
            extra_nanvixd_args_with_kargs.as_slice(),
        )?;

        let mut nanvixd: NanvixdTerminal =
            NanvixdTerminal::spawn(runner_config, &nanvixd_args).await?;

        // Close stdin and drain stdio so the daemon never blocks on a full pipe buffer.
        drop(nanvixd.take_stdin());
        let stdout_drain: Option<JoinHandle<::std::io::Result<()>>> = nanvixd
            .take_stdout()
            .map(|pipe| ::tokio::spawn(drain_stream(pipe)));
        let stderr_drain: Option<JoinHandle<::std::io::Result<()>>> = nanvixd
            .take_stderr()
            .map(|pipe| ::tokio::spawn(drain_stream(pipe)));

        let outcome: SaveExitOutcome = wait_save_and_exit(
            nanvixd.wait_exit_code(),
            &snapshots_dir,
            SNAPSHOT_POLL_INTERVAL,
            SNAPSHOT_SAVE_TIMEOUT,
            NANVIXD_EXIT_TIMEOUT,
        )
        .await;

        // Migrate guest component logs regardless of outcome so failures keep their breadcrumbs.
        guest_log_tracker.move_new_logs(log_layout.test_directory())?;
        log_layout.normalize_component_logs(iteration)?;

        match outcome {
            SaveExitOutcome::SaveTimedOut(artifacts) => {
                stop_nanvixd(nanvixd, stdout_drain, stderr_drain).await;
                let reason: String = format!(
                    "snapshot save did not complete within {timeout_secs}s \
                     (iteration={iteration}, {artifacts})",
                    timeout_secs = SNAPSHOT_SAVE_TIMEOUT.as_secs()
                );
                error!("run_iterations(): {reason}");
                return Err(anyhow!(reason));
            },
            SaveExitOutcome::ExitTimedOut => {
                stop_nanvixd(nanvixd, stdout_drain, stderr_drain).await;
                let reason: String = format!(
                    "nanvixd did not exit within {timeout_secs}s after the snapshot save \
                     completed (iteration={iteration})",
                    timeout_secs = NANVIXD_EXIT_TIMEOUT.as_secs()
                );
                error!("run_iterations(): {reason}");
                return Err(anyhow!(reason));
            },
            SaveExitOutcome::Exited(Err(error)) => {
                error!(
                    "run_iterations(): waiting for nanvixd exit failed (iteration={iteration}, \
                     error={error:?})"
                );
                return Err(error);
            },
            SaveExitOutcome::Exited(Ok(exit_code)) => {
                await_drain(stdout_drain, DrainStream::Stdout).await?;
                await_drain(stderr_drain, DrainStream::Stderr).await?;
                if exit_code != expected_exit_code {
                    let reason: String = format!(
                        "snapshot save phase exited with status {exit_code}, expected \
                         {expected_exit_code} (iteration={iteration})"
                    );
                    error!("run_iterations(): {reason}");
                    return Err(anyhow!(reason));
                }
                if !snapshot_artifacts_present(&snapshots_dir) {
                    let reason: String = format!(
                        "nanvixd exited cleanly but snapshot artifacts are missing under {} \
                         (iteration={iteration})",
                        snapshots_dir.display()
                    );
                    error!("run_iterations(): {reason}");
                    return Err(anyhow!(reason));
                }
            },
        }
    }

    Ok(())
}

/// Outcome of waiting for `nanvixd` to save the snapshot and exit.
#[derive(Debug)]
enum SaveExitOutcome {
    /// `nanvixd` exited, with the result of waiting for its exit code.
    Exited(Result<i32>),
    /// The snapshot artifacts were not written in time, with their state at the deadline.
    SaveTimedOut(String),
    /// `nanvixd` did not exit in time after the snapshot artifacts were written.
    ExitTimedOut,
}

///
/// # Description
///
/// Waits for `nanvixd` to write the snapshot artifacts, and then to exit.
///
/// The exit deadline starts only once the snapshot artifacts are written, so the time that the
/// VMM takes to write the snapshot to disk, which depends on the speed of the host disk, is not
/// mistaken for a hang on exit. If `nanvixd` exits before the snapshot artifacts are written, its
/// exit is reported right away.
///
/// # Parameters
///
/// - `exit`: Future that resolves to the exit code of `nanvixd` once it exits.
/// - `snapshots_dir`: Directory where the VMM writes the snapshot artifacts.
/// - `poll_interval`: Delay between checks for the snapshot artifacts.
/// - `save_timeout`: Maximum time to wait for the snapshot artifacts to be written.
/// - `exit_timeout`: Maximum time to wait for `nanvixd` to exit after the snapshot artifacts are
///   written.
///
/// # Return Value
///
/// Returns the outcome of the wait.
///
async fn wait_save_and_exit<F>(
    exit: F,
    snapshots_dir: &Path,
    poll_interval: Duration,
    save_timeout: Duration,
    exit_timeout: Duration,
) -> SaveExitOutcome
where
    F: Future<Output = Result<i32>>,
{
    let mut exit: Pin<&mut F> = ::std::pin::pin!(exit);

    let save_result: Result<Option<Result<i32>>, Elapsed> = timeout(save_timeout, async {
        loop {
            if snapshot_artifacts_present(snapshots_dir) {
                return None;
            }
            if let Ok(exit_result) = timeout(poll_interval, &mut exit).await {
                return Some(exit_result);
            }
        }
    })
    .await;

    match save_result {
        Err(_elapsed) => SaveExitOutcome::SaveTimedOut(describe_snapshot_artifacts(snapshots_dir)),
        Ok(Some(exit_result)) => SaveExitOutcome::Exited(exit_result),
        Ok(None) => match timeout(exit_timeout, exit).await {
            Ok(exit_result) => SaveExitOutcome::Exited(exit_result),
            Err(_elapsed) => SaveExitOutcome::ExitTimedOut,
        },
    }
}

/// Forces `nanvixd` down and aborts the stdio drain tasks, so that neither blocks an error path.
async fn stop_nanvixd(
    nanvixd: NanvixdTerminal,
    stdout_drain: Option<JoinHandle<::std::io::Result<()>>>,
    stderr_drain: Option<JoinHandle<::std::io::Result<()>>>,
) {
    drop(nanvixd);
    for handle in [stdout_drain, stderr_drain].into_iter().flatten() {
        handle.abort();
        let _ = handle.await;
    }
}

/// Encodes the workload argv (with the two harness flags appended) and the workload env into
/// the single `<args>;<env>` string consumed by nanvixd's interactive mode.
fn build_combined_program_args(
    parsed_program_args: &[String],
    program_env: Option<&str>,
) -> Vec<String> {
    let mut argv: Vec<String> = parsed_program_args.to_vec();
    argv.push(SNAPSHOT_FLAG.to_string());
    argv.push(NO_EXIT_FLAG.to_string());
    let joined_args: String = argv.join(" ");
    let combined: String = combine_args_env(
        if joined_args.is_empty() {
            None
        } else {
            Some(joined_args.as_str())
        },
        program_env,
    );
    if combined.is_empty() {
        Vec::new()
    } else {
        vec![combined]
    }
}

fn prepare_snapshots_dir(snapshots_dir: &Path) -> Result<()> {
    match fs::metadata(snapshots_dir) {
        Ok(metadata) if !metadata.is_dir() => {
            let reason: String = format!(
                "snapshots path exists but is not a directory (path={})",
                snapshots_dir.display()
            );
            error!("prepare_snapshots_dir(): {reason}");
            return Err(anyhow!(reason));
        },
        Ok(_) => {
            for filename in [SNAPSHOT_VMEM_FILENAME, SNAPSHOT_KVM_JSON_FILENAME] {
                let path: PathBuf = snapshots_dir.join(filename);
                if path.exists() {
                    fs::remove_file(&path).map_err(|error| {
                        let reason: String = format!(
                            "failed to remove stale snapshot artifact (path={}, error={error})",
                            path.display()
                        );
                        error!("prepare_snapshots_dir(): {reason}");
                        anyhow!(reason)
                    })?;
                }
            }
        },
        Err(error) if error.kind() == ::std::io::ErrorKind::NotFound => {
            fs::create_dir_all(snapshots_dir).map_err(|error| {
                let reason: String = format!(
                    "failed to create snapshots directory (path={}, error={error})",
                    snapshots_dir.display()
                );
                error!("prepare_snapshots_dir(): {reason}");
                anyhow!(reason)
            })?;
        },
        Err(error) => {
            let reason: String = format!(
                "failed to stat snapshots directory (path={}, error={error})",
                snapshots_dir.display()
            );
            error!("prepare_snapshots_dir(): {reason}");
            return Err(anyhow!(reason));
        },
    }
    Ok(())
}

fn snapshot_artifacts_present(snapshots_dir: &Path) -> bool {
    snapshots_dir.join(SNAPSHOT_VMEM_FILENAME).is_file()
        && snapshots_dir.join(SNAPSHOT_KVM_JSON_FILENAME).is_file()
}

/// Describes the size of each snapshot artifact for diagnostics, for example
/// `kernel.vmem=268439552 bytes, kernel.kvm.json=missing`.
fn describe_snapshot_artifacts(snapshots_dir: &Path) -> String {
    let descriptions: Vec<String> = [SNAPSHOT_VMEM_FILENAME, SNAPSHOT_KVM_JSON_FILENAME]
        .iter()
        .map(|filename| match fs::metadata(snapshots_dir.join(filename)) {
            Ok(metadata) => format!("{filename}={} bytes", metadata.len()),
            Err(error) if error.kind() == ::std::io::ErrorKind::NotFound => {
                format!("{filename}=missing")
            },
            Err(error) => format!("{filename}=unknown ({error})"),
        })
        .collect();
    descriptions.join(", ")
}

/// Identifies which nanvixd stdio pipe a drain task is associated with, used for diagnostics.
#[derive(Clone, Copy)]
enum DrainStream {
    Stdout,
    Stderr,
}

impl DrainStream {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }
}

/// Awaits a stdio drain task and surfaces both join failures and I/O errors as test failures.
async fn await_drain(
    handle: Option<JoinHandle<::std::io::Result<()>>>,
    stream: DrainStream,
) -> Result<()> {
    let Some(handle) = handle else {
        return Ok(());
    };
    let label: &str = stream.as_str();
    match handle.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => {
            let reason: String = format!("error draining nanvixd {label}: {error}");
            error!("await_drain(): {reason}");
            Err(anyhow!(reason))
        },
        Err(error) => {
            let reason: String = format!("nanvixd {label} drain task join failed: {error:?}");
            error!("await_drain(): {reason}");
            Err(anyhow!(reason))
        },
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;
