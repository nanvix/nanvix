// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::chrono::Local;
use ::log::error;
use ::std::{
    collections::HashSet,
    fs::{
        create_dir_all,
        read_dir,
        rename,
        write,
    },
    path::{
        Path,
        PathBuf,
    },
    sync::OnceLock,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Component label used for Nanvix Daemon logs.
const RUNNER_COMPONENT: &str = "runner";
/// File prefix applied to guest logs emitted by the sandbox runtime.
const GUEST_LOG_PREFIX: &str = "guest_";
/// File prefix applied to legacy Nanvix Daemon logs.
const NANVIXD_LEGACY_PREFIX: &str = "nanvixd_";
/// File prefix applied to legacy User VM logs.
const USERVM_LEGACY_PREFIX: &str = "uservm_";

/// Component normalization rules indexed by legacy on-disk prefix.
const COMPONENT_NORMALIZATION_RULES: [LegacyComponentRule; 3] = [
    LegacyComponentRule {
        component: "nanvixd",
        legacy_prefix: NANVIXD_LEGACY_PREFIX,
    },
    LegacyComponentRule {
        component: "uservm",
        legacy_prefix: USERVM_LEGACY_PREFIX,
    },
    LegacyComponentRule {
        component: "guest",
        legacy_prefix: GUEST_LOG_PREFIX,
    },
];

//==================================================================================================
// Global Variables
//==================================================================================================

/// Stores the timestamp captured when `nanvix-test` starts running.
static RUN_START_TIMESTAMP: OnceLock<String> = OnceLock::new();

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Captures the filesystem layout used while persisting Nanvix Daemon logs for an individual
/// test case. Each test stores its artifacts under
/// `logs/<timestamp>/<runner-name>/<program-name>/`.
///
pub(crate) struct TestLogLayout {
    /// Directory where all artifacts for the current test case are stored.
    test_dir: PathBuf,
    /// Friendly label used when naming program output logs.
    program_stem: String,
}

///
/// # Description
///
/// Convenience container returned when allocating stdout/stderr files for a Nanvix Daemon run.
///
pub(crate) struct RunnerLogPaths {
    /// Path where Nanvix Daemon stdout is stored.
    pub stdout: PathBuf,
    /// Path where Nanvix Daemon stderr is stored.
    pub stderr: PathBuf,
}

///
/// # Description
///
/// Tracks guest logs created under the shared log root, enabling the runner to relocate new
/// entries into the per-test directory once execution completes.
///
pub(crate) struct GuestLogTracker {
    /// Path to the shared log directory monitored for guest logs.
    log_root: PathBuf,
    /// Set of guest log paths that existed before the tracker was initialized.
    existing_paths: HashSet<PathBuf>,
}

///
/// # Description
///
/// Defines the association between a canonical component name and the historical on-disk prefix
/// produced by existing daemons.
///
struct LegacyComponentRule {
    /// Component label used when emitting normalized filenames.
    component: &'static str,
    /// Legacy on-disk prefix used to detect files that require normalization.
    legacy_prefix: &'static str,
}

///
/// # Description
///
/// Tracks metadata for a legacy log file so entries can be processed in chronological order.
///
struct LegacyLogEntry {
    /// Absolute path to the legacy log file within the test directory.
    path: PathBuf,
    /// Filesystem timestamp used to keep rename operations deterministic.
    modified_at: SystemTime,
}

///
/// # Description
///
/// Wraps the timestamp string inserted into the run-scoped directory hierarchy, allowing
/// consistent formatting across helper functions.
///
struct LogTimestamp {
    formatted: String,
}

impl LogTimestamp {
    ///
    /// # Description
    ///
    /// Generates a timestamp string used when naming the run-scoped directory.
    ///
    /// # Return Value
    ///
    /// Returns a `LogTimestamp` populated with the current local time.
    ///
    fn now() -> Self {
        let formatted: String = Local::now().format("%Y_%m_%d_%H_%M_%S").to_string();
        Self { formatted }
    }

    ///
    /// # Description
    ///
    /// Consumes the timestamp helper and returns the owned string representation.
    ///
    /// # Return Value
    ///
    /// Returns the formatted timestamp captured at construction time.
    ///
    fn into_string(self) -> String {
        self.formatted
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Captures the timestamp used to name the run-scoped directory that stores all log artifacts.
/// This function should be called once, immediately after `nanvix-test` starts running, so all
/// subsequent tests share the same `<timestamp>` component.
///
/// # Return Value
///
/// Returns the cached timestamp string after ensuring it has been initialized.
///
pub(crate) fn initialize_run_timestamp() -> &'static str {
    RUN_START_TIMESTAMP.get_or_init(|| LogTimestamp::now().into_string())
}

///
/// # Description
///
/// Retrieves the cached timestamp component used when building run-scoped directories. This is a
/// thin wrapper around `initialize_run_timestamp()` to emphasize the directory-friendly semantics.
///
/// # Return Value
///
/// Returns the run-scoped timestamp string captured when `nanvix-test` started.
///
fn run_timestamp_component() -> &'static str {
    initialize_run_timestamp()
}

///
/// # Description
///
/// Computes the directory path used to store all logs for the current `nanvix-test` invocation.
/// The run directory lives under the configured base path using the cached timestamp component.
///
/// # Parameters
///
/// - `base_dir`: Root directory configured for storing log artifacts.
///
/// # Return Value
///
/// Returns the absolute path to `base_dir/<timestamp>`.
///
fn run_directory(base_dir: &Path) -> PathBuf {
    base_dir.join(run_timestamp_component())
}

//==================================================================================================
// Implementations
//==================================================================================================

impl TestLogLayout {
    ///
    /// # Description
    ///
    /// Configures a log layout for workloads that provide a concrete program path.
    /// Artifacts are stored under `<base>/<runner-name>/<program-name>` with sanitized path
    /// components to avoid filesystem conflicts.
    ///
    /// # Parameters
    ///
    /// - `base_dir`: Root directory configured by the runner.
    /// - `runner_name`: Logical runner label (e.g., `empty`, `http`, `terminal`).
    /// - `program_path`: Absolute or relative path to the workload under test.
    ///
    /// # Return Value
    ///
    /// Returns a layout rooted at `logs/<timestamp>/<runner-name>/<program-name>` whose files are
    /// named with the sanitized program stem.
    ///
    pub(crate) fn for_program(
        base_dir: &Path,
        runner_name: &str,
        program_path: &str,
    ) -> Result<Self> {
        let program_component_raw: String = match Path::new(program_path).file_name() {
            Some(name) => name.to_string_lossy().into_owned(),
            None => program_path.to_string(),
        };
        let program_component: String = sanitize_component(program_component_raw.as_str());
        let program_stem: String = match Path::new(program_component.as_str()).file_stem() {
            Some(stem) => stem.to_string_lossy().into_owned(),
            None => program_component.clone(),
        };
        let runner_component: String = sanitize_runner_component(runner_name);

        Self::new(
            base_dir,
            runner_component.as_str(),
            program_component.as_str(),
            program_stem.as_str(),
        )
    }

    ///
    /// # Description
    ///
    /// Configures a log layout for workloads that do not expose a concrete binary (e.g., empty
    /// executor sweeps). The sanitized label populates the `logs/<runner-name>/<label>` directory
    /// and program stems.
    ///
    /// # Parameters
    ///
    /// - `base_dir`: Root directory configured by the runner.
    /// - `runner_name`: Logical runner label (e.g., `empty`, `http`, `terminal`).
    /// - `label`: Identifier used when naming the directory and program logs.
    ///
    /// # Return Value
    ///
    /// Returns the configured layout stored under `logs/<timestamp>/<runner-name>/<label>`.
    ///
    pub(crate) fn for_label(base_dir: &Path, runner_name: &str, label: &str) -> Result<Self> {
        let sanitized_label: String = sanitize_component(label);
        let runner_component: String = sanitize_runner_component(runner_name);
        Self::new(
            base_dir,
            runner_component.as_str(),
            sanitized_label.as_str(),
            sanitized_label.as_str(),
        )
    }

    ///
    /// # Description
    ///
    /// Allocates fresh stdout/stderr paths for Nanvix Daemon logs, embedding the run iteration
    /// label to ensure the generated log files remain unique per run.
    ///
    /// # Parameters
    ///
    /// - `iteration`: Optional iteration index (appended to the file name when provided).
    ///
    /// # Return Value
    ///
    /// Returns a `RunnerLogPaths` container with unique stdout/stderr paths.
    ///
    pub(crate) fn allocate_runner_logs(&self, iteration: Option<usize>) -> RunnerLogPaths {
        RunnerLogPaths {
            stdout: self.runner_stdout_path(iteration),
            stderr: self.runner_stderr_path(iteration),
        }
    }

    ///
    /// # Description
    ///
    /// Persists the actual program output captured during the test run, stripping embedded nul
    /// bytes and naming the file with the sanitized program stem plus the iteration label.
    ///
    /// # Parameters
    ///
    /// - `iteration`: Iteration index used to disambiguate file names.
    /// - `payload`: Bytes collected from the workload stdout.
    ///
    /// # Return Value
    ///
    /// Returns the path where the payload was written; returns an error when writing fails.
    ///
    pub(crate) fn persist_program_output(
        &self,
        iteration: usize,
        payload: &[u8],
    ) -> Result<PathBuf> {
        let output_path: PathBuf = self.program_output_path(Some(iteration));
        let sanitized_payload: Vec<u8> =
            payload.iter().copied().filter(|byte| *byte != 0).collect();

        if let Err(error) = write(&output_path, sanitized_payload.as_slice()) {
            let reason: String = format!(
                "failed to write program output log (path={}, error={error})",
                output_path.display()
            );
            error!("persist_program_output(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        }

        Ok(output_path)
    }

    ///
    /// # Description
    ///
    /// Creates a new per-test directory and associates it with the layout metadata.
    ///
    /// # Parameters
    ///
    /// - `base_dir`: Root directory where all test-specific logs reside.
    /// - `runner_component`: Sanitized directory component derived from the runner name.
    /// - `program_component`: Sanitized directory component derived from the workload name.
    /// - `program_stem`: Sanitized stem used when naming program output files.
    ///
    /// # Return Value
    ///
    /// Returns the initialized log layout on success; returns an error if the directory creation
    /// fails.
    ///
    fn new(
        base_dir: &Path,
        runner_component: &str,
        program_component: &str,
        program_stem: &str,
    ) -> Result<Self> {
        let run_dir: PathBuf = run_directory(base_dir);
        let runner_dir: PathBuf = run_dir.join(runner_component);
        let test_dir: PathBuf = runner_dir.join(program_component);

        if let Err(error) = create_dir_all(&test_dir) {
            let reason: String = format!(
                "failed to create log directory (path={}, error={error})",
                test_dir.display()
            );
            error!("new(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        }

        Ok(Self {
            test_dir,
            program_stem: program_stem.to_string(),
        })
    }

    ///
    /// # Description
    ///
    /// Computes the path where Nanvix Daemon stdout logs should be written.
    ///
    /// # Parameters
    ///
    /// - `iteration`: Optional iteration index appended to the filename.
    ///
    /// # Return Value
    ///
    /// Returns the stdout log path scoped to the current test directory.
    ///
    fn runner_stdout_path(&self, iteration: Option<usize>) -> PathBuf {
        let filename: String = component_log_filename(RUNNER_COMPONENT, iteration, "stdout");
        self.test_dir.join(filename)
    }

    ///
    /// # Description
    ///
    /// Computes the path where Nanvix Daemon stderr logs should be written.
    ///
    /// # Parameters
    ///
    /// - `iteration`: Optional iteration index appended to the filename.
    ///
    /// # Return Value
    ///
    /// Returns the stderr log path scoped to the current test directory.
    ///
    fn runner_stderr_path(&self, iteration: Option<usize>) -> PathBuf {
        let filename: String = component_log_filename(RUNNER_COMPONENT, iteration, "stderr");
        self.test_dir.join(filename)
    }

    ///
    /// # Description
    ///
    /// Computes the path where captured program stdout should be persisted.
    ///
    /// # Parameters
    ///
    /// - `iteration`: Optional iteration index appended to the filename.
    ///
    /// # Return Value
    ///
    /// Returns the program output log path scoped to the current test directory.
    ///
    fn program_output_path(&self, iteration: Option<usize>) -> PathBuf {
        let filename: String =
            component_log_filename(self.program_stem.as_str(), iteration, "stdout");
        self.test_dir.join(filename)
    }

    ///
    /// # Description
    ///
    /// Returns the directory that stores all artifacts for the associated test case.
    ///
    /// # Return Value
    ///
    /// Returns the path to `logs/<timestamp>/<runner-name>/<program-name>` managed by this layout
    /// instance.
    ///
    pub(crate) fn test_directory(&self) -> &Path {
        self.test_dir.as_path()
    }

    ///
    /// # Description
    ///
    /// Renames legacy component logs (nanvixd, uservm, guest) so they follow the
    /// `<component>-<stream>-<iter>.log` convention.
    ///
    /// # Parameters
    ///
    /// - `iteration`: Iteration index used when computing the normalized filename.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` after all matching files have been renamed; returns an error if any
    /// rename operation fails.
    ///
    pub(crate) fn normalize_component_logs(&self, iteration: usize) -> Result<()> {
        for rule in COMPONENT_NORMALIZATION_RULES.iter() {
            rename_component_logs(
                self.test_directory(),
                rule.component,
                iteration,
                rule.legacy_prefix,
            )?;
        }

        Ok(())
    }
}

//==================================================================================================
// Helper Functions
//==================================================================================================

///
/// # Description
///
/// Formats an optional iteration index so filenames remain unique across repeated runs.
///
/// # Parameters
///
/// - `iteration`: Optional iteration number associated with the current run.
///
/// # Return Value
///
/// Returns the iteration identifier inserted into filenames, defaulting to `iter000` when
/// unspecified.
///
fn iteration_token(iteration: Option<usize>) -> String {
    let value: usize = iteration.unwrap_or(0);
    format!("iter{value:03}")
}

///
/// # Description
///
/// Normalizes filename segments so hyphens and dots are reserved for separating the component,
/// stream, and iteration tokens or for the `.log` extension.
///
/// # Parameters
///
/// - `segment`: Candidate text inserted into a filename.
///
/// # Return Value
///
/// Returns the sanitized segment where any hyphen or dot is replaced by an underscore.
///
fn normalize_filename_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|ch| match ch {
            '-' | '.' => '_',
            _ => ch,
        })
        .collect()
}

///
/// # Description
///
/// Builds the log filename following the `<component>-<stream>-<iter>.log` convention.
///
/// # Parameters
///
/// - `component`: Sanitized component name (runner, nanvixd, uservm, etc.).
/// - `iteration`: Optional iteration index associated with the artifact.
/// - `stream`: Log stream label (`stdout` or `stderr`).
///
/// # Return Value
///
/// Returns the fully formatted filename.
///
fn component_log_filename(component: &str, iteration: Option<usize>, stream: &str) -> String {
    let component_token: String = normalize_filename_segment(component);
    let stream_token: String = normalize_filename_segment(stream);
    let iteration_raw: String = iteration_token(iteration);
    let iteration_token: String = normalize_filename_segment(iteration_raw.as_str());
    format!("{component_token}-{stream_token}-{iteration_token}.log")
}

///
/// # Description
///
/// Converts arbitrary strings into filesystem-friendly components by replacing unsupported
/// characters with dashes.
///
/// # Parameters
///
/// - `component`: Candidate string that needs sanitization.
///
/// # Return Value
///
/// Returns the sanitized component containing only alphanumeric characters, hyphens, underscores,
/// or dots.
///
fn sanitize_component(component: &str) -> String {
    component
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '-',
        })
        .collect()
}

///
/// # Description
///
/// Sanitizes the runner identifier so it can be used safely as a directory component for log
/// storage.
///
/// # Parameters
///
/// - `runner_identifier`: Text describing the runner type or binary path.
///
/// # Return Value
///
/// Returns a filesystem-safe representation derived from the runner identifier.
///
fn sanitize_runner_component(runner_identifier: &str) -> String {
    match Path::new(runner_identifier).file_stem() {
        Some(stem) => sanitize_component(stem.to_string_lossy().as_ref()),
        None => sanitize_component(runner_identifier),
    }
}

impl GuestLogTracker {
    ///
    /// # Description
    ///
    /// Captures the current set of guest logs so the runner can detect which files were created
    /// during a test case.
    ///
    /// # Parameters
    ///
    /// - `log_root`: Shared log directory scanned for guest log files.
    ///
    /// # Return Value
    ///
    /// Returns a tracker populated with the guest logs present before the test starts.
    ///
    pub(crate) fn capture(log_root: &Path) -> Result<Self> {
        let mut existing_paths: HashSet<PathBuf> = HashSet::new();

        match read_dir(log_root) {
            Err(error) => {
                let reason: String = format!(
                    "failed to scan guest log directory (path={}, error={error})",
                    log_root.display()
                );
                error!("GuestLogTracker::capture(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path: PathBuf = entry.path();
                    if !path.is_file() || !is_guest_log(path.as_path()) {
                        continue;
                    }
                    existing_paths.insert(path);
                }
            },
        }

        Ok(Self {
            log_root: log_root.to_path_buf(),
            existing_paths,
        })
    }

    ///
    /// # Description
    ///
    /// Moves any guest logs created after the tracker was initialized into the provided
    /// destination directory.
    ///
    /// # Parameters
    ///
    /// - `destination_dir`: Per-test directory where the guest logs should reside.
    ///
    /// # Return Value
    ///
    /// Returns the list of destination paths populated with guest logs.
    ///
    pub(crate) fn move_new_logs(&self, destination_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut moved_paths: Vec<PathBuf> = Vec::new();

        let entries = match read_dir(self.log_root.as_path()) {
            Err(error) => {
                let reason: String = format!(
                    "failed to scan guest log directory (path={}, error={error})",
                    self.log_root.display()
                );
                error!("GuestLogTracker::move_new_logs(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
            Ok(entries) => entries,
        };

        for entry in entries.flatten() {
            let source_path: PathBuf = entry.path();
            if !source_path.is_file() || !is_guest_log(source_path.as_path()) {
                continue;
            }
            if self.existing_paths.contains(&source_path) {
                continue;
            }
            let Some(file_name) = source_path.file_name() else {
                continue;
            };
            let destination_path: PathBuf = destination_dir.join(file_name);
            if let Err(error) = rename(source_path.as_path(), destination_path.as_path()) {
                let reason: String = format!(
                    "failed to move guest log (source={}, destination={}, error={error})",
                    source_path.display(),
                    destination_path.display()
                );
                error!("GuestLogTracker::move_new_logs(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            }
            moved_paths.push(destination_path);
        }

        Ok(moved_paths)
    }
}

///
/// # Description
///
/// Determines whether the provided path points to a guest log file.
///
/// # Parameters
///
/// - `path`: Filesystem path inspected for the guest log prefix.
///
/// # Return Value
///
/// Returns `true` when the filename begins with the guest log prefix; otherwise returns `false`.
///
fn is_guest_log(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };
    let Some(file_str) = file_name.to_str() else {
        return false;
    };

    file_str.starts_with(GUEST_LOG_PREFIX)
}

///
/// # Description
///
/// Renames legacy component logs so they adhere to the standardized log naming convention.
///
/// # Parameters
///
/// - `test_dir`: Directory that stores the artifacts for the current test case.
/// - `component`: Canonical component name (e.g., `nanvixd`).
/// - `iteration`: Iteration index recorded in the final filename.
/// - `legacy_prefix`: On-disk prefix used by the legacy component logs.
///
/// # Return Value
///
/// Returns `Ok(())` if all detected files were normalized successfully; otherwise returns an
/// error describing the failure.
///
fn rename_component_logs(
    test_dir: &Path,
    component: &str,
    iteration: usize,
    legacy_prefix: &str,
) -> Result<()> {
    let mut legacy_entries: Vec<LegacyLogEntry> = Vec::new();

    let entries = match read_dir(test_dir) {
        Err(error) => {
            let reason: String = format!(
                "failed to scan test directory (path={}, error={error})",
                test_dir.display()
            );
            error!("rename_component_logs(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        },
        Ok(entries) => entries,
    };

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        let file_name_raw = entry.file_name();
        let Some(file_name) = file_name_raw.to_str() else {
            continue;
        };
        if !file_name.starts_with(legacy_prefix) || !file_name.ends_with(".log") {
            continue;
        }

        let modified_at: SystemTime = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH);

        legacy_entries.push(LegacyLogEntry {
            path: entry.path(),
            modified_at,
        });
    }

    if legacy_entries.is_empty() {
        return Ok(());
    }

    legacy_entries.sort_by(|left, right| {
        left.modified_at
            .cmp(&right.modified_at)
            .then_with(|| left.path.cmp(&right.path))
    });

    for (index, entry) in legacy_entries.into_iter().enumerate() {
        let stream_label: &str = entry
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .map(stream_label_from_legacy_filename)
            .unwrap_or("stdout");
        let base_filename: String =
            component_log_filename(component, Some(iteration), stream_label);
        let candidate_name: String = if index == 0 {
            base_filename
        } else {
            format!("{base_filename}.{index}")
        };
        let mut destination_path: PathBuf = test_dir.join(candidate_name.clone());
        let mut collision_suffix: usize = 1;
        while destination_path.exists() {
            let duplicate_name: String = format!("{candidate_name}.{collision_suffix}");
            destination_path = test_dir.join(duplicate_name);
            collision_suffix += 1;
        }

        if let Err(error) = rename(entry.path.as_path(), destination_path.as_path()) {
            let reason: String = format!(
                "failed to rename component log (source={}, destination={}, error={error})",
                entry.path.display(),
                destination_path.display()
            );
            error!("rename_component_logs(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        }
    }

    Ok(())
}

///
/// # Description
///
/// Infers whether a legacy component log captured stdout or stderr based on its filename.
///
/// # Parameters
///
/// - `file_name`: Filename that still uses the legacy prefix-based convention.
///
/// # Return Value
///
/// Returns `stderr` when the filename contains the substring `stderr` (case-insensitive);
/// otherwise defaults to `stdout`.
///
fn stream_label_from_legacy_filename(file_name: &str) -> &'static str {
    let lowercase: String = file_name.to_ascii_lowercase();
    if lowercase.contains("stderr") {
        "stderr"
    } else {
        "stdout"
    }
}
