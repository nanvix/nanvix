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
    /// Inter-arrival time.
    frequency: u128,
    /// Timeout.
    timeout: u64,
    /// Number of threads.
    nthreads: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    const OPT_HELP: &'static str = "-help";
    /// Package injection frequency.
    const OPT_FREQUENCY: &'static str = "-frequency";
    /// Timeout.
    const OPT_TIMEOUT: &'static str = "-timeout";
    /// Number of threads.
    const OPT_NTHREADS: &'static str = "-nthreads";
    /// Server socket address.
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
        let mut interarrival: u128 = 0;
        let mut timeout: u64 = 0;
        let mut nthreads: usize = 0;

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    return Err(anyhow::anyhow!("help message"));
                },
                Self::OPT_FREQUENCY => {
                    i += 1;
                    interarrival = match args[i].parse::<u128>() {
                        Ok(num) => num,
                        Err(_) => {
                            return Err(anyhow::anyhow!("invalid package injection frequency"));
                        },
                    };
                },
                Self::OPT_NTHREADS => {
                    i += 1;
                    nthreads = match args[i].parse::<usize>() {
                        Ok(num) => num,
                        Err(_) => {
                            return Err(anyhow::anyhow!("invalid number of threads"));
                        },
                    };
                },
                Self::OPT_SERVER_SOCKADDR => {
                    i += 1;
                    server_sockaddr = args[i].clone();
                },
                Self::OPT_TIMEOUT => {
                    i += 1;
                    timeout = match args[i].parse::<u64>() {
                        Ok(num) => num,
                        Err(_) => {
                            return Err(anyhow::anyhow!("invalid timeout"));
                        },
                    };
                },
                _ => {
                    return Err(anyhow::anyhow!("invalid argument"));
                },
            }

            i += 1;
        }

        Ok(Self {
            frequency: interarrival,
            timeout,
            nthreads,
            server_sockaddr,
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
            "Usage: {} {} <injection-frequency> {} <nthreads> {} <server-sockaddr> {} <timeout>",
            program_name,
            Self::OPT_FREQUENCY,
            Self::OPT_NTHREADS,
            Self::OPT_SERVER_SOCKADDR,
            Self::OPT_TIMEOUT
        );
    }

    ///
    /// # Description
    ///
    /// Returns the package-injection frequency.
    ///
    /// # Returns
    ///
    /// The package-injection frequency.
    ///
    pub fn frequency(&self) -> u128 {
        self.frequency
    }

    ///
    /// # Description
    ///
    /// Returns the number of threads.
    ///
    /// # Returns
    ///
    /// The number of threads.
    ///
    pub fn nthreads(&self) -> usize {
        self.nthreads
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

    ///
    /// # Description
    ///
    /// Returns the timeout.
    ///
    /// # Returns
    ///
    /// The timeout.
    ///
    pub fn timeout(&self) -> u64 {
        self.timeout
    }
}
