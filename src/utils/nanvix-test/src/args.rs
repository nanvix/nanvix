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
/// Partition selector that assigns a disjoint, round-robin subset of the filtered test list to a
/// single parallel shard. Sharding is positional, so it is independent of test names and
/// automatically distributes newly added tests across shards.
#[derive(Clone, Copy)]
pub struct Shard {
    /// Zero-based index of this shard within `total`.
    index: usize,
    /// Total number of shards the test list is partitioned into.
    total: usize,
}

impl Shard {
    ///
    /// # Description
    ///
    /// Determines whether the test at `position` (zero-based, within the filtered list) is assigned
    /// to this shard. Round-robin assignment keeps shards balanced as tests are added.
    ///
    /// # Parameters
    ///
    /// - `position`: Zero-based index of the test within the filtered list.
    ///
    /// # Return Value
    ///
    /// Returns `true` when the test belongs to this shard; otherwise returns `false`.
    ///
    pub fn selects(&self, position: usize) -> bool {
        position % self.total == self.index
    }

    ///
    /// # Description
    ///
    /// Retrieves the one-based index of this shard, suitable for display.
    ///
    /// # Return Value
    ///
    /// Returns the one-based shard index.
    ///
    pub fn index(&self) -> usize {
        self.index + 1
    }

    ///
    /// # Description
    ///
    /// Retrieves the total number of shards.
    ///
    /// # Return Value
    ///
    /// Returns the total shard count.
    ///
    pub fn total(&self) -> usize {
        self.total
    }
}

///
/// # Description
///
/// CLI arguments resolved for the Nanvix test utility entrypoint.
pub struct Args {
    /// Path to the Nanvix test configuration file supplied on the command line.
    config_file_path: String,
    /// Optional test filter to select specific tests to run.
    test_filter: Option<String>,
    /// Optional shard selector that partitions the filtered tests across parallel runners.
    shard: Option<Shard>,
    /// When true, list selected tests and exit without running them.
    list: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    const OPT_HELP: &'static str = "-help";
    const OPT_TEST_SELECTOR: &'static str = "-test";
    const OPT_SHARD: &'static str = "-shard";
    const OPT_LIST: &'static str = "-list";

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
    {shard} <INDEX/TOTAL>    Run only shard INDEX of TOTAL (1-based), partitioning the selected \
             tests round-robin across parallel runners (e.g., '-shard 1/4').
    {list}                   List selected tests and exit without running them.

Required positional arguments:
    config-file             Path to the TOML configuration that describes the runner and test case.
",
            program_name = program_name,
            help = Self::OPT_HELP,
            filter = Self::OPT_TEST_SELECTOR,
            shard = Self::OPT_SHARD,
            list = Self::OPT_LIST,
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
        let mut shard: Option<Shard> = None;
        let mut list: bool = false;

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    process::exit(0);
                },
                Self::OPT_LIST => {
                    list = true;
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
                Self::OPT_SHARD => {
                    if i + 1 < args.len() {
                        match Self::parse_shard(args[i + 1].as_str()) {
                            Ok(parsed_shard) => shard = Some(parsed_shard),
                            Err(reason) => {
                                Self::usage(args[0].as_str());
                                eprintln!("parse(): {reason}");
                                return Err(::anyhow::anyhow!(reason));
                            },
                        }
                        i += 1; // Skip the next argument since it's the shard selector
                    } else {
                        let reason: String = "missing argument for option '-shard': expected \
                                              INDEX/TOTAL (e.g. '1/4')"
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
            shard,
            list,
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
    /// Retrieves the shard selector specified via the CLI.
    ///
    /// # Return Value
    ///
    /// Returns the shard selector when `-shard` was provided; otherwise returns `None`.
    ///
    pub fn shard(&self) -> Option<Shard> {
        self.shard
    }

    ///
    /// # Description
    ///
    /// Returns whether the `-list` flag was specified on the command line.
    ///
    /// # Return Value
    ///
    /// Returns `true` when the user requested test listing without execution.
    ///
    pub fn list(&self) -> bool {
        self.list
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

    ///
    /// # Description
    ///
    /// Parses a `-shard` selector of the form `INDEX/TOTAL` (1-based index).
    ///
    /// # Parameters
    ///
    /// - `value`: Raw selector string supplied on the command line.
    ///
    /// # Return Value
    ///
    /// Returns the parsed `Shard` when the selector is well formed; otherwise returns an error
    /// message describing the problem.
    ///
    fn parse_shard(value: &str) -> Result<Shard, String> {
        let trimmed: &str = value.trim();
        let (index_str, total_str) = trimmed.split_once('/').ok_or_else(|| {
            format!("invalid '-shard' value '{trimmed}': expected INDEX/TOTAL (e.g. '1/4')")
        })?;
        let index: usize = index_str.trim().parse().map_err(|_| {
            format!(
                "invalid shard index '{}' in '-shard {trimmed}': expected a positive integer",
                index_str.trim()
            )
        })?;
        let total: usize = total_str.trim().parse().map_err(|_| {
            format!(
                "invalid shard total '{}' in '-shard {trimmed}': expected a positive integer",
                total_str.trim()
            )
        })?;
        if total == 0 {
            return Err(format!("invalid '-shard {trimmed}': TOTAL must be greater than zero"));
        }
        if index == 0 || index > total {
            return Err(format!(
                "invalid '-shard {trimmed}': INDEX must be in the range 1..={total}"
            ));
        }
        Ok(Shard {
            index: index - 1,
            total,
        })
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
    #[test]
    fn list_flag() {
        let args = vec![
            "nanvix-test".to_string(),
            "-list".to_string(),
            "test/test-standalone.toml".to_string(),
        ];
        let parsed = super::Args::parse(args).expect("Failed to parse -list flag");
        assert!(parsed.list(), "-list flag must set list to true");
        assert_eq!(parsed.config_file_path(), "test/test-standalone.toml");
    }
    #[test]
    fn list_flag_combined_with_test_filter() {
        let args = vec![
            "nanvix-test".to_string(),
            "-list".to_string(),
            "-test".to_string(),
            "http/*".to_string(),
            "test/test-standalone.toml".to_string(),
        ];
        let parsed = super::Args::parse(args).expect("Failed to parse -list with -test filter");
        assert!(parsed.list(), "-list flag must be set");
        assert_eq!(parsed.test_filter(), Some("http/*"));
        assert_eq!(parsed.config_file_path(), "test/test-standalone.toml");
    }
    #[test]
    fn no_list_flag_defaults_to_false() {
        let args = vec![
            "nanvix-test".to_string(),
            "test/test-standalone.toml".to_string(),
        ];
        let parsed = super::Args::parse(args).expect("Failed to parse args without -list");
        assert!(!parsed.list(), "list must default to false");
    }
    #[test]
    fn shard_flag_parses_valid_value() {
        let args = vec![
            "nanvix-test".to_string(),
            "-shard".to_string(),
            "2/4".to_string(),
            "test/test-standalone.toml".to_string(),
        ];
        let parsed = super::Args::parse(args).expect("Failed to parse -shard flag");
        let shard = parsed.shard().expect("shard must be set");
        assert_eq!(shard.index(), 2, "one-based shard index must be 2");
        assert_eq!(shard.total(), 4, "shard total must be 4");
        assert_eq!(parsed.config_file_path(), "test/test-standalone.toml");
    }
    #[test]
    fn shard_flag_defaults_to_none() {
        let args = vec![
            "nanvix-test".to_string(),
            "test/test-standalone.toml".to_string(),
        ];
        let parsed = super::Args::parse(args).expect("Failed to parse args without -shard");
        assert!(parsed.shard().is_none(), "shard must default to None");
    }
    #[test]
    fn shard_selects_round_robin() {
        let args = vec![
            "nanvix-test".to_string(),
            "-shard".to_string(),
            "1/4".to_string(),
            "test/test-standalone.toml".to_string(),
        ];
        let parsed = super::Args::parse(args).expect("Failed to parse -shard flag");
        let shard = parsed.shard().expect("shard must be set");
        // Shard 1 of 4 (zero-based index 0) selects positions 0, 4, 8, ...
        assert!(shard.selects(0));
        assert!(!shard.selects(1));
        assert!(!shard.selects(3));
        assert!(shard.selects(4));
    }
    #[test]
    fn shard_flag_rejects_zero_total() {
        let args = vec![
            "nanvix-test".to_string(),
            "-shard".to_string(),
            "1/0".to_string(),
            "test/test-standalone.toml".to_string(),
        ];
        assert!(super::Args::parse(args).is_err(), "zero TOTAL must be rejected");
    }
    #[test]
    fn shard_flag_rejects_index_out_of_range() {
        let args = vec![
            "nanvix-test".to_string(),
            "-shard".to_string(),
            "5/4".to_string(),
            "test/test-standalone.toml".to_string(),
        ];
        assert!(super::Args::parse(args).is_err(), "INDEX greater than TOTAL must be rejected");
    }
    #[test]
    fn shard_flag_rejects_zero_index() {
        let args = vec![
            "nanvix-test".to_string(),
            "-shard".to_string(),
            "0/4".to_string(),
            "test/test-standalone.toml".to_string(),
        ];
        assert!(super::Args::parse(args).is_err(), "zero INDEX must be rejected");
    }
    #[test]
    fn shard_flag_rejects_malformed_value() {
        let args = vec![
            "nanvix-test".to_string(),
            "-shard".to_string(),
            "abc".to_string(),
            "test/test-standalone.toml".to_string(),
        ];
        assert!(super::Args::parse(args).is_err(), "malformed shard value must be rejected");
    }
}
