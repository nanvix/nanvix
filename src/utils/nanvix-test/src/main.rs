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
#![feature(let_chains)] // let chains in if/while conditions.

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod config;
mod environment;
mod executor;
mod log_layout;
mod nanvixd;
#[cfg(unix)]
mod port;
#[cfg(unix)]
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

#[cfg(unix)]
use crate::executor::http::test_with_http_executor;
use crate::{
    config::{
        NanvixTestConfig,
        TestCaseConfig,
    },
    environment::{
        prepare_l2_artifacts,
        prepare_runner_environment,
    },
    executor::{
        ExecutorName,
        WorkloadSpec,
        empty::empty,
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
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default log-level (overridden by RUST_LOG environment variable if set).
const DEFAULT_LOG_LEVEL: &str = "error";
/// Default tenant identifier used when creating test sandboxes.
#[cfg(unix)]
pub(crate) const DEFAULT_TENANT_ID: &str = "nanvix-test";

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

    runtime.block_on(run())
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
async fn run() -> Result<()> {
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
    let selected_tests: Vec<TestCaseConfig> = tests
        .into_iter()
        .filter(|test_config| test_config.matches_filter(test_glob_filter.as_ref()))
        .collect();
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
        "main(): selected {selected_count} of {total_tests} tests to run (filter={})",
        parsed_args
            .test_filter()
            .map(|f| format!("\"{}\"", f))
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

    if runner_config.l2_enabled
        && let Err(error) = prepare_l2_artifacts(
            runner_config.toolchain_path.as_str(),
            Path::new(runner_config.working_directory.as_str()),
        )
        .await
    {
        let reason: String = format!(
            "failed to prepare L2 artifacts (working_directory={}, toolchain={}, error={error})",
            runner_config.working_directory, runner_config.toolchain_path
        );
        error!("main(): {reason}");
        return Err(::anyhow::anyhow!(reason));
    }

    prepare_runner_environment(
        runner_config.l2_enabled,
        runner_config.port_num,
        Path::new(runner_config.tmp_directory.as_str()),
        runner_config.tcp_cleanup_max_wait_seconds,
        runner_config.tcp_cleanup_poll_interval_seconds,
    )
    .await;
    warning::fail_if_triggered("prepare_runner_environment")?;

    // Machine type used for filtering tests.
    let machine: &str = "microvm";

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
             program_args={:?}, expected_output={:?}, expect_empty_output={}, \
             expected_exit_code={:?}, extra_nanvixd_args={:?})",
            executor,
            name,
            iterations,
            program,
            program_args,
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
                empty(&runner_config, iterations, &log_layout, &extra_nanvixd_args).await?;
                let context: String = format!("empty executor completed (label={})", executor);
                warning::fail_if_triggered(context.as_str())?;
            },
            #[cfg(unix)]
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
                )
                .await?;
                let context: String = format!(
                    "http executor completed (program={}, test={})",
                    program_path, executor
                );
                warning::fail_if_triggered(context.as_str())?;
            },
            #[cfg(unix)]
            (ExecutorName::Http, None) => {
                let reason: String =
                    "tests entries with http executor must define the 'program' field".to_string();
                error!("main(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            #[cfg(not(unix))]
            (ExecutorName::Http, _) => {
                let reason: String = "HTTP executor is not supported on this platform".to_string();
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
        }
    }

    warning::fail_if_triggered("nanvix-test completion")?;

    Ok(())
}
