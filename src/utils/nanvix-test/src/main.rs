// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![forbid(clippy::all)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
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

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod config;
mod environment;
mod executor;
mod log_layout;
mod nanvixd;
mod uservm;
mod warning;

#[macro_export]
macro_rules! warn_with_policy {
    ($($arg:tt)+) => {{
        let formatted_message: String = format!($($arg)+);
        ::nanvix::log::warn!("{}", formatted_message);
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
    environment::{
        prepare_l2_artifacts,
        prepare_runner_environment,
    },
    executor::{
        ExecutorName,
        empty::empty,
        http::test_with_http_executor,
        terminal::test_with_terminal_executor,
    },
    log_layout::{
        TestLogLayout,
        initialize_run_timestamp,
    },
};
use ::anyhow::Result;
use ::nanvix::log::{
    self,
    debug,
    error,
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
    log::init(false, DEFAULT_LOG_LEVEL, String::new(), None);

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
    );
    warning::fail_if_triggered("prepare_runner_environment")?;

    for test_config in tests {
        let TestCaseConfig {
            executor,
            iterations,
            program,
            program_args,
            input,
            expected_output,
        } = test_config;

        debug!(
            "main(): running test (executor={}, iterations={}, program={:?}, program_args={:?})",
            executor, iterations, program, program_args,
        );

        let executor_name: ExecutorName = ExecutorName::from_str(executor.as_str())?;

        match (executor_name, program, program_args, input, expected_output) {
            (ExecutorName::Empty, _, _, _, _) => {
                let log_layout: TestLogLayout = TestLogLayout::for_label(
                    log_root,
                    ExecutorName::Empty.to_str(),
                    executor.as_str(),
                )?;
                empty(&runner_config, iterations, &log_layout).await?;
                let context: String = format!("empty executor completed (label={})", executor);
                warning::fail_if_triggered(context.as_str())?;
            },
            (ExecutorName::Http, Some(program_path), program_args, input, expected_output) => {
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

                test_with_http_executor(
                    &runner_config,
                    iterations,
                    program_path.as_str(),
                    program_args.as_deref(),
                    input.as_deref(),
                    expected_output.as_deref(),
                    &log_layout,
                )
                .await?;
                let context: String = format!(
                    "http executor completed (program={}, test={})",
                    program_path, executor
                );
                warning::fail_if_triggered(context.as_str())?;
            },
            (ExecutorName::Http, None, _, _, _) => {
                let reason: String =
                    "tests entries with http executor must define the 'program' field".to_string();
                error!("main(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            (ExecutorName::Terminal, Some(program_path), program_args, input, expected_output) => {
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

                test_with_terminal_executor(
                    &runner_config,
                    iterations,
                    program_path.as_str(),
                    program_args.as_deref(),
                    input.as_deref(),
                    expected_output.as_deref(),
                    &log_layout,
                )
                .await?;
                let context: String = format!(
                    "terminal executor completed (program={}, test={})",
                    program_path, executor
                );
                warning::fail_if_triggered(context.as_str())?;
            },
            (ExecutorName::Terminal, None, _, _, _) => {
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
