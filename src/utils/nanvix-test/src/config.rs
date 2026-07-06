// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::globset::GlobSet;
use ::nanvixd::config::DEFAULT_TMP_DIRECTORY;
use ::std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
};
use ::toml::{
    Value,
    value::Table,
};

//==================================================================================================
// Constants
//==================================================================================================

// -------------------------------------------------------------------------------------------------
// Nanvix Daemon Shutdown Configuration
// -------------------------------------------------------------------------------------------------
// These values control how long the test harness waits for nanvixd to exit gracefully after
// sending SIGINT. If the timeout is exceeded, SIGKILL is sent.
//
// 10 attempts × 100ms = 1 second total.
// This is sufficient for nanvixd to propagate shutdown to linuxd (which has an internal 1-second
// shutdown timeout) and perform basic cleanup.
// -------------------------------------------------------------------------------------------------

/// Default number of Nanvix Daemon shutdown attempts when omitted in the TOML file.
const DEFAULT_NANVIXD_SHUTDOWN_ATTEMPTS_MAX: usize = 10;
/// Default Nanvix Daemon shutdown retry interval (in milliseconds) when omitted in the TOML file.
const DEFAULT_NANVIXD_SHUTDOWN_RETRY_INTERVAL_MS: u64 = 100;

// -------------------------------------------------------------------------------------------------
// Nanvix Daemon Readiness Configuration
// -------------------------------------------------------------------------------------------------
// These values control how long the test harness waits for nanvixd to become ready (accept HTTP
// connections) after spawning. The daemon must bind its HTTP socket and initialize the sandbox
// cache before accepting requests.
//
// Total timeout: 50 attempts × 100ms = 5 seconds.
// -------------------------------------------------------------------------------------------------

/// Default number of readiness probes issued before giving up on the Nanvix Daemon HTTP endpoint.
const DEFAULT_NANVIXD_READY_ATTEMPTS_MAX: usize = 50;
/// Default interval (in milliseconds) between Nanvix Daemon readiness probes.
const DEFAULT_NANVIXD_READY_RETRY_INTERVAL_MS: u64 = 100;

// -------------------------------------------------------------------------------------------------
// Test Iteration Configuration
// -------------------------------------------------------------------------------------------------

/// Default iteration count for the requested test case when not specified.
const DEFAULT_TEST_ITERATIONS: usize = 1;

// -------------------------------------------------------------------------------------------------
// User VM Cleanup Configuration
// -------------------------------------------------------------------------------------------------
// Brief pause between User VM teardowns to avoid resource contention (file descriptors, sockets).
// -------------------------------------------------------------------------------------------------

/// Default delay (in milliseconds) before launching another User VM.
const DEFAULT_CLEANUP_USERVM_SLEEP_DURATION_MS: u64 = 10;

// -------------------------------------------------------------------------------------------------
// Stream Collection Configuration
// -------------------------------------------------------------------------------------------------
// Maximum time allowed for collecting stdout/stderr from interactive workloads. Set to 5 minutes
// to accommodate long-running tests (stress tests, interpreter benchmarks like QuickJS test
// suites) that may produce output over extended periods.
// -------------------------------------------------------------------------------------------------

/// Default timeout (in milliseconds) applied when collecting interactive stdout/stderr streams.
const DEFAULT_STREAM_COLLECTION_TIMEOUT_MS: u64 = 300_000;

// -------------------------------------------------------------------------------------------------
// Gateway Connection Configuration
// -------------------------------------------------------------------------------------------------
// These values control the client-side connection loop when connecting to a User VM gateway
// socket. User VM startup time is variable (depends on kernel boot, linuxd initialization,
// workload size), so exponential backoff is used.
//
// Max attempts: 100, with exponential backoff from 10ms to 500ms.
// Total timeout: 15 seconds hard cap on the connection loop.
// -------------------------------------------------------------------------------------------------

/// Default maximum number of gateway connection attempts before failing a spawn.
const DEFAULT_GATEWAY_CONNECT_MAX_ATTEMPTS: usize = 100;
/// Default initial backoff (in milliseconds) between gateway connection retries.
const DEFAULT_GATEWAY_CONNECT_INITIAL_BACKOFF_MS: u64 = 10;
/// Default maximum backoff (in milliseconds) between gateway connection retries.
const DEFAULT_GATEWAY_CONNECT_MAX_BACKOFF_MS: u64 = 500;
/// Default timeout (in milliseconds) applied to the gateway connection loop.
const DEFAULT_GATEWAY_CONNECT_TIMEOUT_MS: u64 = 15_000;

// -------------------------------------------------------------------------------------------------
// Miscellaneous
// -------------------------------------------------------------------------------------------------

/// Placeholder token replaced with the configured sysroot path inside test definitions.
const SYSROOT_PATH_PLACEHOLDER: &str = "${sysroot_path}";

//==================================================================================================
// Nanvix Test Configuration
//==================================================================================================

///
/// # Description
///
/// Complete Nanvix test configuration loaded from a TOML definition.
///
pub struct NanvixTestConfig {
    /// Runner configuration applied when spawning the Nanvix Daemon.
    pub runner: RunnerConfig,
    /// List of test cases to execute sequentially.
    pub tests: Vec<TestCaseConfig>,
}

impl NanvixTestConfig {
    ///
    /// # Description
    ///
    /// Loads the Nanvix test configuration from a TOML file.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the TOML configuration file.
    ///
    /// # Return Value
    ///
    /// Returns the parsed configuration when the file is readable and valid; returns an error
    /// when file I/O, TOML parsing, or validation fails.
    ///
    pub fn from_path(path: &Path) -> Result<Self> {
        let contents: String = match fs::read_to_string(path) {
            Ok(data) => data,
            Err(error) => {
                let reason: String = format!(
                    "failed to read nanvix-test config (path={}, error={error})",
                    path.display()
                );
                return Err(::anyhow::anyhow!(reason));
            },
        };

        let parsed_value: Value = match ::toml::from_str(contents.as_str()) {
            Ok(parsed) => parsed,
            Err(error) => {
                let reason: String = format!(
                    "failed to parse nanvix-test config (path={}, error={error})",
                    path.display()
                );
                return Err(::anyhow::anyhow!(reason));
            },
        };

        let mut config: NanvixTestConfig = NanvixTestConfig::from_toml_value(parsed_value)?;

        let base_dir: &Path = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        config.runner.resolve_paths(base_dir)?;

        for (index, test_config) in config.tests.iter_mut().enumerate() {
            test_config.apply_runner_placeholders(&config.runner);
            test_config.resolve_paths(base_dir, index)?;
            test_config.validate(index)?;
        }

        Ok(config)
    }

    ///
    /// # Description
    ///
    /// Builds a `NanvixTestConfig` structure from the parsed TOML root value.
    ///
    /// # Parameters
    ///
    /// - `value`: Parsed TOML representation of the configuration file.
    ///
    /// # Return Value
    ///
    /// Returns the fully populated configuration tree when the TOML schema matches the expected
    /// layout; returns an error when the `runner` or `tests` tables are malformed.
    ///
    /// # Errors
    ///
    /// Returns an error when the root value is not a table or nested entries use unexpected
    /// types.
    ///
    fn from_toml_value(value: Value) -> Result<Self> {
        let root_table: Table = match value {
            Value::Table(table) => table,
            other => {
                let reason: String = format!(
                    "nanvix-test config must be a TOML table (found={})",
                    describe_toml_type(&other)
                );
                return Err(::anyhow::anyhow!(reason));
            },
        };

        let runner_table: &Table = get_required_table(&root_table, "runner", "runner")?;
        let runner: RunnerConfig = RunnerConfig::from_table(runner_table)?;

        let test_entries: &Vec<Value> = get_required_array(&root_table, "tests", "tests")?;
        if test_entries.is_empty() {
            let reason: String =
                "at least one test entry must be provided under '[[tests]]'".to_string();
            return Err(::anyhow::anyhow!(reason));
        }

        let mut tests: Vec<TestCaseConfig> = Vec::with_capacity(test_entries.len());
        for (index, entry) in test_entries.iter().enumerate() {
            let field_name: String = format!("tests[{index}]");
            let entry_table: &Table = match entry {
                Value::Table(table) => table,
                other => {
                    let reason: String = format!(
                        "{field_name} must be a table (found={})",
                        describe_toml_type(other)
                    );
                    return Err(::anyhow::anyhow!(reason));
                },
            };
            tests.push(TestCaseConfig::from_table(entry_table, index)?);
        }

        Ok(Self { runner, tests })
    }
}

//==================================================================================================
// Runner Configuration
//==================================================================================================

///
/// # Description
///
/// Configuration required to spawn a Nanvix Daemon instance.
#[allow(dead_code)]
#[derive(Clone)]
pub struct RunnerConfig {
    /// Path to the Nanvix Daemon executable that should be launched.
    pub nanvixd_binary_path: String,
    /// Directory used as the current working directory for the Nanvix Daemon.
    pub working_directory: String,
    /// Directory where Nanvix Daemon stdout/stderr logs are written.
    pub log_directory: String,
    /// Directory scanned for temporary artifacts created during Nanvix test runs.
    pub tmp_directory: String,
    /// IPv4 address where the Nanvix Daemon should listen for HTTP requests.
    pub ipv4_addr: String,
    /// TCP port where the Nanvix Daemon exposes its HTTP endpoint.
    pub port_num: u16,
    /// Optional hwloc topology description forwarded to the Nanvix Daemon. Empty strings are
    /// treated as unset values.
    pub hwloc_file_path: Option<String>,
    /// Flag enabling fatal mode, causing warnings to fail tests when set to `true`.
    pub fatal: bool,
    /// Path to the toolchain root; its `bin/` directory is forwarded to the Nanvix Daemon.
    pub toolchain_path: String,
    /// Path to the sysroot that hosts interpreter runtimes and shared assets.
    pub sysroot_path: String,
    /// Maximum number of Nanvix Daemon shutdown polling attempts before giving up.
    pub nanvixd_shutdown_attempts_max: usize,
    /// Milliseconds to wait between Nanvix Daemon shutdown polling attempts.
    pub nanvixd_shutdown_retry_interval_ms: u64,
    /// Maximum number of readiness probes issued before giving up on the Nanvix Daemon HTTP endpoint.
    pub nanvixd_ready_attempts_max: usize,
    /// Interval (in milliseconds) between readiness probes for the Nanvix Daemon HTTP endpoint.
    pub nanvixd_ready_retry_interval_ms: u64,
    /// Milliseconds to wait after tearing down a User VM before spawning the next workload.
    pub cleanup_uservm_sleep_duration_ms: u64,
    /// Maximum time (in milliseconds) allowed for collecting interactive stdout/stderr streams.
    pub stream_collection_timeout_ms: u64,
    /// Maximum number of attempts performed while connecting to a uservm gateway.
    pub gateway_connect_max_attempts: usize,
    /// Initial backoff (in milliseconds) between uservm gateway connection retries.
    pub gateway_connect_initial_backoff_ms: u64,
    /// Maximum backoff (in milliseconds) between uservm gateway connection retries.
    pub gateway_connect_max_backoff_ms: u64,
    /// Maximum time (in milliseconds) spent attempting to connect to the uservm gateway.
    pub gateway_connect_timeout_ms: u64,
}

impl RunnerConfig {
    ///
    /// # Description
    ///
    /// Parses the `[runner]` table from the TOML configuration and produces a `RunnerConfig`
    /// instance.
    ///
    /// # Parameters
    ///
    /// - `table`: Key/value map containing the runner configuration fields.
    ///
    /// # Return Value
    ///
    /// Returns the parsed `RunnerConfig` when every required field is present and valid; returns
    /// an error when parsing fails or a field uses an unexpected type.
    ///
    fn from_table(table: &Table) -> Result<Self> {
        Ok(Self {
            nanvixd_binary_path: read_required_string(
                table,
                "nanvixd_binary_path",
                "runner.nanvixd_binary_path",
            )?,
            working_directory: read_required_string(
                table,
                "working_directory",
                "runner.working_directory",
            )?,
            log_directory: read_required_string(table, "log_directory", "runner.log_directory")?,
            tmp_directory: read_string_with_default(
                table,
                "tmp_directory",
                "runner.tmp_directory",
                default_tmp_directory,
            )?,
            ipv4_addr: read_required_string(table, "ipv4_addr", "runner.ipv4_addr")?,
            port_num: read_u16_required(table, "port_num", "runner.port_num")?,
            hwloc_file_path: read_hwloc_file_path(table)?,
            fatal: read_bool_with_default(table, "fatal", "runner.fatal", false)?,
            toolchain_path: read_required_string(table, "toolchain_path", "runner.toolchain_path")?,
            sysroot_path: read_required_non_empty_string(
                table,
                "sysroot_path",
                "runner.sysroot_path",
            )?,
            nanvixd_shutdown_attempts_max: read_usize_with_default(
                table,
                "nanvixd_shutdown_attempts_max",
                "runner.nanvixd_shutdown_attempts_max",
                default_nanvixd_shutdown_attempts_max(),
            )?,
            nanvixd_shutdown_retry_interval_ms: read_u64_with_default(
                table,
                "nanvixd_shutdown_retry_interval_ms",
                "runner.nanvixd_shutdown_retry_interval_ms",
                default_nanvixd_shutdown_retry_interval_ms(),
            )?,
            nanvixd_ready_attempts_max: read_usize_with_default(
                table,
                "nanvixd_ready_attempts_max",
                "runner.nanvixd_ready_attempts_max",
                default_nanvixd_ready_attempts_max(),
            )?,
            nanvixd_ready_retry_interval_ms: read_u64_with_default(
                table,
                "nanvixd_ready_retry_interval_ms",
                "runner.nanvixd_ready_retry_interval_ms",
                default_nanvixd_ready_retry_interval_ms(),
            )?,
            cleanup_uservm_sleep_duration_ms: read_u64_with_default(
                table,
                "cleanup_uservm_sleep_duration_ms",
                "runner.cleanup_uservm_sleep_duration_ms",
                default_cleanup_uservm_sleep_duration_ms(),
            )?,
            stream_collection_timeout_ms: read_u64_with_default(
                table,
                "stream_collection_timeout_ms",
                "runner.stream_collection_timeout_ms",
                default_stream_collection_timeout_ms(),
            )?,
            gateway_connect_max_attempts: read_usize_with_default(
                table,
                "gateway_connect_max_attempts",
                "runner.gateway_connect_max_attempts",
                default_gateway_connect_max_attempts(),
            )?,
            gateway_connect_initial_backoff_ms: read_u64_with_default(
                table,
                "gateway_connect_initial_backoff_ms",
                "runner.gateway_connect_initial_backoff_ms",
                default_gateway_connect_initial_backoff_ms(),
            )?,
            gateway_connect_max_backoff_ms: read_u64_with_default(
                table,
                "gateway_connect_max_backoff_ms",
                "runner.gateway_connect_max_backoff_ms",
                default_gateway_connect_max_backoff_ms(),
            )?,
            gateway_connect_timeout_ms: read_u64_with_default(
                table,
                "gateway_connect_timeout_ms",
                "runner.gateway_connect_timeout_ms",
                default_gateway_connect_timeout_ms(),
            )?,
        })
    }

    ///
    /// # Description
    ///
    /// Resolves relative paths using the directory that contains the TOML file.
    ///
    /// # Parameters
    ///
    /// - `base_dir`: Directory used to resolve relative paths.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` once all relative paths are resolved successfully; returns an error when
    /// any resolved path cannot be represented as UTF-8.
    ///
    pub fn resolve_paths(&mut self, base_dir: &Path) -> Result<()> {
        let invocation_dir: PathBuf = current_working_directory()?;

        self.nanvixd_binary_path = resolve_with_invocation_dirs(
            base_dir,
            invocation_dir.as_path(),
            self.nanvixd_binary_path.as_str(),
            "runner.nanvixd_binary_path",
        )?;
        self.working_directory = resolve_with_invocation_dirs(
            base_dir,
            invocation_dir.as_path(),
            self.working_directory.as_str(),
            "runner.working_directory",
        )?;
        self.log_directory = resolve_with_invocation_dirs(
            base_dir,
            invocation_dir.as_path(),
            self.log_directory.as_str(),
            "runner.log_directory",
        )?;
        self.tmp_directory = resolve_with_invocation_dirs(
            base_dir,
            invocation_dir.as_path(),
            self.tmp_directory.as_str(),
            "runner.tmp_directory",
        )?;
        self.toolchain_path = resolve_with_invocation_dirs(
            base_dir,
            invocation_dir.as_path(),
            self.toolchain_path.as_str(),
            "runner.toolchain_path",
        )?;
        self.sysroot_path = resolve_with_invocation_dirs(
            base_dir,
            invocation_dir.as_path(),
            self.sysroot_path.as_str(),
            "runner.sysroot_path",
        )?;

        if let Some(path) = self.hwloc_file_path.clone() {
            self.hwloc_file_path = Some(resolve_with_invocation_dirs(
                base_dir,
                invocation_dir.as_path(),
                path.as_str(),
                "runner.hwloc_file_path",
            )?);
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Builds the IPv4:port socket string used to reach the Nanvix Daemon HTTP endpoint.
    ///
    /// # Return Value
    ///
    /// Returns a socket address string formatted as `addr:port`.
    ///
    pub fn http_endpoint(&self) -> String {
        format!("{}:{}", self.ipv4_addr, self.port_num)
    }
}

//==================================================================================================
// Test Case Configuration
//==================================================================================================

///
/// # Description
///
/// Description of the test case that should be executed.
///
pub struct TestCaseConfig {
    /// Executor identifier requested for the test case (e.g., `empty`, `http`, `terminal`).
    pub executor: String,
    /// Name for the test case. If none specified, defaults to an autogenerated name
    /// based on the executor and program path (e.g., http/echo-rust-nostd.elf).
    pub name: String,
    /// Number of times the test case should run.
    pub iterations: usize,
    /// Optional program path required by some test cases.
    pub program: Option<String>,
    /// Optional command-line arguments forwarded to the workload under test.
    pub program_args: Option<String>,
    /// Optional input payload forwarded to the workload under test.
    pub input: Option<String>,
    /// Optional output payload used when validating the workload response.
    pub expected_output: Option<String>,
    /// Flag indicating that the workload must not produce any stdout output.
    pub expect_empty_output: bool,
    /// Optional extra arguments passed directly to nanvixd.
    pub extra_nanvixd_args: Option<String>,
    /// Optional expected exit code that the workload must produce.
    pub expected_exit_code: Option<i32>,
    /// Optional list of machine types on which this test should run.
    pub runs_on: Option<Vec<String>>,
    /// Required, non-empty list of build modes (`debug`, `release`) in which this test should run.
    /// The test is skipped in any build mode not listed. This gates heavy tests to builds where
    /// they are tractable: a test that loads a large image many times is prohibitively slow under a
    /// `debug` (trace-enabled) build, where the kernel emits per-page trace over a byte-at-a-time
    /// UART, but completes quickly under a `release` (trace-disabled) build.
    pub build_modes: Vec<String>,
    /// Optional environment variables forwarded to the workload under test.
    /// Formatted as space-separated `KEY=VALUE` pairs (e.g., `"FOO=bar BAZ=qux"`).
    pub program_env: Option<String>,
    /// Optional padding length used to generate a synthetic `program_args` string at runtime.
    /// When set, the runner creates a repeated-character string of exactly this many bytes and
    /// uses it as `program_args`. Mutually exclusive with `program_args`.
    pub program_args_padding_len: Option<usize>,
}

impl TestCaseConfig {
    /// Build modes recognized by the `build_modes` gate and by the active `BUILD_MODE` selector.
    pub const KNOWN_BUILD_MODES: [&'static str; 2] = ["debug", "release"];

    ///
    /// # Description
    ///
    /// Parses a single `[[tests]]` entry from the TOML configuration and populates a
    /// `TestCaseConfig` structure.
    ///
    /// # Parameters
    ///
    /// - `table`: TOML table that stores the test case fields.
    /// - `index`: Position of the entry within the `tests` array (used for error reporting).
    ///
    /// # Return Value
    ///
    /// Returns the parsed test case configuration when all fields are valid; returns an error
    /// when a required key is missing or of the wrong type.
    ///
    fn from_table(table: &Table, index: usize) -> Result<Self> {
        let entry_prefix: String = format!("tests[{index}]");
        let name_field: String = format!("{entry_prefix}.name");
        let executor_field: String = format!("{entry_prefix}.executor");
        let iterations_field: String = format!("{entry_prefix}.iterations");
        let program_field: String = format!("{entry_prefix}.program");
        let program_args_field: String = format!("{entry_prefix}.program_args");
        let input_field: String = format!("{entry_prefix}.input");
        let expected_output_field: String = format!("{entry_prefix}.expected_output");
        let expect_empty_output_field: String = format!("{entry_prefix}.expect_empty_output");
        let extra_nanvixd_args_field: String = format!("{entry_prefix}.extra_nanvixd_args");
        let expected_exit_code_field: String = format!("{entry_prefix}.expected_exit_code");
        let runs_on_field: String = format!("{entry_prefix}.runs_on");
        let build_modes_field: String = format!("{entry_prefix}.build_modes");
        let program_env_field: String = format!("{entry_prefix}.program_env");
        let program_args_padding_len_field: String =
            format!("{entry_prefix}.program_args_padding_len");

        Ok(Self {
            executor: read_required_string(table, "executor", executor_field.as_str())?,
            iterations: read_usize_with_default(
                table,
                "iterations",
                iterations_field.as_str(),
                default_test_iterations(),
            )?,
            program: read_optional_string(table, "program", program_field.as_str())?,
            program_args: read_optional_string(table, "program_args", program_args_field.as_str())?,
            name: read_optional_name_field(table, "name", name_field.as_str())?,
            input: read_optional_string(table, "input", input_field.as_str())?,
            expected_output: read_optional_string(
                table,
                "expected_output",
                expected_output_field.as_str(),
            )?,
            expect_empty_output: read_bool_with_default(
                table,
                "expect_empty_output",
                expect_empty_output_field.as_str(),
                false,
            )?,
            extra_nanvixd_args: read_optional_string(
                table,
                "extra_nanvixd_args",
                extra_nanvixd_args_field.as_str(),
            )?,
            expected_exit_code: read_optional_i32(
                table,
                "expected_exit_code",
                expected_exit_code_field.as_str(),
            )?,
            runs_on: read_optional_string_array(table, "runs_on", runs_on_field.as_str())?,
            build_modes: read_optional_string_array(
                table,
                "build_modes",
                build_modes_field.as_str(),
            )?
            .ok_or_else(|| ::anyhow::anyhow!("{build_modes_field} is a required field"))?,
            program_env: read_optional_string(table, "program_env", program_env_field.as_str())?,
            program_args_padding_len: read_optional_usize(
                table,
                "program_args_padding_len",
                program_args_padding_len_field.as_str(),
            )?,
        })
    }

    ///
    /// # Description
    ///
    /// Validates the parsed test case configuration.
    ///
    /// # Parameters
    ///
    /// - `index`: Position of the test case in the configuration (used for error reporting).
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` when the test case definition is valid; returns an error if iterations are
    /// zero or required fields are missing for the selected executor name.
    ///
    pub fn validate(&self, index: usize) -> Result<()> {
        if self.iterations == 0 {
            let reason: String = format!(
                "tests[{index}] iterations must be greater than zero (executor={})",
                self.executor
            );
            return Err(::anyhow::anyhow!(reason));
        }

        if (self.executor == "http" || self.executor == "terminal") && self.program.is_none() {
            let reason: String = format!(
                "tests[{index}] requires the 'program' field when executor is '{}'",
                self.executor
            );
            return Err(::anyhow::anyhow!(reason));
        }

        if self.expect_empty_output
            && self
                .expected_output
                .as_ref()
                .map(|output| !output.is_empty())
                .unwrap_or(false)
        {
            let reason: String = format!(
                "tests[{index}] cannot set 'expect_empty_output=true' while also providing a \
                 non-empty expected_output"
            );
            return Err(::anyhow::anyhow!(reason));
        }

        if self.program_args.is_some() && self.program_args_padding_len.is_some() {
            let reason: String = format!(
                "tests[{index}] cannot set both 'program_args' and 'program_args_padding_len'"
            );
            return Err(::anyhow::anyhow!(reason));
        }

        // `build_modes` must list at least one mode: an empty list would match no build mode and
        // silently disable the test in every run.
        if self.build_modes.is_empty() {
            let reason: String =
                format!("tests[{index}] must specify a non-empty 'build_modes' list");
            return Err(::anyhow::anyhow!(reason));
        }

        // Reject unknown build-mode filters early: a typo (e.g. "Release") would otherwise match no
        // build mode and silently exclude the test from every run.
        if let Some(unknown_mode) = self
            .build_modes
            .iter()
            .find(|mode| !Self::KNOWN_BUILD_MODES.contains(&mode.as_str()))
        {
            let reason: String = format!(
                "tests[{index}] has an unknown build mode '{unknown_mode}' in 'build_modes' \
                 (known modes: {})",
                Self::KNOWN_BUILD_MODES.join(", ")
            );
            return Err(::anyhow::anyhow!(reason));
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Applies runner-provided placeholders to optional program paths.
    ///
    /// # Parameters
    ///
    /// - `runner`: Runner configuration that exposes placeholder values.
    ///
    pub fn apply_runner_placeholders(&mut self, runner: &RunnerConfig) {
        if let Some(program_path) = self.program.as_mut()
            && program_path.contains(SYSROOT_PATH_PLACEHOLDER)
        {
            let substituted: String =
                program_path.replace(SYSROOT_PATH_PLACEHOLDER, runner.sysroot_path.as_str());
            *program_path = substituted;
        }
    }

    ///
    /// # Description
    ///
    /// Resolves optional relative paths present in the test case definition.
    ///
    /// # Parameters
    ///
    /// - `base_dir`: Directory used to resolve relative paths.
    /// - `index`: Index of this test entry (used for error reporting).
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` when optional paths resolve successfully; returns an error if the resolved
    /// path cannot be represented as UTF-8.
    ///
    pub fn resolve_paths(&mut self, base_dir: &Path, index: usize) -> Result<()> {
        if let Some(program_path) = self.program.clone() {
            let field_name: String = format!("tests[{index}].program");
            let invocation_dir: PathBuf = current_working_directory()?;
            let resolved_path: String = resolve_with_invocation_dirs(
                base_dir,
                invocation_dir.as_path(),
                program_path.as_str(),
                field_name.as_str(),
            )?;
            self.program = Some(resolved_path);
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Checks whether this test case should run on the specified machine type.
    ///
    /// # Parameters
    ///
    /// - `machine`: Machine type to check against.
    ///
    /// # Return Value
    ///
    /// Returns `true` when the test should run on the given machine; returns `false` when the
    /// test is restricted to other machines.
    ///
    pub fn should_run_on(&self, machine: &str) -> bool {
        match &self.runs_on {
            // No filter specified in test config - run on all machines.
            None => true,
            // Filter specified - check if machine is in the list.
            Some(allowed_machines) => allowed_machines.iter().any(|m| m == machine),
        }
    }

    ///
    /// # Description
    ///
    /// Checks whether this test case should run in the specified build mode.
    ///
    /// # Parameters
    ///
    /// - `build_mode`: Build mode to check against (`debug` or `release`).
    ///
    /// # Return Value
    ///
    /// Returns `true` when the test lists the given build mode in its `build_modes` filter; returns
    /// `false` otherwise.
    ///
    pub fn should_run_in_build_mode(&self, build_mode: &str) -> bool {
        self.build_modes.iter().any(|m| m == build_mode)
    }

    ///
    /// # Description
    ///
    /// Compares the test case name against a glob filter to determine if it matches.
    ///
    /// # Return Value
    ///
    /// Returns `true` if the test case name matches the filter; otherwise returns `false`.
    ///
    pub fn matches_filter(&self, filter: Option<&GlobSet>) -> bool {
        match filter {
            None => true,
            Some(globset) => globset.is_match(self.name.as_str()),
        }
    }
}

//==================================================================================================
// Helper Functions
//==================================================================================================

///
/// # Description
///
/// Returns the default number of shutdown attempts used when the value is omitted in the
/// configuration file.
///
/// # Return Value
///
/// Returns the maximum number of shutdown attempts applied to Nanvix Daemon teardown.
///
fn default_nanvixd_shutdown_attempts_max() -> usize {
    DEFAULT_NANVIXD_SHUTDOWN_ATTEMPTS_MAX
}

///
/// # Description
///
/// Returns the default delay between shutdown attempts when the field is not specified.
///
/// # Return Value
///
/// Returns the milliseconds to wait between Nanvix Daemon shutdown polling attempts.
///
fn default_nanvixd_shutdown_retry_interval_ms() -> u64 {
    DEFAULT_NANVIXD_SHUTDOWN_RETRY_INTERVAL_MS
}

///
/// # Description
///
/// Returns the default number of readiness probes for the Nanvix Daemon HTTP endpoint.
///
/// # Return Value
///
/// Returns the maximum number of Nanvix Daemon readiness probes.
///
fn default_nanvixd_ready_attempts_max() -> usize {
    DEFAULT_NANVIXD_READY_ATTEMPTS_MAX
}

///
/// # Description
///
/// Returns the default interval between Nanvix Daemon readiness probes.
///
/// # Return Value
///
/// Returns the milliseconds to wait between readiness attempts.
///
fn default_nanvixd_ready_retry_interval_ms() -> u64 {
    DEFAULT_NANVIXD_READY_RETRY_INTERVAL_MS
}

///
/// # Description
///
/// Returns the default temporary directory path applied when the configuration omits the field.
///
/// # Return Value
///
/// Returns the temporary directory path scanned for Nanvix test artifacts.
///
fn default_tmp_directory() -> String {
    DEFAULT_TMP_DIRECTORY.to_string()
}

///
/// # Description
///
/// Reports the default number of iterations each test case should run when unspecified.
///
/// # Return Value
///
/// Returns the number of repetitions applied to a test case.
///
fn default_test_iterations() -> usize {
    DEFAULT_TEST_ITERATIONS
}

///
/// # Description
///
/// Provides the default delay between User VM launches during cleanup.
///
/// # Return Value
///
/// Returns the milliseconds spent waiting before launching the next User VM.
///
fn default_cleanup_uservm_sleep_duration_ms() -> u64 {
    DEFAULT_CLEANUP_USERVM_SLEEP_DURATION_MS
}

///
/// # Description
///
/// Provides the default timeout applied when collecting interactive stdout/stderr streams.
///
/// # Return Value
///
/// Returns the milliseconds spent waiting for interactive stream collection.
///
fn default_stream_collection_timeout_ms() -> u64 {
    DEFAULT_STREAM_COLLECTION_TIMEOUT_MS
}

///
/// # Description
///
/// Provides the default maximum number of gateway connection attempts.
///
/// # Return Value
///
/// Returns the retry budget applied while connecting to uservm gateways.
///
fn default_gateway_connect_max_attempts() -> usize {
    DEFAULT_GATEWAY_CONNECT_MAX_ATTEMPTS
}

///
/// # Description
///
/// Provides the default initial gateway connection backoff.
///
/// # Return Value
///
/// Returns the milliseconds spent waiting before the first retry.
///
fn default_gateway_connect_initial_backoff_ms() -> u64 {
    DEFAULT_GATEWAY_CONNECT_INITIAL_BACKOFF_MS
}

///
/// # Description
///
/// Provides the default maximum gateway connection backoff.
///
/// # Return Value
///
/// Returns the upper bound on the milliseconds spent between retries.
///
fn default_gateway_connect_max_backoff_ms() -> u64 {
    DEFAULT_GATEWAY_CONNECT_MAX_BACKOFF_MS
}

///
/// # Description
///
/// Provides the default timeout applied to gateway connection attempts.
///
/// # Return Value
///
/// Returns the milliseconds spent trying to connect to the gateway before failing.
///
fn default_gateway_connect_timeout_ms() -> u64 {
    DEFAULT_GATEWAY_CONNECT_TIMEOUT_MS
}

///
/// # Description
///
/// Retrieves the working directory used when `nanvix-test` was invoked.
///
/// # Return Value
///
/// Returns the invocation directory when it can be queried successfully.
///
/// # Errors
///
/// Returns an error when the filesystem query for the current directory fails.
///
fn current_working_directory() -> Result<PathBuf> {
    match ::std::env::current_dir() {
        Ok(directory) => Ok(directory),
        Err(error) => {
            let reason: String =
                format!("failed to read current working directory (error={error})");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Resolves a configuration-provided path by first interpreting it relative to the invocation
/// directory and then falling back to the configuration directory when there is no matching entry.
///
/// # Parameters
///
/// - `config_dir`: Directory where the configuration file resides.
/// - `invocation_dir`: Directory where `nanvix-test` was launched.
/// - `provided`: Path string supplied in the configuration file.
/// - `field_name`: Fully qualified configuration key used for error reporting.
///
/// # Return Value
///
/// Returns the normalized path string.
///
fn resolve_with_invocation_dirs(
    config_dir: &Path,
    invocation_dir: &Path,
    provided: &str,
    field_name: &str,
) -> Result<String> {
    let provided_path: &Path = Path::new(provided);

    if provided_path.is_absolute() {
        return path_to_utf8_string(provided_path, field_name);
    }

    let invocation_candidate: PathBuf = invocation_dir.join(provided_path);
    if invocation_candidate.exists() {
        return path_to_utf8_string(invocation_candidate.as_path(), field_name);
    }

    let config_candidate: PathBuf = config_dir.join(provided_path);
    if config_candidate.exists() {
        return path_to_utf8_string(config_candidate.as_path(), field_name);
    }

    path_to_utf8_string(invocation_candidate.as_path(), field_name)
}

///
/// # Description
///
/// Converts a resolved filesystem path into a UTF-8 string or reports an error when conversion is
/// not possible.
///
/// # Parameters
///
/// - `resolved_path`: Path inspected for UTF-8 compatibility.
/// - `field_name`: Fully qualified configuration key used for error reporting.
///
/// # Return Value
///
/// Returns the path as a UTF-8 string when the conversion succeeds; returns an error when the
/// path contains non-UTF-8 data.
///
fn path_to_utf8_string(resolved_path: &Path, field_name: &str) -> Result<String> {
    match resolved_path.to_str() {
        Some(value) => Ok(value.to_string()),
        None => {
            let reason: String =
                format!("{field_name} resolves to a non-UTF-8 path (path={resolved_path:?})");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Describes the TOML data type of the provided value for diagnostics.
///
/// # Parameters
///
/// - `value`: TOML value inspected for its variant name.
///
/// # Return Value
///
/// Returns a static string that mirrors the TOML variant name.
///
fn describe_toml_type(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::Boolean(_) => "boolean",
        Value::Datetime(_) => "datetime",
        Value::Array(_) => "array",
        Value::Table(_) => "table",
    }
}

///
/// # Description
///
/// Retrieves a required nested table from the provided TOML table.
///
/// # Parameters
///
/// - `table`: Parent table searched for the nested table.
/// - `key`: Key used to locate the nested table.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns a reference to the nested table when present.
///
/// # Errors
///
/// Returns an error when the key is missing or the associated value is not a table.
///
fn get_required_table<'a>(table: &'a Table, key: &str, field_name: &str) -> Result<&'a Table> {
    match table.get(key) {
        Some(Value::Table(value)) => Ok(value),
        Some(other) => {
            let reason: String =
                format!("{field_name} must be a table (found={})", describe_toml_type(other));
            Err(::anyhow::anyhow!(reason))
        },
        None => {
            let reason: String = format!("missing required table '{field_name}'");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Retrieves a required array from the provided TOML table.
///
/// # Parameters
///
/// - `table`: Parent table searched for the array.
/// - `key`: Key used to locate the array within the table.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns a reference to the TOML array when present.
///
/// # Errors
///
/// Returns an error when the key is missing or the value is not an array.
///
fn get_required_array<'a>(table: &'a Table, key: &str, field_name: &str) -> Result<&'a Vec<Value>> {
    match table.get(key) {
        Some(Value::Array(value)) => Ok(value),
        Some(other) => {
            let reason: String =
                format!("{field_name} must be an array (found={})", describe_toml_type(other));
            Err(::anyhow::anyhow!(reason))
        },
        None => {
            let reason: String = format!("missing required array '{field_name}'");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Reads a required string field from the TOML table.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the string.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns the string value when the field exists and is a string.
///
/// # Errors
///
/// Returns an error when the field is missing or not a string.
///
fn read_required_string(table: &Table, key: &str, field_name: &str) -> Result<String> {
    match table.get(key) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(other) => {
            let reason: String =
                format!("{field_name} must be a string (found={})", describe_toml_type(other));
            Err(::anyhow::anyhow!(reason))
        },
        None => {
            let reason: String = format!("missing required field '{field_name}'");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Reads a required string field and ensures it is not empty or whitespace-only.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the string.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns the non-empty string when the field exists and contains characters beyond whitespace.
///
/// # Errors
///
/// Returns an error when the field is missing, not a string, or empty.
///
fn read_required_non_empty_string(table: &Table, key: &str, field_name: &str) -> Result<String> {
    let value: String = read_required_string(table, key, field_name)?;
    if value.trim().is_empty() {
        let reason: String = format!("{field_name} must not be empty");
        return Err(::anyhow::anyhow!(reason));
    }
    Ok(value)
}

///
/// # Description
///
/// Reads a string field that supports a caller-provided default when absent.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the string.
/// - `field_name`: Fully qualified field name used in error messages.
/// - `default_provider`: Closure that supplies the fallback value when the key is missing.
///
/// # Return Value
///
/// Returns the parsed string or the fallback value when the field is absent.
///
/// # Errors
///
/// Returns an error when the field exists but is not a string.
///
fn read_string_with_default<F>(
    table: &Table,
    key: &str,
    field_name: &str,
    default_provider: F,
) -> Result<String>
where
    F: FnOnce() -> String,
{
    match table.get(key) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(other) => {
            let reason: String =
                format!("{field_name} must be a string (found={})", describe_toml_type(other));
            Err(::anyhow::anyhow!(reason))
        },
        None => Ok(default_provider()),
    }
}

///
/// # Description
///
/// Reads the name field for a test case. Autogenerates a default name based
/// on the executor and program path when the field is absent. If the program
/// field is also absent, falls back to using just the executor name.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the string.
/// - `name_field`: Fully qualified field name (e.g., `tests[N].name`).
///
/// # Return Value
///
/// Returns the parsed string or the autogenerated default value when the field is absent.
///
/// # Errors
///
/// Returns an error when the field exists but is not a string, when the executor
/// field is missing, or when `program` is present but cannot be used to derive a name.
///
fn read_optional_name_field(table: &Table, key: &str, name_field: &str) -> Result<String> {
    match read_optional_string(table, key, name_field)? {
        Some(name) => Ok(name),
        None => {
            // name_field is test[N].name, split on the '.' and take the first part as the entry prefix for error messages
            let entry_prefix = name_field.split('.').next().unwrap_or(name_field);
            let executor: &str =
                table
                    .get("executor")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ::anyhow::anyhow!(
                            "cannot derive default name: missing 'executor' in {}",
                            entry_prefix
                        )
                    })?;
            // Only fall back to the executor when 'program' is absent. If 'program'
            // is present but invalid (wrong type or unusable path), return an error
            // so misconfigurations are caught early.
            if let Some(program_value) = table.get("program") {
                let program_str = program_value.as_str().ok_or_else(|| {
                    ::anyhow::anyhow!("'program' in {} must be a string", entry_prefix)
                })?;
                let program = ::std::path::Path::new(program_str)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        ::anyhow::anyhow!(
                            "cannot derive default name: invalid 'program' in {}",
                            entry_prefix
                        )
                    })?;
                Ok(format!("{executor}/{program}"))
            } else {
                Ok(executor.to_string())
            }
        },
    }
}

///
/// # Description
///
/// Reads an optional string field from the TOML table.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the string.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns `Some(String)` when the field exists and is a string; otherwise returns `None` when
/// the field is absent.
///
/// # Errors
///
/// Returns an error when the field exists but is not a string.
///
fn read_optional_string(table: &Table, key: &str, field_name: &str) -> Result<Option<String>> {
    match table.get(key) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(other) => {
            let reason: String =
                format!("{field_name} must be a string (found={})", describe_toml_type(other));
            Err(::anyhow::anyhow!(reason))
        },
        None => Ok(None),
    }
}

///
/// # Description
///
/// Reads an optional 32-bit signed integer from a TOML table.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the integer.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns `Some(i32)` when the field exists and is a valid integer; otherwise returns `None`.
///
/// # Errors
///
/// Returns an error when the field exists but is not an integer or overflows `i32`.
///
fn read_optional_i32(table: &Table, key: &str, field_name: &str) -> Result<Option<i32>> {
    match table.get(key) {
        Some(Value::Integer(value)) => {
            let converted: i32 = match i32::try_from(*value) {
                Ok(v) => v,
                Err(error) => {
                    let reason: String =
                        format!("{field_name} value overflows i32 (value={value}, error={error})");
                    return Err(::anyhow::anyhow!(reason));
                },
            };
            Ok(Some(converted))
        },
        Some(other) => {
            let reason: String =
                format!("{field_name} must be an integer (found={})", describe_toml_type(other));
            Err(::anyhow::anyhow!(reason))
        },
        None => Ok(None),
    }
}

///
/// # Description
///
/// Reads an optional non-negative integer from a TOML table and returns it as `usize`.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the integer.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns `Some(usize)` when the field exists and is a valid non-negative integer; otherwise
/// returns `None`.
///
/// # Errors
///
/// Returns an error when the field exists but is not an integer, is negative, or overflows `usize`.
///
fn read_optional_usize(table: &Table, key: &str, field_name: &str) -> Result<Option<usize>> {
    match table.get(key) {
        Some(value) => {
            let raw: u64 = parse_non_negative_integer(value, field_name)?;
            match usize::try_from(raw) {
                Ok(v) => Ok(Some(v)),
                Err(_) => {
                    let reason: String = format!("{field_name} exceeds usize range (value={raw})");
                    Err(::anyhow::anyhow!(reason))
                },
            }
        },
        None => Ok(None),
    }
}

///
/// # Description
///
/// Reads an optional array of strings from the TOML table.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the array.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns `Some(Vec<String>)` when the field exists and is a valid array of strings; otherwise
/// returns `None` when the field is absent.
///
/// # Errors
///
/// Returns an error when the field exists but is not an array or contains non-string elements.
///
fn read_optional_string_array(
    table: &Table,
    key: &str,
    field_name: &str,
) -> Result<Option<Vec<String>>> {
    match table.get(key) {
        Some(Value::Array(values)) => {
            let mut result: Vec<String> = Vec::with_capacity(values.len());
            for (index, value) in values.iter().enumerate() {
                match value {
                    Value::String(s) => result.push(s.clone()),
                    other => {
                        let reason: String = format!(
                            "{field_name}[{index}] must be a string (found={})",
                            describe_toml_type(other)
                        );
                        return Err(::anyhow::anyhow!(reason));
                    },
                }
            }
            Ok(Some(result))
        },
        Some(other) => {
            let reason: String =
                format!("{field_name} must be an array (found={})", describe_toml_type(other));
            Err(::anyhow::anyhow!(reason))
        },
        None => Ok(None),
    }
}

///
/// # Description
///
/// Reads an optional string and filters out whitespace-only values.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the string.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns `Some(String)` when the field exists and is non-empty; otherwise returns `None`.
///
/// # Errors
///
/// Returns an error when the field exists but is not a string.
///
fn read_optional_non_empty_string(
    table: &Table,
    key: &str,
    field_name: &str,
) -> Result<Option<String>> {
    Ok(read_optional_string(table, key, field_name)?.filter(|value| !value.trim().is_empty()))
}

///
/// # Description
///
/// Reads a boolean field from the TOML table, applying a default when the field is absent.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the boolean.
/// - `field_name`: Fully qualified field name used in error messages.
/// - `default_value`: Value returned when the key is missing.
///
/// # Return Value
///
/// Returns the parsed boolean or the provided default when the field is absent.
///
/// # Errors
///
/// Returns an error when the field exists but is not a boolean.
///
fn read_bool_with_default(
    table: &Table,
    key: &str,
    field_name: &str,
    default_value: bool,
) -> Result<bool> {
    match table.get(key) {
        Some(Value::Boolean(value)) => Ok(*value),
        Some(other) => {
            let reason: String =
                format!("{field_name} must be a boolean (found={})", describe_toml_type(other));
            Err(::anyhow::anyhow!(reason))
        },
        None => Ok(default_value),
    }
}

///
/// # Description
///
/// Reads an unsigned 64-bit integer field from the TOML table, applying a default when absent.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the integer.
/// - `field_name`: Fully qualified field name used in error messages.
/// - `default_value`: Value returned when the key is missing.
///
/// # Return Value
///
/// Returns the parsed `u64` or the default value when the field is absent.
///
/// # Errors
///
/// Returns an error when the field exists but cannot be parsed as a non-negative integer.
///
fn read_u64_with_default(
    table: &Table,
    key: &str,
    field_name: &str,
    default_value: u64,
) -> Result<u64> {
    match table.get(key) {
        Some(value) => {
            let parsed: u64 = parse_non_negative_integer(value, field_name)?;
            Ok(parsed)
        },
        None => Ok(default_value),
    }
}

///
/// # Description
///
/// Reads an unsigned `usize` field from the TOML table, applying a default when absent.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the integer.
/// - `field_name`: Fully qualified field name used in error messages.
/// - `default_value`: Value returned when the key is missing.
///
/// # Return Value
///
/// Returns the parsed `usize` or the default value when the field is absent.
///
/// # Errors
///
/// Returns an error when the field exists but cannot be converted to `usize`.
///
fn read_usize_with_default(
    table: &Table,
    key: &str,
    field_name: &str,
    default_value: usize,
) -> Result<usize> {
    let raw_value: u64 = read_u64_with_default(table, key, field_name, default_value as u64)?;
    match usize::try_from(raw_value) {
        Ok(value) => Ok(value),
        Err(_) => {
            let reason: String = format!("{field_name} exceeds usize range (value={raw_value})");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Reads a required `u16` field from the TOML table.
///
/// # Parameters
///
/// - `table`: Table that stores the target field.
/// - `key`: Key used to retrieve the integer.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns the parsed `u16` value when the field exists and can be converted.
///
/// # Errors
///
/// Returns an error when the field is missing, negative, or outside the `u16` range.
///
fn read_u16_required(table: &Table, key: &str, field_name: &str) -> Result<u16> {
    match table.get(key) {
        Some(value) => {
            let parsed_value: u64 = parse_non_negative_integer(value, field_name)?;
            match u16::try_from(parsed_value) {
                Ok(port) => Ok(port),
                Err(_) => {
                    let reason: String =
                        format!("{field_name} exceeds u16 range (value={parsed_value})");
                    Err(::anyhow::anyhow!(reason))
                },
            }
        },
        None => {
            let reason: String = format!("missing required field '{field_name}'");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Parses a TOML integer value into a non-negative `u64`.
///
/// # Parameters
///
/// - `value`: TOML value inspected for its integer content.
/// - `field_name`: Fully qualified field name used in error messages.
///
/// # Return Value
///
/// Returns the parsed `u64` when the value is a non-negative integer.
///
/// # Errors
///
/// Returns an error when the value is not an integer, is negative, or cannot be represented as
/// `u64`.
///
fn parse_non_negative_integer(value: &Value, field_name: &str) -> Result<u64> {
    match value {
        Value::Integer(raw) => {
            if *raw < 0 {
                let reason: String = format!("{field_name} must be non-negative (value={raw})");
                Err(::anyhow::anyhow!(reason))
            } else {
                match u64::try_from(*raw) {
                    Ok(value) => Ok(value),
                    Err(_) => {
                        let reason: String =
                            format!("{field_name} exceeds u64 range (value={raw})");
                        Err(::anyhow::anyhow!(reason))
                    },
                }
            }
        },
        other => {
            let reason: String =
                format!("{field_name} must be an integer (found={})", describe_toml_type(other));
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Reads the optional `hwloc_file_path` field from the runner table, ignoring empty strings.
///
/// # Parameters
///
/// - `table`: Table that stores the optional hwloc path.
///
/// # Return Value
///
/// Returns `Some(String)` when the field exists and is non-empty; otherwise returns `None`.
///
/// # Errors
///
/// Returns an error when the field exists but is not a string.
///
fn read_hwloc_file_path(table: &Table) -> Result<Option<String>> {
    read_optional_non_empty_string(table, "hwloc_file_path", "runner.hwloc_file_path")
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
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
        table.insert(
            "build_modes".to_string(),
            Value::Array(vec![Value::String("debug".to_string())]),
        );
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
        let table: Table =
            build_test_table("empty", Some(Value::String("not_a_number".to_string())));
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
            build_modes: vec!["debug".to_string()],
            program_env: None,
            program_args_padding_len: None,
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
            build_modes: vec!["debug".to_string()],
            program_env: None,
            program_args_padding_len: None,
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
        assert!(
            result,
            "matches_filter() must return true when glob pattern matches the test name"
        );
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
        assert!(
            result.is_err(),
            "auto-generated name must fail when 'program' is present but invalid"
        );
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
            build_modes: vec!["debug".to_string()],
            program_env: None,
            program_args_padding_len: Some(100),
        };
        let result = config.validate(0);
        assert!(
            result.is_err(),
            "validate() must reject configs with both 'program_args' and \
             'program_args_padding_len'"
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
            build_modes,
            program_env: None,
            program_args_padding_len: None,
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
        assert!(
            config.should_run_in_build_mode("release"),
            "must run under a listed mode (release)"
        );
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
}
