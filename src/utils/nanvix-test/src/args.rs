// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::globset::{
    Glob,
    GlobSet,
    GlobSetBuilder,
};
use ::log::error;
use ::std::process;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// CLI arguments resolved for the Nanvix test utility entrypoint.
pub struct Args {
    /// Path to the Nanvix test configuration file supplied on the command line.
    config_file_path: String,
    /// Optional test filter to select specific tests to run.
    test_filter: Option<String>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    const OPT_HELP: &'static str = "-help";
    const OPT_TEST_SELECTOR: &'static str = "-test";

    ///
    /// # Description
    ///
    /// Displays the command-line usage information for the Nanvix test utility.
    ///
    /// # Parameters
    ///
    /// - `program_name`: Executable name printed in the usage banner.
    ///
    /// # Return Value
    ///
    /// Returns immediately after printing the usage banner; does not fail.
    ///
    fn usage(program_name: &str) {
        println!(
            "\
Nanvix Test Utility - Helper tool for testing Nanvix.

Usage:
    {program_name} [OPTIONS] <config-file>

Options:
    {help}                   Show this help message and exit.
    {filter} <pattern>         Specify a comma-separated list or a matching pattern of test(s) to \
             run (e.g., '-test http/*' to run all tests in the 'http' executor).

Required positional arguments:
    config-file             Path to the TOML configuration that describes the runner and test case.
",
            program_name = program_name,
            help = Self::OPT_HELP,
            filter = Self::OPT_TEST_SELECTOR,
        );
    }

    ///
    /// # Description
    ///
    /// Parses the CLI arguments and produces the resolved configuration options.
    ///
    /// # Parameters
    ///
    /// - `args`: Vector containing the raw arguments provided to the program.
    ///
    /// # Return Value
    ///
    /// Returns an `Args` instance when the CLI input is valid; returns an error when the
    /// configuration file argument is missing, misordered, or an unexpected flag appears.
    ///
    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut config_file_path: Option<String> = None;
        let mut test_filter_string: Option<String> = None;

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    process::exit(0);
                },
                Self::OPT_TEST_SELECTOR => {
                    if i + 1 < args.len() {
                        test_filter_string = Some(args[i + 1].to_string());
                        i += 1; // Skip the next argument since it's the filter pattern
                    } else {
                        let reason: String = "missing argument for option '-test': expected a \
                                              comma-separated list of test names or a pattern"
                            .to_string();
                        Self::usage(args[0].as_str());
                        eprintln!("parse(): {reason}");
                        return Err(::anyhow::anyhow!(reason));
                    }
                },
                argument => {
                    if i == args.len() - 1 {
                        config_file_path = Some(argument.to_string());
                    } else {
                        let reason: String = format!(
                            "invalid argument order: configuration file must be the last argument \
                             (found '{argument}')"
                        );
                        Self::usage(args[0].as_str());
                        eprintln!("parse(): {reason}");
                        return Err(::anyhow::anyhow!(reason));
                    }
                },
            }
            i += 1;
        }

        let config_file_path: String = match config_file_path {
            Some(path) => path,
            None => {
                let reason: String =
                    "missing required positional argument: <config-file>".to_string();
                Self::usage(args[0].as_str());
                eprintln!("parse(): {reason}");
                return Err(::anyhow::anyhow!(reason));
            },
        };

        let test_filter: Option<String> = test_filter_string.map(|s| s.trim().to_string());

        Ok(Self {
            config_file_path,
            test_filter,
        })
    }

    ///
    /// # Description
    ///
    /// Retrieves the path to the TOML configuration file supplied via the CLI.
    ///
    /// # Return Value
    ///
    /// Returns the configuration file path.
    ///
    pub fn config_file_path(&self) -> &str {
        self.config_file_path.as_str()
    }

    ///
    /// # Description
    ///
    /// Retrieves the test filter specified via the CLI.
    ///
    /// # Return Value
    ///
    /// Returns the test filter string, if any.
    ///
    pub fn test_filter(&self) -> Option<&str> {
        self.test_filter.as_deref()
    }

    ///
    /// # Description
    ///
    /// Constructs a `GlobSet` from the test filter string, if provided. The filter string can be a
    /// comma-separated list of test names or patterns.
    ///
    /// # Return Value
    ///
    /// Returns the `GlobSet` if a test filter is specified; otherwise returns `None`.
    ///
    pub fn glob_filter(&self) -> Option<GlobSet> {
        self.test_filter
            .as_ref()
            .map(|filter_str: &String| build_globset(filter_str))
    }
}

//==================================================================================================
// Helper Functions
//==================================================================================================

///
/// # Description
///
/// Constructs a `GlobSet` from `build_globset_internal` that wraps GlobSetBuilder::build.
/// Returns the `GlobSet` if successful; otherwise logs the error and exits the program with a
/// non-zero status code.
///
/// # Parameters
///
/// - `filter`: Comma-separated list of test names or patterns.
///
/// # Return Value
///
/// Returns a `GlobSet` containing the compiled patterns.
///
fn build_globset(filter: &str) -> GlobSet {
    match build_globset_internal(filter) {
        Ok(globset) => globset,
        Err(message) => {
            error!("{message}");
            process::exit(1);
        },
    }
}

///
/// # Description
///
/// Constructs a `GlobSet` from the test filter string, which can be a comma-separated list of test
/// names or patterns.
///
/// # Parameters
///
/// - `filter`: Comma-separated list of test names or patterns.
///
/// # Return Value
///
/// Returns a `GlobSet` containing the compiled patterns.
///
fn build_globset_internal(filter: &str) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in filter.split(',') {
        let trimmed = pattern.trim();
        if trimmed.is_empty() {
            continue;
        }
        let glob =
            Glob::new(trimmed).map_err(|err| format!("Invalid glob pattern '{trimmed}': {err}"))?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|err| format!("Failed to build glob set from filter '{filter}': {err}"))
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::build_globset_internal;
    #[test]
    fn single_pattern_matching() {
        let filter: &str = "http/*";
        let globset =
            build_globset_internal(filter).expect("Failed to build glob set for a single pattern");
        assert!(globset.is_match("http/test1"));
        assert!(!globset.is_match("db/test1"));
    }
    #[test]
    fn comma_separated_pattern_list_matching() {
        let filter: &str = "http/*,db/*";
        let globset = build_globset_internal(filter)
            .expect("Failed to build glob set for comma-separated patterns");
        assert!(globset.is_match("http/test1"));
        assert!(globset.is_match("db/test1"));
        assert!(!globset.is_match("cache/test1"));
    }
    #[test]
    fn whitespace_handling_in_patterns() {
        let filter: &str = "  http/*  ,   db/*  ";
        let globset = build_globset_internal(filter)
            .expect("Failed to build glob set with whitespace in filter");
        assert!(globset.is_match("http/test2"));
        assert!(globset.is_match("db/test2"));
        assert!(!globset.is_match("cache/test2"));
    }
    #[test]
    fn empty_filter_handling() {
        let empty_filter: &str = "";
        let globset_empty = build_globset_internal(empty_filter)
            .expect("Failed to build glob set for empty filter");
        assert!(!globset_empty.is_match("any/test"));
        let whitespace_filter: &str = "  ,   ,  ";
        let globset_whitespace = build_globset_internal(whitespace_filter)
            .expect("Failed to build glob set for whitespace-only filter");
        assert!(!globset_whitespace.is_match("any/test"));
    }
    #[test]
    fn invalid_glob_pattern_returns_error() {
        let filter: &str = "[";
        let err = build_globset_internal(filter)
            .expect_err("Expected invalid glob pattern to return an error");
        assert!(err.contains("Invalid glob pattern '['"));
    }
}
