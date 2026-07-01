// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![forbid(clippy::all)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::cast_sign_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod config;
mod environment;
mod executor;
mod log_layout;
mod nanvixd;
mod port;
mod uservm;
mod warning;

#[macro_export]
macro_rules! warn_with_policy {
    ($($arg:tt)+) => {{
        let formatted_message: String = format!($($arg)+);
        ::log::warn!("{}", formatted_message);
        $crate::warning::record_warning(formatted_message);
    }};
}

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::{
        NanvixTestConfig,
        TestCaseConfig,
    },
    environment::prepare_runner_environment,
    executor::{
        ExecutorName,
        WorkloadSpec,
        empty::empty,
        http::test_with_http_executor,
        snapshot_restore::test_with_snapshot_restore_executor,
        snapshot_save_exit::test_with_snapshot_save_exit_executor,
        terminal::test_with_terminal_executor,
    },
    log_layout::{
        TestLogLayout,
        initialize_run_timestamp,
    },
};
use ::anyhow::Result;
use ::globset::GlobSet;
use ::log::{
    debug,
    error,
    info,
};
use ::std::{
    fs::create_dir_all,
    path::Path,
    process,
    sync::{
        Arc,
        atomic::{
            AtomicI32,
            Ordering,
        },
    },
};
#[cfg(unix)]
use ::tokio::signal::unix::{
    SignalKind,
    signal,
};
use ::tokio_util::sync::CancellationToken;

//==================================================================================================
// Constants
//==================================================================================================

/// Base return code for test interrupts; actual exit code will be 128 + signal number.
const BASE_RETURN_CODE: i32 = 128;
/// Default log-level (overridden by RUST_LOG environment variable if set).
const DEFAULT_LOG_LEVEL: &str = "error";
/// Default tenant identifier used when creating test sandboxes.
pub(crate) const DEFAULT_TENANT_ID: &str = "nanvix-test";
/// Signal number for a Windows Ctrl+C event. Windows has no `SIGINT`/`SIGTERM`
/// distinction, so we map Ctrl+C to the POSIX `SIGINT` value (2) to keep the `128 + signum`
/// exit-code convention.
#[cfg(not(unix))]
const WINDOWS_CTRL_C_SIGNUM: i32 = 2;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes logging, builds a Tokio runtime, and dispatches the asynchronous test harness.
///
/// # Return Value
///
/// Returns the status produced by `run()`.
///
fn main() -> Result<()> {
    ::nanvix::log::init(false, DEFAULT_LOG_LEVEL, String::new(), None);

    let runtime: ::tokio::runtime::Runtime = ::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            let reason: String = format!("failed to build tokio runtime (error={error})");
            error!("main(): {reason}");
            ::anyhow::anyhow!(reason)
        })?;

    // Shared state: signal number received (0 = none).
    let received_signal: Arc<AtomicI32> = Arc::new(AtomicI32::new(0));
    let cancellation_token: CancellationToken = CancellationToken::new();
    let force_exit_token: CancellationToken = CancellationToken::new();

    // Spawn a background task that listens for interrupt signals.
    // First signal: cancel the token so in-flight work drains gracefully.
    // Second signal: cancel the force-exit token so that main() can shut down the runtime.
    #[cfg(unix)]
    {
        let sig_flag: Arc<AtomicI32> = Arc::clone(&received_signal);
        let sig_token: CancellationToken = cancellation_token.clone();
        let sig_force_token: CancellationToken = force_exit_token.clone();
        runtime.spawn(async move {
            let mut sigint = match signal(SignalKind::interrupt()) {
                Ok(stream) => stream,
                Err(error) => {
                    error!("signal_listener(): failed to register SIGINT handler (error={error})");
                    return;
                },
            };
            let mut sigterm = match signal(SignalKind::terminate()) {
                Ok(stream) => stream,
                Err(error) => {
                    error!("signal_listener(): failed to register SIGTERM handler (error={error})");
                    return;
                },
            };

            // Wait for the first signal.
            let (signum, signame): (i32, &str) = tokio::select! {
                _ = sigint.recv() => (libc::SIGINT, "SIGINT"),
                _ = sigterm.recv() => (libc::SIGTERM, "SIGTERM"),
            };
            info!("signal_listener(): received {signame}, requesting graceful shutdown...");
            sig_flag.store(signum, Ordering::SeqCst);
            sig_token.cancel();

            // Wait for a second signal. Record it and cancel the force-exit token.
            let (second_signum, second_signame): (i32, &str) = tokio::select! {
                _ = sigint.recv() => (libc::SIGINT, "SIGINT"),
                _ = sigterm.recv() => (libc::SIGTERM, "SIGTERM"),
            };
            error!(
                "signal_listener(): received second signal {second_signame}, forcing immediate \
                 exit"
            );
            sig_flag.store(second_signum, Ordering::SeqCst);
            sig_force_token.cancel();
        });
    }

    #[cfg(not(unix))]
    {
        let sig_flag: Arc<AtomicI32> = Arc::clone(&received_signal);
        let sig_token: CancellationToken = cancellation_token.clone();
        let sig_force_token: CancellationToken = force_exit_token.clone();
        runtime.spawn(async move {
            // First Ctrl+C: request graceful shutdown.
            if tokio::signal::ctrl_c().await.is_err() {
                error!("signal_listener(): failed to register Ctrl+C handler");
                return;
            }
            info!("signal_listener(): received Ctrl+C, requesting graceful shutdown...");
            sig_flag.store(WINDOWS_CTRL_C_SIGNUM, Ordering::SeqCst);
            sig_token.cancel();

            // Second Ctrl+C: force immediate exit.
            if tokio::signal::ctrl_c().await.is_err() {
                error!("signal_listener(): failed to register second Ctrl+C handler");
                return;
            }
            error!("signal_listener(): received second Ctrl+C, forcing immediate exit");
            sig_flag.store(WINDOWS_CTRL_C_SIGNUM, Ordering::SeqCst);
            sig_force_token.cancel();
        });
    }

    // Run the main test loop. Abort early if a second signal forces exit.
    let result: Result<()> = runtime.block_on(async {
        tokio::select! {
            result = run(cancellation_token) => result,
            _ = force_exit_token.cancelled() => {
                error!("main(): forcing exit on repeat signal...");
                Err(::anyhow::anyhow!("force exit"))
            },
        }
    });

    // If a signal was caught, override the exit code to 128+signum.
    let signum: i32 = received_signal.load(Ordering::SeqCst);
    if signum != 0 {
        drop(runtime);
        process::exit(BASE_RETURN_CODE.saturating_add(signum));
    }

    result
}

///
/// # Description
///
/// Parses CLI arguments, loads the configuration file, and runs the selected Nanvix Daemon
/// integration tests.
///
/// # Return Value
///
/// Returns `Ok(())` once all requested tests finish successfully; returns an error if CLI parsing,
/// configuration loading, log directory creation, or test execution fails.
///
async fn run(cancellation_token: CancellationToken) -> Result<()> {
    initialize_run_timestamp();

    let parsed_args: args::Args =
        args::Args::parse(::std::env::args().collect::<::std::vec::Vec<String>>())?;
    let NanvixTestConfig {
        runner: runner_config,
        tests,
    } = NanvixTestConfig::from_path(Path::new(parsed_args.config_file_path()))?;
    warning::configure(runner_config.fatal);

    // Select tests before preparing the environment to avoid unnecessary environment setup.
    let test_glob_filter: Option<GlobSet> = parsed_args.glob_filter();
    let total_tests: usize = tests.len();
    let filtered_tests: Vec<TestCaseConfig> = tests
        .into_iter()
        .filter(|test_config| test_config.matches_filter(test_glob_filter.as_ref()))
        .collect();
    let filtered_count: usize = filtered_tests.len();

    // Partition the filtered tests across shards when a shard selector is provided. Sharding is
    // positional (round-robin over the filtered, TOML-ordered list), so it is independent of test
    // names and automatically distributes newly added tests across shards.
    let selected_tests: Vec<TestCaseConfig> = match parsed_args.shard() {
        Some(shard) => filtered_tests
            .into_iter()
            .enumerate()
            .filter(|(position, _)| shard.selects(*position))
            .map(|(_, test_config)| test_config)
            .collect(),
        None => filtered_tests,
    };
    let selected_count: usize = selected_tests.len();

    // List mode: print selected tests and exit without executing.
    if parsed_args.list() {
        println!(
            "Tests in {} ({selected_count} of {total_tests} selected):\n",
            parsed_args.config_file_path()
        );

        // Compute column widths from the data.
        let name_width: usize = selected_tests
            .iter()
            .map(|t| t.name.len())
            .max()
            .unwrap_or(4)
            .max(4); // "Name" header length
        let exec_width: usize = selected_tests
            .iter()
            .map(|t| t.executor.len())
            .max()
            .unwrap_or(8)
            .max(8); // "Executor" header length

        // Header.
        println!("  {:<name_width$}  {:<exec_width$}", "Name", "Executor");
        println!("  {:-<name_width$}  {:-<exec_width$}", "", "");

        // Rows.
        for test_config in &selected_tests {
            println!("  {:<name_width$}  {:<exec_width$}", test_config.name, test_config.executor);
        }
        return Ok(());
    }

    // Fail early when no tests match the requested filter.
    if selected_tests.is_empty() {
        // An empty shard is not an error: when TOTAL exceeds the number of filtered tests, some
        // shards legitimately receive no work. Only treat an empty selection as success when the
        // filtered test list is non-empty (i.e. the filter matched tests but this shard received
        // none); an empty filtered list is still fatal, matching the non-sharded path.
        if let (Some(shard), true) = (parsed_args.shard(), filtered_count > 0) {
            info!(
                "main(): no tests assigned to shard {}/{} (filter={})",
                shard.index(),
                shard.total(),
                parsed_args
                    .test_filter()
                    .map(|f| format!("\"{}\"", f))
                    .unwrap_or("None".to_string())
            );
            return Ok(());
        }
        let reason: String = format!(
            "no tests selected (filter={})",
            parsed_args
                .test_filter()
                .map(|f| format!("\"{}\"", f))
                .unwrap_or("None".to_string())
        );
        error!("main(): {reason}");
        return Err(::anyhow::anyhow!(reason));
    }

    info!(
        "main(): selected {selected_count} of {total_tests} tests to run (filter={}, shard={})",
        parsed_args
            .test_filter()
            .map(|f| format!("\"{}\"", f))
            .unwrap_or("None".to_string()),
        parsed_args
            .shard()
            .map(|s| format!("{}/{}", s.index(), s.total()))
            .unwrap_or("None".to_string())
    );

    let log_directory: &str = runner_config.log_directory.as_str();
    if let Err(error) = create_dir_all(log_directory) {
        let reason: String =
            format!("failed to create nanvixd log directory (path={log_directory}, error={error})");
        error!("main(): {reason}");
        return Err(::anyhow::anyhow!(reason));
    }
    let log_root: &Path = Path::new(log_directory);

    tokio::select! {
        _ = prepare_runner_environment(
            Path::new(runner_config.tmp_directory.as_str()),
        ) => {},
        _ = cancellation_token.cancelled() => {
            error!("main(): cancelled during environment preparation");
            return Err(::anyhow::anyhow!("cancelled"));
        },
    }
    warning::fail_if_triggered("prepare_runner_environment")?;

    // Machine type used for filtering tests.
    let machine: &str = "microvm";

    // Build mode used for filtering tests. The Makefile exports the guest `BUILD_MODE` (`debug` or
    // `release`) into the environment. A missing or unrecognized value is a hard error rather than
    // a silent default, so a misconfigured run fails fast instead of quietly skipping every
    // build-mode-gated test.
    let build_mode: String = match ::std::env::var("BUILD_MODE") {
        Ok(value) => value.trim().to_string(),
        Err(_) => {
            return Err(::anyhow::anyhow!(
                "BUILD_MODE environment variable is required (expected one of: {})",
                TestCaseConfig::KNOWN_BUILD_MODES.join(", ")
            ));
        },
    };
    if !TestCaseConfig::KNOWN_BUILD_MODES.contains(&build_mode.as_str()) {
        return Err(::anyhow::anyhow!(
            "unknown BUILD_MODE '{build_mode}' (expected one of: {})",
            TestCaseConfig::KNOWN_BUILD_MODES.join(", ")
        ));
    }

    // Target architecture used for filtering tests. The Makefile exports the guest `TARGET` (`x86`
    // or `x86_64`); a missing or empty value defaults to `x86` so direct, single-arch invocations
    // keep working. Tests whose `targets` filter excludes this architecture are skipped, which lets
    // a config list suites that only build for one ABI (e.g. the i686-only dlfcn/setjmp suites)
    // without the other ABI's run failing on the images those suites never produced.
    let target: String = ::std::env::var("TARGET")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "x86".to_string());

    for test_config in selected_tests {
        // Skip tests that are not applicable to the current machine.
        if !test_config.should_run_on(machine) {
            debug!(
                "main(): skipping test not applicable to machine (executor={}, name={}, \
                 program={:?}, machine={}, runs_on={:?})",
                test_config.executor,
                test_config.name,
                test_config.program,
                machine,
                test_config.runs_on
            );
            continue;
        }

        // Skip tests that are not applicable to the current target architecture. This gates suites
        // that only build/run on a specific guest ABI (e.g. the i686-only dlfcn dynamic-linker and
        // setjmp suites) so an x86_64 run does not attempt to boot images that were never built.
        if !test_config.should_run_on_target(&target) {
            info!(
                "main(): skipping test not applicable to target (executor={}, name={}, \
                 program={:?}, target={}, targets={:?})",
                test_config.executor,
                test_config.name,
                test_config.program,
                target,
                test_config.targets
            );
            continue;
        }

        // Skip tests that are not applicable to the current build mode. This keeps heavy tests
        // (e.g. those that load a large image many times) off the debug/trace builds, where the
        // per-page kernel trace over a byte-at-a-time UART makes them too slow, while still running
        // them in release builds where tracing is disabled.
        if !test_config.should_run_in_build_mode(&build_mode) {
            info!(
                "main(): skipping test not applicable to build mode (executor={}, name={}, \
                 program={:?}, build_mode={}, build_modes={:?})",
                test_config.executor,
                test_config.name,
                test_config.program,
                build_mode,
                test_config.build_modes
            );
            continue;
        }

        // Check whether a signal arrived between test iterations.
        if cancellation_token.is_cancelled() {
            error!("main(): cancellation requested, skipping remaining tests");
            break;
        }

        let TestCaseConfig {
            executor,
            name,
            iterations,
            program,
            mut program_args,
            input,
            expected_output,
            expect_empty_output,
            extra_nanvixd_args,
            expected_exit_code,
            runs_on: _,
            targets: _,
            build_modes: _,
            program_env,
            program_args_padding_len,
        } = test_config;

        // When program_args_padding_len is set, generate a synthetic argument string.
        if let Some(padding_len) = program_args_padding_len {
            if padding_len > ::nanvix::config::system::MAX_CMDLINE_ARGS_LEN {
                let reason: String = format!(
                    "program_args_padding_len ({padding_len}) exceeds MAX_CMDLINE_ARGS_LEN ({})",
                    ::nanvix::config::system::MAX_CMDLINE_ARGS_LEN
                );
                error!("main(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            }
            program_args = Some("A".repeat(padding_len));
        }

        // Parse extra_nanvixd_args string into a vector of arguments.
        let extra_nanvixd_args: Vec<String> = match extra_nanvixd_args.as_ref() {
            Some(args) => match ::shell_words::split(args) {
                Ok(values) => values,
                Err(parse_error) => {
                    let reason: String = format!(
                        "failed to parse extra_nanvixd_args (args='{}', error={parse_error})",
                        args
                    );
                    error!("main(): {reason}");
                    return Err(::anyhow::anyhow!(reason));
                },
            },
            None => Vec::new(),
        };

        debug!(
            "main(): running test (executor={}, name={}, iterations={}, program={:?}, \
             program_args={:?}, program_env={:?}, expected_output={:?}, expect_empty_output={}, \
             expected_exit_code={:?}, extra_nanvixd_args={:?})",
            executor,
            name,
            iterations,
            program,
            program_args,
            program_env,
            expected_output,
            expect_empty_output,
            expected_exit_code,
            extra_nanvixd_args
        );

        let executor_name: ExecutorName = ExecutorName::from_str(executor.as_str())?;

        match (executor_name, &program) {
            (ExecutorName::Empty, _) => {
                let log_layout: TestLogLayout = TestLogLayout::for_label(
                    log_root,
                    ExecutorName::Empty.to_str(),
                    executor.as_str(),
                )?;
                empty(
                    &runner_config,
                    iterations,
                    &log_layout,
                    &extra_nanvixd_args,
                    cancellation_token.clone(),
                )
                .await?;
                let context: String = format!("empty executor completed (label={})", executor);
                warning::fail_if_triggered(context.as_str())?;
            },
            (ExecutorName::Http, Some(program_path)) => {
                if !Path::new(program_path.as_str()).exists() {
                    warn_with_policy!(
                        "main(): skipping tests with http executor because program path is \
                         missing (path={})",
                        program_path
                    );
                    warning::fail_if_triggered("http executor missing program")?;
                    continue;
                }

                let log_layout: TestLogLayout = TestLogLayout::for_program(
                    log_root,
                    ExecutorName::Http.to_str(),
                    program_path.as_str(),
                )?;

                let workload: WorkloadSpec = WorkloadSpec::new(
                    program_path.as_str(),
                    program_args.as_deref(),
                    program_env.as_deref(),
                    input.as_deref(),
                    expected_output.as_deref(),
                    expect_empty_output,
                    expected_exit_code,
                );

                test_with_http_executor(
                    &runner_config,
                    iterations,
                    workload,
                    &log_layout,
                    &extra_nanvixd_args,
                    cancellation_token.clone(),
                )
                .await?;
                let context: String = format!(
                    "http executor completed (program={}, test={})",
                    program_path, executor
                );
                warning::fail_if_triggered(context.as_str())?;
            },
            (ExecutorName::Http, None) => {
                let reason: String =
                    "tests entries with http executor must define the 'program' field".to_string();
                error!("main(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            (ExecutorName::Terminal, Some(program_path)) => {
                if !Path::new(program_path.as_str()).exists() {
                    warn_with_policy!(
                        "main(): skipping tests with terminal executor because program path is \
                         missing (path={})",
                        program_path
                    );
                    warning::fail_if_triggered("terminal executor missing program")?;
                    continue;
                }

                let log_layout: TestLogLayout = TestLogLayout::for_program(
                    log_root,
                    ExecutorName::Terminal.to_str(),
                    program_path.as_str(),
                )?;

                let workload: WorkloadSpec = WorkloadSpec::new(
                    program_path.as_str(),
                    program_args.as_deref(),
                    program_env.as_deref(),
                    input.as_deref(),
                    expected_output.as_deref(),
                    expect_empty_output,
                    expected_exit_code,
                );

                test_with_terminal_executor(
                    &runner_config,
                    iterations,
                    workload,
                    &log_layout,
                    &extra_nanvixd_args,
                    cancellation_token.clone(),
                )
                .await?;
                let context: String = format!(
                    "terminal executor completed (program={}, test={})",
                    program_path, executor
                );
                warning::fail_if_triggered(context.as_str())?;
            },
            (ExecutorName::Terminal, None) => {
                let reason: String = "test entries with terminal executor must define the \
                                      'program' field"
                    .to_string();
                error!("main(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            (ExecutorName::SnapshotRestore, Some(program_path)) => {
                if !Path::new(program_path.as_str()).exists() {
                    warn_with_policy!(
                        "main(): skipping tests with snapshot-restore executor because program \
                         path is missing (path={})",
                        program_path
                    );
                    warning::fail_if_triggered("snapshot-restore executor missing program")?;
                    continue;
                }

                let log_layout: TestLogLayout = TestLogLayout::for_program(
                    log_root,
                    ExecutorName::SnapshotRestore.to_str(),
                    program_path.as_str(),
                )?;

                let workload: WorkloadSpec = WorkloadSpec::new(
                    program_path.as_str(),
                    program_args.as_deref(),
                    program_env.as_deref(),
                    input.as_deref(),
                    expected_output.as_deref(),
                    expect_empty_output,
                    expected_exit_code,
                );

                test_with_snapshot_restore_executor(
                    &runner_config,
                    iterations,
                    workload,
                    &log_layout,
                    &extra_nanvixd_args,
                    cancellation_token.clone(),
                )
                .await?;
                let context: String = format!(
                    "snapshot-restore executor completed (program={}, test={})",
                    program_path, executor
                );
                warning::fail_if_triggered(context.as_str())?;
            },
            (ExecutorName::SnapshotRestore, None) => {
                let reason: String = "test entries with snapshot-restore executor must define the \
                                      'program' field"
                    .to_string();
                error!("main(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            (ExecutorName::SnapshotSaveExit, Some(program_path)) => {
                if !Path::new(program_path.as_str()).exists() {
                    warn_with_policy!(
                        "main(): skipping tests with snapshot-save-exit executor because program \
                         path is missing (path={})",
                        program_path
                    );
                    warning::fail_if_triggered("snapshot-save-exit executor missing program")?;
                    continue;
                }

                let log_layout: TestLogLayout = TestLogLayout::for_program(
                    log_root,
                    ExecutorName::SnapshotSaveExit.to_str(),
                    program_path.as_str(),
                )?;

                let workload: WorkloadSpec = WorkloadSpec::new(
                    program_path.as_str(),
                    program_args.as_deref(),
                    program_env.as_deref(),
                    input.as_deref(),
                    expected_output.as_deref(),
                    expect_empty_output,
                    expected_exit_code,
                );

                test_with_snapshot_save_exit_executor(
                    &runner_config,
                    iterations,
                    workload,
                    &log_layout,
                    &extra_nanvixd_args,
                    cancellation_token.clone(),
                )
                .await?;
                let context: String = format!(
                    "snapshot-save-exit executor completed (program={}, test={})",
                    program_path, executor
                );
                warning::fail_if_triggered(context.as_str())?;
            },
            (ExecutorName::SnapshotSaveExit, None) => {
                let reason: String = "test entries with snapshot-save-exit executor must define \
                                      the 'program' field"
                    .to_string();
                error!("main(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
        }
    }

    warning::fail_if_triggered("nanvix-test completion")?;

    Ok(())
}
