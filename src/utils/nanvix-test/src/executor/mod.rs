// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod empty;
#[cfg(unix)]
pub mod http;
pub mod terminal;

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
        input: Option<&'a str>,
        expected_output: Option<&'a str>,
        expect_empty_output: bool,
        expected_exit_code: Option<i32>,
    ) -> Self {
        Self {
            program_path,
            program_args,
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
    /// Returns one of `empty`, `http`, or `terminal` for use when organizing logs.
    ///
    pub const fn to_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Http => "http",
            Self::Terminal => "terminal",
        }
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
            WorkloadSpec::new("./bin/test.elf", None, None, None, true, Some(0));
        assert_eq!(spec.expected_exit_code(), 0);
    }

    #[test]
    fn workload_spec_expected_exit_code_none() {
        let spec: WorkloadSpec = WorkloadSpec::new("./bin/test.elf", None, None, None, false, None);
        assert_eq!(spec.expected_exit_code(), 0);
    }

    #[test]
    fn workload_spec_expected_exit_code_nonzero() {
        let spec: WorkloadSpec =
            WorkloadSpec::new("./bin/test.elf", None, None, None, true, Some(13));
        assert_eq!(spec.expected_exit_code(), 13);
    }

    #[test]
    fn workload_spec_expected_exit_code_negative() {
        let spec: WorkloadSpec =
            WorkloadSpec::new("./bin/test.elf", None, None, None, false, Some(-1));
        assert_eq!(spec.expected_exit_code(), -1);
    }
}
