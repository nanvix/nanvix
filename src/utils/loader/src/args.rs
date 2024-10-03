// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
/// This structure packs the command-line arguments that were passed to the program.
///
pub struct Args {
    /// Wasm filename.
    wasm_filename: String,
    /// HTTP server address.
    sockaddr: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    const OPT_HELP: &'static str = "-help";
    /// Command-line option for HTTP server address.
    const OPT_HTTP: &'static str = "-http";
    /// Command-line option for Wasm filename.
    const OPT_WASM: &'static str = "-wasm";

    ///
    /// # Description
    ///
    /// Parses the command-line arguments that were passed to the program.
    ///
    /// # Parameters
    ///
    /// - `args`: Command-line arguments.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the parsed command-line arguments that were passed to the
    /// program. Upon failure, the function returns an error.
    ///
    pub fn parse(args: Vec<String>) -> Result<Self> {
        trace!("parse(): parsing command-line arguments...");
        let mut wasm_filename: Option<String> = None;
        let mut sockaddr: Option<String> = None;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_WASM => {
                    i += 1;
                    wasm_filename = Some(args[i].clone());
                },
                Self::OPT_HTTP => {
                    i += 1;
                    sockaddr = Some(args[i].clone());
                },
                Self::OPT_HELP => {
                    Self::usage();
                    return Err(anyhow::anyhow!("help message"));
                },
                _ => {
                    return Err(anyhow::anyhow!("invalid argument"));
                },
            }

            i += 1;
        }

        // Get HTTP server address.
        let sockaddr: String = sockaddr.ok_or_else(|| {
            Self::usage();
            anyhow::anyhow!("missing HTTP server address")
        })?;

        // Get wasm filename.
        let wasm_filename: String = wasm_filename.ok_or_else(|| {
            Self::usage();
            anyhow::anyhow!("missing Wasm filename")
        })?;

        Ok(Self {
            wasm_filename,
            sockaddr,
        })
    }

    ///
    /// # Description
    ///
    /// Prints program usage.
    ///
    pub fn usage() {
        println!(
            "Usage: loader {} <wasm_filename> {} <http_server_address>",
            Self::OPT_WASM,
            Self::OPT_HTTP
        );
    }

    ///
    /// # Description
    ///
    /// Returns the Wasm filename passed as a command-line argument.
    ///
    /// # Returns
    ///
    /// The Wasm filename.
    ///
    pub fn wasm_filename(&self) -> &str {
        &self.wasm_filename
    }

    ///
    /// # Description
    ///
    /// Returns the HTTP server address passed as a command-line argument.
    ///
    /// # Returns
    ///
    /// The HTTP server address.
    ///
    pub fn sockaddr(&self) -> &str {
        &self.sockaddr
    }
}
