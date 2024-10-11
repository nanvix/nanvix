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
    /// Server socket address.
    server_sockaddr: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    const OPT_HELP: &'static str = "-help";
    const OPT_SERVER_SOCKADDR: &'static str = "-server";

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

        let mut server_sockaddr: String = String::new();

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    return Err(anyhow::anyhow!("help message"));
                },
                Self::OPT_SERVER_SOCKADDR => {
                    i += 1;
                    server_sockaddr = args[i].clone();
                },
                _ => {
                    return Err(anyhow::anyhow!("invalid argument"));
                },
            }

            i += 1;
        }

        Ok(Self { server_sockaddr })
    }

    ///
    /// # Description
    ///
    /// Prints program usage.
    ///
    /// # Parameters
    ///
    /// - `program_name`: Name of the program.
    ///
    pub fn usage(program_name: &str) {
        println!("Usage: {} {} <server-sockaddr>", program_name, Self::OPT_SERVER_SOCKADDR,);
    }

    ///
    /// # Description
    ///
    /// Returns the server socket address.
    ///
    /// # Returns
    ///
    /// The server socket address.
    ///
    pub fn server_sockaddr(&self) -> String {
        self.server_sockaddr.to_string()
    }
}
