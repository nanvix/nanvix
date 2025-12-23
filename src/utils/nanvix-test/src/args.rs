// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
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
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    const OPT_HELP: &'static str = "-help";

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

Required positional arguments:
    config-file             Path to the TOML configuration that describes the runner and test case.
",
            program_name = program_name,
            help = Self::OPT_HELP,
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

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    process::exit(0);
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

        Ok(Self { config_file_path })
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
}
