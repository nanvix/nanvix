// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::{
    borrow::Cow,
    path::PathBuf,
    process::{
        Command,
        Output,
    },
};

/// Environment variable that is set when a test runs in a child process.
const CHILD_PROCESS_VAR: &str = "SYSLOG_HOSTED_TESTS_CHILD_PROCESS";

/// Message of the log record that a test writes in a child process.
const RECORD_MESSAGE: &str = "syslog log destination test record";

///
/// Returns whether the calling test runs in a child process.
///
fn in_child_process() -> bool {
    ::std::env::var_os(CHILD_PROCESS_VAR).is_some()
}

///
/// Runs a test of this module in a child process and returns the output of the child process.
///
/// The logger can be initialized only once per process, so a test that initializes it reruns
/// itself in a child process, which [`in_child_process()`] detects.
///
fn run_in_child_process(test_name: &str) -> Output {
    let test_executable: PathBuf =
        ::std::env::current_exe().expect("failed to get test executable path");
    let output: Output = Command::new(test_executable)
        .args(["--exact", &format!("hosted::tests::{test_name}")])
        .env(CHILD_PROCESS_VAR, "1")
        .env("RUST_LOG", "info")
        .output()
        .expect("failed to run child process");
    let stdout: Cow<str> = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed"),
        "child process did not run {test_name} successfully: {output:?}"
    );
    output
}

#[test]
fn stdout_destination_writes_records_to_stdout() {
    if in_child_process() {
        init_with_destination(LogDestination::Stdout, "info");
        ::log::info!("{RECORD_MESSAGE}");
        return;
    }

    let output: Output = run_in_child_process("stdout_destination_writes_records_to_stdout");
    assert!(String::from_utf8_lossy(&output.stdout).contains(RECORD_MESSAGE), "{output:?}");
    assert!(!String::from_utf8_lossy(&output.stderr).contains(RECORD_MESSAGE), "{output:?}");
}

#[test]
fn stderr_destination_writes_records_to_stderr() {
    if in_child_process() {
        init_with_destination(LogDestination::Stderr, "info");
        ::log::info!("{RECORD_MESSAGE}");
        return;
    }

    let output: Output = run_in_child_process("stderr_destination_writes_records_to_stderr");
    assert!(String::from_utf8_lossy(&output.stderr).contains(RECORD_MESSAGE), "{output:?}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains(RECORD_MESSAGE), "{output:?}");
}
