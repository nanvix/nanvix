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
use ::shell_words;
use ::std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
    time::Duration,
};
use ::tokio::{
    task::JoinHandle,
    time::sleep,
};
use ::tokio_util::sync::CancellationToken;

//==================================================================================================
// Constants
//==================================================================================================

/// Brief pause (milliseconds) between iterations to let nanvixd resources drain.
const CLEANUP_SLEEP_DURATION_MS: u64 = 200;
/// Path (relative to nanvixd's working directory) at which the VMM stores snapshot artifacts.
const SNAPSHOTS_DIR: &str = "snapshots";
/// Subdirectory under `working_directory` that holds built guest binaries.
const BIN_DIR: &str = "bin";
/// Filename of the kernel ELF used as the snapshot restore source.
const KERNEL_ELF_FILENAME: &str = "kernel.elf";
/// Flag passed to the guest workload to trigger snapshot capture during phase 1.
const SNAPSHOT_WORKLOAD_FLAG: &str = "--snapshot";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Drives a deterministic snapshot save / restore lifecycle for a Nanvix user workload by
/// spawning the Nanvix Daemon as a subprocess. Each iteration:
///   1. Spawns `nanvixd` with `-kernel-args <SNAPSHOT_TOKEN>` and the workload invoked with
///      `--snapshot`, causing the kernel to take a snapshot mid-execution and exit cleanly.
///   2. Spawns a fresh `nanvixd` with `-snapshot <kernel-elf>`, restoring from the snapshot
///      artifact written by the previous phase and running the post-snapshot workload to
///      completion.
///
/// The test fails when either phase exits with a code that does not match
/// `workload.expected_exit_code()`, when the daemon process fails to spawn, or when an
/// internal I/O error occurs while draining the daemon's stdio. This is the regression
/// coverage for issue #2434 (warm-restore guest panic): under the broken vCPU / IRQ chip
/// restore ordering, phase 2 panics on resume rather than completing the workload.
///
/// The executor does not currently support workloads that configure `input`,
/// `expected_output`, or `expect_empty_output`; such workloads are rejected up-front to
/// avoid silently ignoring the configuration.
///
/// # Parameters
///
/// - `runner_config`: Runner configuration; `working_directory` is used to locate the kernel ELF.
/// - `iterations`: Number of save / restore cycles to perform.
/// - `workload`: Workload specification; `program_path` is the user binary to snapshot and the
///   expected exit code is asserted against both phases.
/// - `log_layout`: Layout that defines the target directory for component logs.
/// - `extra_nanvixd_args`: Additional command-line arguments forwarded to `nanvixd`.
/// - `cancellation_token`: Token to abort test execution on shutdown.
///
/// # Return Value
///
/// Returns `Ok(())` after all iterations succeed; returns an error on the first failure.
///
pub async fn test_with_snapshot_restore_executor(
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
            error!("test_with_snapshot_restore_executor(): cancellation requested");
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
    // Reject configurations whose expectations this executor cannot honour. Both phases
    // discard stdout/stderr and close stdin, so silently accepting these fields would let
    // misconfigured tests pass for the wrong reason.
    if workload.input().is_some() {
        let reason: String = "snapshot-restore executor does not support workloads with 'input' \
                              configured"
            .to_string();
        error!("run_iterations(): {reason}");
        return Err(anyhow!(reason));
    }
    if workload.expected_output().is_some() || workload.expect_empty_output() {
        let reason: String = "snapshot-restore executor does not support workloads with \
                              'expected_output' or 'expect_empty_output' configured"
            .to_string();
        error!("run_iterations(): {reason}");
        return Err(anyhow!(reason));
    }

    let working_directory: PathBuf = PathBuf::from(runner_config.working_directory.as_str());
    let kernel_filename: PathBuf = working_directory.join(BIN_DIR).join(KERNEL_ELF_FILENAME);

    let snapshot_program: String = workload.program_path().to_string();
    if !Path::new(snapshot_program.as_str()).exists() {
        let reason: String = format!("snapshot workload not found (path={snapshot_program})");
        error!("run_iterations(): {reason}");
        return Err(anyhow!(reason));
    }

    let expected_exit_code: i32 = workload.expected_exit_code();

    // The VMM writes snapshot artifacts relative to nanvixd's CWD, which the spawn helper sets
    // to `runner_config.working_directory`. Mirror that here so the directory we pre-create
    // matches where the daemon will actually look.
    let snapshots_dir: PathBuf = working_directory.join(SNAPSHOTS_DIR);
    match fs::metadata(&snapshots_dir) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                let reason: String = format!(
                    "snapshots path exists but is not a directory (path={})",
                    snapshots_dir.display()
                );
                error!("run_iterations(): {reason}");
                return Err(anyhow!(reason));
            }
        },
        Err(error) if error.kind() == ::std::io::ErrorKind::NotFound => {
            fs::create_dir_all(&snapshots_dir).map_err(|error| {
                let reason: String = format!(
                    "failed to create snapshots directory (path={}, error={error})",
                    snapshots_dir.display()
                );
                error!("run_iterations(): {reason}");
                anyhow!(reason)
            })?;
        },
        Err(error) => {
            let reason: String = format!(
                "failed to stat snapshots directory (path={}, error={error})",
                snapshots_dir.display()
            );
            error!("run_iterations(): {reason}");
            return Err(anyhow!(reason));
        },
    }

    // Parse and re-encode the workload's program_args/program_env into the single
    // `<args>;<env>` string expected by nanvixd interactive mode (matches the terminal
    // executor's behaviour). Phase 1 additionally appends `--snapshot`.
    let parsed_program_args: Vec<String> = match workload.program_args() {
        Some(args) => shell_words::split(args).map_err(|error| {
            let reason: String = format!(
                "failed to parse snapshot-restore program_args (args='{args}', error={error})"
            );
            error!("run_iterations(): {reason}");
            anyhow!(reason)
        })?,
        None => Vec::new(),
    };
    let save_combined_args: Vec<String> =
        build_combined_program_args(&parsed_program_args, true, workload.program_env());
    let restore_combined_args: Vec<String> =
        build_combined_program_args(&parsed_program_args, false, workload.program_env());

    let hwloc_file_path: Option<String> = runner_config.hwloc_file_path.clone();
    let log_directory: PathBuf = log_layout.test_directory().to_path_buf();
    // Capture pre-existing legacy guest-component logs so they are not later overwritten by
    // the multiple nanvixd spawns this executor performs. The tracker is then used to move
    // newly produced logs into the per-phase test directory between phases.
    let log_root: &Path = Path::new(runner_config.log_directory.as_str());
    let guest_log_tracker: GuestLogTracker = GuestLogTracker::capture(log_root)?;

    for iteration in 0..iterations {
        debug!(
            "run_iterations(): starting iteration {iteration} of {iterations} \
             (program={snapshot_program})"
        );

        // Phase 1: cold-boot the guest with `--snapshot` to produce a snapshot artifact.
        let save_extra: Vec<String> = build_extra_args(
            extra_nanvixd_args,
            &[
                "-kernel-args".to_string(),
                ::koptions::SNAPSHOT_TOKEN.to_string(),
            ],
        );
        let save_exit: i32 = spawn_nanvixd(
            runner_config,
            hwloc_file_path.as_deref().map(str::to_string),
            snapshot_program.as_str(),
            save_combined_args.as_slice(),
            log_directory.as_path(),
            save_extra.as_slice(),
        )
        .await
        .map_err(|error| {
            error!(
                "run_iterations(): snapshot save phase failed (iteration={iteration}): {error:?}"
            );
            error
        })?;
        // Migrate the save-phase guest component logs into the test directory and normalize
        // their filenames with a phase-specific iteration token so the restore phase does not
        // overwrite them. The phase token is `iteration * 2` for save.
        guest_log_tracker.move_new_logs(log_layout.test_directory())?;
        log_layout.normalize_component_logs(iteration.saturating_mul(2))?;
        // The save phase exits as soon as the VMM persists the snapshot artifacts (status 0):
        // the guest never resumes past `pm::snapshot()` in this phase, so the workload's
        // configured `expected_exit_code` only applies to the restore phase below.
        const SAVE_PHASE_EXPECTED_EXIT_CODE: i32 = 0;
        if save_exit != SAVE_PHASE_EXPECTED_EXIT_CODE {
            let reason: String = format!(
                "snapshot save phase exited with status {save_exit}, expected \
                 {SAVE_PHASE_EXPECTED_EXIT_CODE} (iteration={iteration})"
            );
            error!("run_iterations(): {reason}");
            return Err(anyhow!(reason));
        }

        sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION_MS)).await;

        // Phase 2: restore the snapshot and run the post-snapshot workload to completion. This is
        // the path that regressed under issue #2434.
        let restore_extra: Vec<String> = build_extra_args(
            extra_nanvixd_args,
            &[
                "-snapshot".to_string(),
                kernel_filename.to_string_lossy().into_owned(),
            ],
        );
        let restore_exit: i32 = spawn_nanvixd(
            runner_config,
            hwloc_file_path.as_deref().map(str::to_string),
            snapshot_program.as_str(),
            restore_combined_args.as_slice(),
            log_directory.as_path(),
            restore_extra.as_slice(),
        )
        .await
        .map_err(|error| {
            error!(
                "run_iterations(): snapshot restore phase failed (iteration={iteration}): \
                 {error:?}"
            );
            error
        })?;
        // Migrate restore-phase logs with a distinct token (`iteration * 2 + 1`).
        guest_log_tracker.move_new_logs(log_layout.test_directory())?;
        log_layout.normalize_component_logs(iteration.saturating_mul(2).saturating_add(1))?;
        if restore_exit != expected_exit_code {
            let reason: String = format!(
                "snapshot restore phase exited with status {restore_exit}, expected \
                 {expected_exit_code} (iteration={iteration})"
            );
            error!("run_iterations(): {reason}");
            return Err(anyhow!(reason));
        }

        sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION_MS)).await;
    }

    Ok(())
}

/// Builds the `program_args` slice forwarded to nanvixd interactive mode, encoding the
/// workload's argv (optionally with `--snapshot` appended for phase 1) and environment into
/// the single `<args>;<env>` string consumed by the daemon.
fn build_combined_program_args(
    parsed_program_args: &[String],
    append_snapshot_flag: bool,
    program_env: Option<&str>,
) -> Vec<String> {
    let mut argv: Vec<String> = parsed_program_args.to_vec();
    if append_snapshot_flag {
        argv.push(SNAPSHOT_WORKLOAD_FLAG.to_string());
    }
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

///
/// # Description
///
/// Concatenates the caller-provided `extra_nanvixd_args` with executor-specific arguments,
/// preserving the order: caller arguments first, then phase-specific overrides.
///
fn build_extra_args(base: &[String], phase: &[String]) -> Vec<String> {
    let mut combined: Vec<String> = Vec::with_capacity(base.len() + phase.len());
    combined.extend_from_slice(base);
    combined.extend_from_slice(phase);
    combined
}

///
/// # Description
///
/// Spawns a single Nanvix Daemon subprocess, drains its stdout/stderr pipes, waits for exit, and
/// returns the daemon's exit code.
///
/// # Parameters
///
/// - `runner_config`: Runner configuration used to locate the daemon binary and runtime layout.
/// - `hwloc_file_path`: Optional hwloc topology file forwarded to the daemon.
/// - `program_path`: Guest workload binary executed inside the sandbox.
/// - `program_args`: Command-line arguments forwarded to the workload.
/// - `log_directory`: Directory where daemon components emit logs.
/// - `extra_nanvixd_args`: Additional command-line arguments forwarded to the daemon.
///
/// # Return Value
///
/// Returns the daemon process exit code on success; returns an error when the daemon cannot be
/// spawned or waited upon.
///
async fn spawn_nanvixd(
    runner_config: &RunnerConfig,
    hwloc_file_path: Option<String>,
    program_path: &str,
    program_args: &[String],
    log_directory: &Path,
    extra_nanvixd_args: &[String],
) -> Result<i32> {
    let nanvixd_args: NanvixdTerminalArgs = NanvixdTerminalArgs::new(
        hwloc_file_path,
        program_path,
        program_args,
        log_directory,
        extra_nanvixd_args,
    )?;

    let mut nanvixd: NanvixdTerminal = NanvixdTerminal::spawn(runner_config, &nanvixd_args).await?;

    // Drop stdin: the snapshot-restore workload does not consume input. Dropping the handle
    // closes the pipe and signals EOF to the daemon if it ever reads.
    drop(nanvixd.take_stdin());

    // Drain stdout and stderr concurrently so that the daemon does not block on a full pipe
    // buffer. The captured bytes are discarded; failures are reported as test errors.
    let stdout_drain: Option<JoinHandle<::std::io::Result<()>>> = nanvixd
        .take_stdout()
        .map(|pipe| ::tokio::spawn(drain_stream(pipe)));
    let stderr_drain: Option<JoinHandle<::std::io::Result<()>>> = nanvixd
        .take_stderr()
        .map(|pipe| ::tokio::spawn(drain_stream(pipe)));

    let exit_code: i32 = nanvixd.wait_exit_code().await?;

    if let Some(handle) = stdout_drain {
        match handle.await {
            Ok(Ok(())) => {},
            Ok(Err(error)) => {
                let reason: String = format!("error draining nanvixd stdout: {error}");
                error!("spawn_nanvixd(): {reason}");
                return Err(anyhow!(reason));
            },
            Err(error) => {
                let reason: String = format!("nanvixd stdout drain task join failed: {error:?}");
                error!("spawn_nanvixd(): {reason}");
                return Err(anyhow!(reason));
            },
        }
    }
    if let Some(handle) = stderr_drain {
        match handle.await {
            Ok(Ok(())) => {},
            Ok(Err(error)) => {
                let reason: String = format!("error draining nanvixd stderr: {error}");
                error!("spawn_nanvixd(): {reason}");
                return Err(anyhow!(reason));
            },
            Err(error) => {
                let reason: String = format!("nanvixd stderr drain task join failed: {error:?}");
                error!("spawn_nanvixd(): {reason}");
                return Err(anyhow!(reason));
            },
        }
    }

    Ok(exit_code)
}
