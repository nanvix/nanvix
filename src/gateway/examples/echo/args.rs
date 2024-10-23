// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Arguments
//!
//! This module provides utilities for parsing command-line arguments that were supplied to the
//! program.
//!

//==================================================================================================
// Imports
//==================================================================================================

use crate::config;
use ::anyhow::Result;
use ::std::{
    env,
    process,
};

//==================================================================================================
// Public Structures
//==================================================================================================

///
/// # Description
///
/// This structure packs the command-line arguments that were passed to the program.
///
pub struct Args {
    /// HTTP server address.
    http_addr: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    const OPT_HELP: &'static str = "-help";
    /// Command-line for HTTP.
    const OPT_HTTP: &'static str = "-http";

    ///
    /// # Description
    ///
    /// Parses the command-line arguments that were passed to the program.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the command-line arguments that were passed
    /// to the program. Otherwise, it returns an error.
    ///
    pub fn parse(args: Vec<String>) -> Result<Self> {
        trace!("parse(): args={:?}", args);

        let mut http_addr: Option<String> = None;

        // Parse command-line arguments.
        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                // Print help message and exit.
                Self::OPT_HELP => {
                    Self::usage();
                    process::exit(0);
                },
                // Set HTTP server.
                Self::OPT_HTTP if i + 1 < args.len() => {
                    http_addr = Some(args[i + 1].clone());
                    i += 1;
                },

                // Invalid argument.
                _ => {
                    Self::usage();
                    let reason: String = format!("invalid argument {}", args[i]);
                    error!("parse(): {}", reason);
                    anyhow::bail!(reason);
                },
            }

            i += 1;
        }

        // Check if the socket address option was set.
        if let Some(http_addr) = http_addr.take() {
            Ok(Self { http_addr })
        } else {
            Self::usage();
            anyhow::bail!("missing socket address")
        }
    }

    ///
    /// # Description
    ///
    /// Prints program usage.
    ///
    pub fn usage() {
        eprintln!(
            "Usage: {} {} <socket-address>",
            env::args()
                .next()
                .unwrap_or(config::PROGRAM_NAME.to_string()),
            Self::OPT_HTTP
        );
    }

    ///
    /// # Description
    ///
    /// Returns the HTTP server address that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The HTTP server address that was passed as a command-line argument to the program. If no
    /// HTTP server address was passed, this method returns `None`.
    ///
    pub fn http_addr(&mut self) -> &str {
        &self.http_addr
    }
}
