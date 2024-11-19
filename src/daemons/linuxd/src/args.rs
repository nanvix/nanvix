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
    bind_sockaddr: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    const OPT_HELP: &'static str = "-help";
    /// Command-line option for setting bind socket address.
    const OPT_BIND_SOCKADDR: &'static str = "-bind-addr";

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

        let mut bind_sockaddr: String = String::new();

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    return Err(anyhow::anyhow!("help message"));
                },
                Self::OPT_BIND_SOCKADDR => {
                    i += 1;
                    bind_sockaddr = args[i].clone();
                },
                _ => {
                    return Err(anyhow::anyhow!("invalid argument"));
                },
            }

            i += 1;
        }


        // Check if server socket address was set.
        if bind_sockaddr.is_empty() {
            return Err(anyhow::anyhow!("server socket address not set"));
        }

        Ok(Self { bind_sockaddr })
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
        println!("Usage: {} {} <server-sockaddr>", program_name, Self::OPT_BIND_SOCKADDR,);
    }

    ///
    /// # Description
    ///
    /// Returns the bind socket address.
    ///
    /// # Returns
    ///
    /// The socket address of the bind socket.
    ///
    pub fn bind_sockaddr(&self) -> String {
        self.bind_sockaddr.to_string()
    }
}
