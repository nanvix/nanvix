// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::globset::{
    Glob,
    GlobSet,
    GlobSetBuilder,
};

/// Builds a minimal TOML table for a test case entry with the given executor and optional
/// `expected_exit_code` value.
fn build_test_table(executor: &str, expected_exit_code: Option<Value>) -> Table {
    let mut table: Table = Table::new();
    table.insert("executor".to_string(), Value::String(executor.to_string()));
    table.insert("iterations".to_string(), Value::Integer(1));
    table.insert("build_modes".to_string(), Value::Array(vec![Value::String("debug".to_string())]));
    if let Some(exit_code) = expected_exit_code {
        table.insert("expected_exit_code".to_string(), exit_code);
    }
    table
}

fn build_globset_with_pattern(pattern: &str) -> GlobSet {
    let mut builder: GlobSetBuilder = GlobSetBuilder::new();
    builder.add(Glob::new(pattern).expect("failed to compile glob pattern"));
    builder.build().expect("failed to build glob set")
}

#[test]
fn from_table_parses_expected_exit_code() -> Result<()> {
    let table: Table = build_test_table("empty", Some(Value::Integer(0)));
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    assert_eq!(config.expected_exit_code, Some(0));
    Ok(())
}

#[test]
fn from_table_parses_negative_expected_exit_code() -> Result<()> {
    let table: Table = build_test_table("empty", Some(Value::Integer(-1)));
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    assert_eq!(config.expected_exit_code, Some(-1));
    Ok(())
}

#[test]
fn from_table_parses_absent_expected_exit_code() -> Result<()> {
    let table: Table = build_test_table("empty", None);
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    assert_eq!(config.expected_exit_code, None);
    Ok(())
}

#[test]
fn from_table_rejects_string_expected_exit_code() {
    let table: Table = build_test_table("empty", Some(Value::String("not_a_number".to_string())));
    let result: Result<TestCaseConfig> = TestCaseConfig::from_table(&table, 0);
    assert!(result.is_err());
}

#[test]
fn from_table_rejects_boolean_expected_exit_code() {
    let table: Table = build_test_table("empty", Some(Value::Boolean(true)));
    let result: Result<TestCaseConfig> = TestCaseConfig::from_table(&table, 0);
    assert!(result.is_err());
}

#[test]
fn from_table_rejects_overflowing_expected_exit_code() {
    let overflow_value: i64 = i64::from(i32::MAX) + 1;
    let table: Table = build_test_table("empty", Some(Value::Integer(overflow_value)));
    let result: Result<TestCaseConfig> = TestCaseConfig::from_table(&table, 0);
    assert!(result.is_err());
}

#[test]
fn validate_accepts_expected_exit_code() -> Result<()> {
    let config: TestCaseConfig = TestCaseConfig {
        executor: "empty".to_string(),
        name: "empty/accepts_expected_exit_code".to_string(),
        iterations: 1,
        program: None,
        program_args: None,
        input: None,
        expected_output: None,
        expect_empty_output: true,
        extra_nanvixd_args: None,
        expected_exit_code: Some(0),
        runs_on: None,
        targets: None,
        build_modes: vec!["debug".to_string()],
        program_env: None,
        program_args_padding_len: None,
        signal: None,
        kill_in_flight: false,
    };
    config.validate(0)?;
    Ok(())
}

#[test]
fn validate_accepts_absent_expected_exit_code() -> Result<()> {
    let config: TestCaseConfig = TestCaseConfig {
        executor: "empty".to_string(),
        name: "empty/accepts_absent_expected_exit_code".to_string(),
        iterations: 1,
        program: None,
        program_args: None,
        input: None,
        expected_output: None,
        expect_empty_output: false,
        extra_nanvixd_args: None,
        expected_exit_code: None,
        runs_on: None,
        targets: None,
        build_modes: vec!["debug".to_string()],
        program_env: None,
        program_args_padding_len: None,
        signal: None,
        kill_in_flight: false,
    };
    config.validate(0)?;
    Ok(())
}

#[test]
fn matches_filter_returns_true_for_no_filter() -> Result<()> {
    let mut table: Table = build_test_table("empty", None);
    table.insert("name".to_string(), Value::String("empty/wildcard".to_string()));
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    let result: bool = config.matches_filter(None);
    assert!(result, "matches_filter(None) must return true for any test case");
    Ok(())
}

#[test]
fn matches_filter_valid_pattern_matches() -> Result<()> {
    let mut table: Table = build_test_table("empty", None);
    table.insert("name".to_string(), Value::String("empty/wildcard".to_string()));
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    let globset: GlobSet = build_globset_with_pattern("empty/*");
    let result: bool = config.matches_filter(Some(&globset));
    assert!(result, "matches_filter() must return true when glob pattern matches the test name");
    Ok(())
}

#[test]
fn matches_filter_valid_pattern_does_not_match() -> Result<()> {
    let mut table: Table = build_test_table("empty", None);
    table.insert("name".to_string(), Value::String("empty/wildcard".to_string()));
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    let globset: GlobSet = build_globset_with_pattern("other/*");
    let result: bool = config.matches_filter(Some(&globset));
    assert!(
        !result,
        "matches_filter() must return false when glob pattern does not match the test name"
    );
    Ok(())
}

#[test]
fn auto_generated_name_executor_and_program() -> Result<()> {
    let mut table: Table = build_test_table("test-executor", None);
    table.insert("program".to_string(), Value::String("./bin/test-program".to_string()));
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    assert_eq!(
        config.name, "test-executor/test-program",
        "auto-generated test name must be in the format 'executor/program'"
    );
    Ok(())
}

#[test]
fn auto_generated_name_executor_only() -> Result<()> {
    // No explicit name and no program; only executor is provided.
    let table: Table = build_test_table("my-executor-only", None);
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    assert_eq!(
        config.name, "my-executor-only",
        "auto-generated test name must fall back to just the executor when program is absent"
    );
    Ok(())
}

#[test]
fn auto_generated_name_invalid_program() {
    // Program is present but has an unusable path (empty string), should error.
    let mut table: Table = build_test_table("test-executor", None);
    table.insert("program".to_string(), Value::String("".to_string()));
    let result: Result<TestCaseConfig> = TestCaseConfig::from_table(&table, 0);
    assert!(result.is_err(), "auto-generated name must fail when 'program' is present but invalid");
}

#[test]
fn validate_rejects_both_program_args_and_padding_len() {
    let config: TestCaseConfig = TestCaseConfig {
        executor: "empty".to_string(),
        name: "empty/mutual_exclusivity".to_string(),
        iterations: 1,
        program: None,
        program_args: Some("explicit-args".to_string()),
        input: None,
        expected_output: None,
        expect_empty_output: false,
        extra_nanvixd_args: None,
        expected_exit_code: None,
        runs_on: None,
        targets: None,
        build_modes: vec!["debug".to_string()],
        program_env: None,
        program_args_padding_len: Some(100),
        signal: None,
        kill_in_flight: false,
    };
    let result = config.validate(0);
    assert!(
        result.is_err(),
        "validate() must reject configs with both 'program_args' and 'program_args_padding_len'"
    );
}

/// Builds a minimal `empty` test case configuration with the given `build_modes` filter.
fn config_with_build_modes(build_modes: Vec<String>) -> TestCaseConfig {
    TestCaseConfig {
        executor: "empty".to_string(),
        name: "empty/build_mode_gate".to_string(),
        iterations: 1,
        program: None,
        program_args: None,
        input: None,
        expected_output: None,
        expect_empty_output: false,
        extra_nanvixd_args: None,
        expected_exit_code: None,
        runs_on: None,
        targets: None,
        build_modes,
        program_env: None,
        program_args_padding_len: None,
        signal: None,
        kill_in_flight: false,
    }
}

#[test]
fn should_run_in_build_mode_respects_listed_modes() {
    let config: TestCaseConfig = config_with_build_modes(vec!["release".to_string()]);
    assert!(
        config.should_run_in_build_mode("release"),
        "a release-gated test must run under release"
    );
    assert!(
        !config.should_run_in_build_mode("debug"),
        "a release-gated test must not run under debug"
    );
}

#[test]
fn should_run_in_build_mode_matches_any_listed_mode() {
    let config: TestCaseConfig =
        config_with_build_modes(vec!["debug".to_string(), "release".to_string()]);
    assert!(config.should_run_in_build_mode("debug"), "must run under a listed mode (debug)");
    assert!(config.should_run_in_build_mode("release"), "must run under a listed mode (release)");
}

#[test]
fn validate_accepts_known_build_modes() -> Result<()> {
    let config: TestCaseConfig =
        config_with_build_modes(vec!["debug".to_string(), "release".to_string()]);
    config.validate(0)?;
    Ok(())
}

#[test]
fn validate_rejects_empty_build_modes() {
    let config: TestCaseConfig = config_with_build_modes(Vec::new());
    let result = config.validate(0);
    assert!(result.is_err(), "validate() must reject an empty 'build_modes' list");
}

#[test]
fn validate_rejects_unknown_build_mode() {
    let config: TestCaseConfig = config_with_build_modes(vec!["release-profiling".to_string()]);
    let result = config.validate(0);
    assert!(result.is_err(), "validate() must reject an unknown build mode in 'build_modes'");
}

#[test]
fn from_table_requires_build_modes() {
    // A table without `build_modes` must fail to parse, since the field is required.
    let mut table: Table = Table::new();
    table.insert("executor".to_string(), Value::String("empty".to_string()));
    table.insert("iterations".to_string(), Value::Integer(1));
    let result: Result<TestCaseConfig> = TestCaseConfig::from_table(&table, 0);
    assert!(result.is_err(), "from_table() must reject a test entry missing 'build_modes'");
}

/// Builds a minimal test case configuration with the given executor and `signal` field.
fn config_with_signal(executor: &str, signal: Option<&str>) -> TestCaseConfig {
    let mut config: TestCaseConfig = config_with_build_modes(vec!["debug".to_string()]);
    config.executor = executor.to_string();
    config.name = format!("{executor}/signal");
    config.program = Some("./bin/test-program".to_string());
    config.signal = signal.map(str::to_string);
    config
}

#[test]
fn from_table_parses_signal() -> Result<()> {
    let mut table: Table = build_test_table("signal", None);
    table.insert("program".to_string(), Value::String("./bin/test-program".to_string()));
    table.insert("signal".to_string(), Value::String("SIGTERM".to_string()));
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    assert_eq!(config.signal.as_deref(), Some("SIGTERM"));
    config.validate(0)?;
    Ok(())
}

#[test]
fn validate_accepts_known_signals() -> Result<()> {
    for signal in TestCaseConfig::KNOWN_SIGNALS {
        config_with_signal("signal", Some(signal)).validate(0)?;
    }
    Ok(())
}

#[test]
fn validate_rejects_signal_executor_without_signal() {
    let result: Result<()> = config_with_signal("signal", None).validate(0);
    assert!(result.is_err(), "validate() must require 'signal' for the signal executor");
}

#[test]
fn validate_rejects_unknown_signal() {
    let result: Result<()> = config_with_signal("signal", Some("SIGKILL")).validate(0);
    assert!(result.is_err(), "validate() must reject a signal that nanvixd does not handle");
}

#[test]
fn validate_rejects_signal_executor_without_program() {
    let mut config: TestCaseConfig = config_with_signal("signal", Some("SIGTERM"));
    config.program = None;
    let result: Result<()> = config.validate(0);
    assert!(result.is_err(), "validate() must require 'program' for the signal executor");
}

#[test]
fn validate_rejects_signal_for_other_executors() {
    let result: Result<()> = config_with_signal("http", Some("SIGTERM")).validate(0);
    assert!(result.is_err(), "validate() must reject 'signal' for executors that ignore it");
}

#[test]
fn from_table_parses_kill_in_flight() -> Result<()> {
    let mut table: Table = build_test_table("signal", None);
    table.insert("program".to_string(), Value::String("./bin/test-program".to_string()));
    table.insert("signal".to_string(), Value::String("SIGTERM".to_string()));
    table.insert("kill_in_flight".to_string(), Value::Boolean(true));
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    assert!(config.kill_in_flight);
    config.validate(0)?;
    Ok(())
}

#[test]
fn from_table_defaults_kill_in_flight_to_false() -> Result<()> {
    let table: Table = build_test_table("empty", None);
    let config: TestCaseConfig = TestCaseConfig::from_table(&table, 0)?;
    assert!(!config.kill_in_flight);
    Ok(())
}

#[test]
fn validate_rejects_kill_in_flight_for_other_executors() {
    let mut config: TestCaseConfig = config_with_signal("http", None);
    config.kill_in_flight = true;
    let result: Result<()> = config.validate(0);
    assert!(result.is_err(), "validate() must reject 'kill_in_flight' for other executors");
}
