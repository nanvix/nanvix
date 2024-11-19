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
    /// Gateway socket address.
    gateway_sockaddr: Option<String>,
    /// Log to file?
    log_to_file: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    const OPT_HELP: &'static str = "-help";
    /// Command-line option for setting bind socket address.
    const OPT_BIND_SOCKADDR: &'static str = "-bind-addr";
    /// Command-line option for setting socket address of gateway.
    const OPT_GATEWAY_SOCKADDR: &'static str = "-gateway-addr";
    /// Command-line option for log redirecting.
    const OPT_LOGFILE: &'static str = "-log-to-file";

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
        let mut bind_sockaddr: String = String::new();
        let mut gateway_sockaddr: Option<String> = None;
        let mut log_to_file: bool = false;

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
                Self::OPT_GATEWAY_SOCKADDR => {
                    i += 1;
                    gateway_sockaddr = Some(args[i].clone());
                },
                Self::OPT_LOGFILE => {
                    log_to_file = true;
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

        Ok(Self {
            bind_sockaddr,
            gateway_sockaddr,
            log_to_file,
        })
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
        println!(
            "Usage: {} {} {} <server-sockaddr> {} <gateway-sockaddr>",
            program_name,
            Self::OPT_LOGFILE,
            Self::OPT_BIND_SOCKADDR,
            Self::OPT_GATEWAY_SOCKADDR
        );
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

    ///
    /// # Description
    ///
    /// Returns the gateway socket address.
    ///
    /// # Returns
    ///
    /// The socket address of the gateway.
    ///
    pub fn gateway_sockaddr(&self) -> Option<String> {
        self.gateway_sockaddr.clone()
    }

    ///
    /// # Description
    ///
    /// Returns the log file.
    ///
    /// # Returns
    ///
    /// The log file.
    ///
    pub fn log_to_file(&self) -> bool {
        self.log_to_file
    }
}
