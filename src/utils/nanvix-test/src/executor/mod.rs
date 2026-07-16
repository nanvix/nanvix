// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod common;
pub mod empty;
pub mod http;
pub mod snapshot_restore;
pub mod snapshot_save_exit;
pub mod terminal;
pub(crate) use self::common::drain_stream;

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Describes workload metadata forwarded to Nanvix executors.
///
#[derive(Clone, Copy)]
pub struct WorkloadSpec<'a> {
    ///
    /// # Description
    ///
    /// Path to the workload binary executed by an executor.
    ///
    program_path: &'a str,
    ///
    /// # Description
    ///
    /// Optional argument string forwarded to the workload entry point.
    ///
    program_args: Option<&'a str>,
    ///
    /// # Description
    ///
    /// Optional environment variable string forwarded to the workload.
    /// Formatted as space-separated `KEY=VALUE` pairs.
    ///
    program_env: Option<&'a str>,
    ///
    /// # Description
    ///
    /// Optional payload injected into the workload stdin or HTTP stream.
    ///
    input: Option<&'a str>,
    ///
    /// # Description
    ///
    /// Optional substring that must appear in the collected stdout payload.
    ///
    expected_output: Option<&'a str>,
    ///
    /// # Description
    ///
    /// Indicates whether the workload is expected to produce an empty stdout payload.
    ///
    expect_empty_output: bool,
    ///
    /// # Description
    ///
    /// Optional expected exit code that the workload must produce.
    ///
    expected_exit_code: Option<i32>,
}

impl<'a> WorkloadSpec<'a> {
    ///
    /// # Description
    ///
    /// Creates a new workload specification used by Nanvix executors.
    ///
    /// # Parameters
    ///
    /// - `program_path`: Path to the workload binary executed by an executor.
    /// - `program_args`: Optional argument string forwarded to the workload entry point.
    /// - `program_env`: Optional environment variable string forwarded to the workload.
    /// - `input`: Optional payload injected into the workload stdin or HTTP stream.
    /// - `expected_output`: Optional substring that must appear in the collected stdout payload.
    /// - `expect_empty_output`: Indicates whether the workload should produce an empty stdout
    ///   payload.
    /// - `expected_exit_code`: Optional exit code that the workload must produce.
    ///
    /// # Return Value
    ///
    /// Returns a workload specification containing the provided metadata.
    pub const fn new(
        program_path: &'a str,
        program_args: Option<&'a str>,
        program_env: Option<&'a str>,
        input: Option<&'a str>,
        expected_output: Option<&'a str>,
        expect_empty_output: bool,
        expected_exit_code: Option<i32>,
    ) -> Self {
        Self {
            program_path,
            program_args,
            program_env,
            input,
            expected_output,
            expect_empty_output,
            expected_exit_code,
        }
    }

    ///
    /// # Description
    ///
    /// Retrieves the path to the workload binary executed by an executor.
    ///
    /// # Return Value
    ///
    /// Returns the workload binary path.
    pub const fn program_path(&self) -> &'a str {
        self.program_path
    }

    ///
    /// # Description
    ///
    /// Retrieves the optional argument string forwarded to the workload entry point.
    ///
    /// # Return Value
    ///
    /// Returns the optional argument string, when provided.
    pub const fn program_args(&self) -> Option<&'a str> {
        self.program_args
    }

    ///
    /// # Description
    ///
    /// Retrieves the optional environment variable string forwarded to the workload.
    ///
    /// # Return Value
    ///
    /// Returns the optional environment variable string, when provided.
    pub const fn program_env(&self) -> Option<&'a str> {
        self.program_env
    }

    ///
    /// # Description
    ///
    /// Retrieves the optional payload injected into the workload stdin or HTTP stream.
    ///
    /// # Return Value
    ///
    /// Returns the optional payload, when provided.
    pub const fn input(&self) -> Option<&'a str> {
        self.input
    }

    ///
    /// # Description
    ///
    /// Retrieves the optional substring that must appear in the collected stdout payload.
    ///
    /// # Return Value
    ///
    /// Returns the optional expected stdout substring, when provided.
    pub const fn expected_output(&self) -> Option<&'a str> {
        self.expected_output
    }

    ///
    /// # Description
    ///
    /// Indicates whether the workload should produce an empty stdout payload.
    ///
    /// # Return Value
    ///
    /// Returns `true` when empty stdout is required; otherwise returns `false`.
    pub const fn expect_empty_output(&self) -> bool {
        self.expect_empty_output
    }

    ///
    /// # Description
    ///
    /// Retrieves the expected exit code that the workload must produce.
    ///
    /// # Return Value
    ///
    /// Returns the expected exit code when specified; defaults to `0` (success) when not set.
    ///
    pub const fn expected_exit_code(&self) -> i32 {
        match self.expected_exit_code {
            Some(code) => code,
            None => 0,
        }
    }
}

//==================================================================================================
// Enumerations
//==================================================================================================

/// Executor variants supported by the Nanvix test runner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutorName {
    /// Empty executor.
    Empty,
    /// HTTP executor.
    Http,
    /// Snapshot save / restore executor.
    SnapshotRestore,
    /// Snapshot save / host-exit executor.
    SnapshotSaveExit,
    /// Terminal executor.
    Terminal,
}

impl ExecutorName {
    ///
    /// # Description
    ///
    /// Parses a textual identifier into the corresponding executor.
    ///
    /// # Parameters
    ///
    /// - `identifier`: Executor label read from the configuration file.
    ///
    /// # Return Value
    ///
    /// Returns the matching executor variant when the identifier is supported; returns an error
    /// when the identifier is invalid.
    pub fn from_str(identifier: &str) -> Result<Self> {
        match identifier {
            "empty" => Ok(Self::Empty),
            "http" => Ok(Self::Http),
            "snapshot-restore" => Ok(Self::SnapshotRestore),
            "snapshot-save-exit" => Ok(Self::SnapshotSaveExit),
            "terminal" => Ok(Self::Terminal),
            _ => Err(::anyhow::anyhow!(format!("invalid executor name '{identifier}'"))),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the canonical directory label associated with the executor variant.
    ///
    /// # Return Value
    ///
    /// Returns one of `empty`, `http`, `terminal`, or (under the `standalone` feature)
    /// `snapshot-restore` for use when organizing logs.
    ///
    pub const fn to_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Http => "http",
            Self::SnapshotRestore => "snapshot-restore",
            Self::SnapshotSaveExit => "snapshot-save-exit",
            Self::Terminal => "terminal",
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds a combined argument string using the documented `<args>;<env>` format.
///
/// When environment variables are present, they are appended after a `;` separator so that
/// the kernel's `split_cmdline()` can split them. When only one of args or env is present,
/// the appropriate prefix or suffix is used. Any literal `;` in either field is escaped to
/// `\;` so that `split_cmdline()` treats it as data rather than the separator.
///
/// Empty strings are normalised to absent (`None`) so that `Some("")` behaves identically
/// to `None`.
///
/// # Contract
///
/// Both `args` and `env` must be raw (unescaped) strings. The function performs all necessary
/// escaping for the `split_cmdline()` wire format. Do not pre-escape `;` in the input — a
/// literal `\;` in the input represents a raw backslash followed by a raw semicolon, and the
/// round-trip through `split_cmdline()` will preserve both characters.
///
/// # Parameters
///
/// - `args`: Optional command-line argument string.
/// - `env`: Optional environment variable string (space-separated `KEY=VALUE` pairs).
///
/// # Return Value
///
/// Returns the combined string ready to be passed as `program_args`.
///
pub fn combine_args_env(args: Option<&str>, env: Option<&str>) -> String {
    // Normalise empty strings to absent so that `Some("")` and `None` behave identically.
    let args: &str = args.unwrap_or("");
    let env: &str = env.unwrap_or("");

    // Always escape literal `;` in args and env so that split_cmdline()
    // never mistakes them for the args/env separator.
    //
    // A raw `\;` in the input becomes `\\;` after escaping. split_cmdline()
    // interprets `\\;` as: literal `\` (next char is `\`, not `;`) followed
    // by `\;` escape → `;`, yielding the original `\;`. This is correct
    // because the input is always raw (unescaped).
    let escaped_args: String = args.replace(';', "\\;");
    let escaped_env: String = env.replace(';', "\\;");

    if escaped_env.is_empty() {
        escaped_args
    } else if escaped_args.is_empty() {
        format!(";{escaped_env}")
    } else {
        format!("{escaped_args};{escaped_env}")
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workload_spec_expected_exit_code_some() {
        let spec: WorkloadSpec =
            WorkloadSpec::new("./bin/test.elf", None, None, None, None, true, Some(0));
        assert_eq!(spec.expected_exit_code(), 0);
    }

    #[test]
    fn workload_spec_expected_exit_code_none() {
        let spec: WorkloadSpec =
            WorkloadSpec::new("./bin/test.elf", None, None, None, None, false, None);
        assert_eq!(spec.expected_exit_code(), 0);
    }

    #[test]
    fn workload_spec_expected_exit_code_nonzero() {
        let spec: WorkloadSpec =
            WorkloadSpec::new("./bin/test.elf", None, None, None, None, true, Some(13));
        assert_eq!(spec.expected_exit_code(), 13);
    }

    #[test]
    fn workload_spec_expected_exit_code_negative() {
        let spec: WorkloadSpec =
            WorkloadSpec::new("./bin/test.elf", None, None, None, None, false, Some(-1));
        assert_eq!(spec.expected_exit_code(), -1);
    }

    #[test]
    fn combine_args_env_no_args_no_env() {
        assert_eq!(combine_args_env(None, None), "");
    }

    #[test]
    fn combine_args_env_args_only() {
        assert_eq!(combine_args_env(Some("arg1 arg2"), None), "arg1 arg2");
    }

    #[test]
    fn combine_args_env_env_only() {
        assert_eq!(combine_args_env(None, Some("VAR=x")), ";VAR=x");
    }

    #[test]
    fn combine_args_env_args_and_env() {
        assert_eq!(combine_args_env(Some("arg1"), Some("VAR=x")), "arg1;VAR=x");
    }

    #[test]
    fn combine_args_env_escapes_semicolons_in_args() {
        assert_eq!(combine_args_env(Some("a;b"), Some("VAR=x")), "a\\;b;VAR=x");
    }

    #[test]
    fn combine_args_env_escapes_semicolons_in_env() {
        assert_eq!(combine_args_env(Some("arg1"), Some("PATH=a;b")), "arg1;PATH=a\\;b");
    }

    #[test]
    fn combine_args_env_escapes_semicolons_even_without_env() {
        assert_eq!(combine_args_env(Some("a;b"), None), "a\\;b");
    }

    #[test]
    fn combine_args_env_empty_env_string_treated_as_absent() {
        assert_eq!(combine_args_env(Some("arg1"), Some("")), "arg1");
    }

    #[test]
    fn combine_args_env_empty_args_string_treated_as_absent() {
        assert_eq!(combine_args_env(Some(""), Some("VAR=x")), ";VAR=x");
    }

    #[test]
    fn combine_args_env_preserves_backslash_semicolon_in_raw_input() {
        // Raw input `\;` (literal backslash + literal semicolon) must be preserved.
        // The `;` is escaped to `\;`, producing `\\;` in the encoded output.
        // split_cmdline() interprets `\\;` as: literal `\` (next char is `\`, not `;`)
        // followed by `\;` escape → `;`. Round-trip result: `\;` — original preserved.
        assert_eq!(combine_args_env(Some("a\\;b"), None), "a\\\\;b");
    }
}
