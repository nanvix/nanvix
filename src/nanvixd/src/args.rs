// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config;
use ::anyhow::Result;
use ::std::time::Duration;

//==================================================================================================
// Structures
//==================================================================================================

pub struct Args {
    http_sockaddr: String,
    linuxd_sockaddr: String,
    sandbox_sockaddr: String,
    console_file: String,
    keep_alive_timeout: Duration,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    const OPT_HELP: &'static str = "-help";
    const OPT_HTTP_SOCKADDR: &'static str = "-http-addr";
    const OPT_LINUXD_SOCKADDR: &'static str = "-linuxd-addr";
    const OPT_SANDBOX_SOCKADDR: &'static str = "-sandbox-addr";
    const OPT_CONSOLE_FILE: &'static str = "-console-file";
    const OPT_KEEP_ALIVE_TIMEOUT: &'static str = "-keep-alive";

    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut http_sockaddr: String = String::new();
        let mut linuxd_sockaddr: String = config::DEFAULT_LINUXD_SOCKADDR.to_string();
        let mut sandbox_sockaddr: String = config::DEFAULT_SANDBOX_SOCKADDR.to_string();
        let mut console_file: String = config::DEFAULT_CONSOLE_FILE.to_string();
        let mut keep_alive_timeout: Duration =
            Duration::from_secs(config::DEFAULT_KEEP_ALIVE_TIMEOUT);

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    return Err(anyhow::anyhow!("wrong usage"));
                },
                Self::OPT_HTTP_SOCKADDR => {
                    i += 1;
                    http_sockaddr = args[i].clone();
                },
                Self::OPT_LINUXD_SOCKADDR => {
                    i += 1;
                    linuxd_sockaddr = args[i].clone();
                },
                Self::OPT_SANDBOX_SOCKADDR => {
                    i += 1;
                    sandbox_sockaddr = args[i].clone();
                },
                Self::OPT_CONSOLE_FILE => {
                    i += 1;
                    console_file = args[i].clone();
                },
                Self::OPT_KEEP_ALIVE_TIMEOUT => {
                    i += 1;
                    keep_alive_timeout = Duration::from_secs(args[i].parse::<u64>()?);
                },
                _ => {
                    return Err(anyhow::anyhow!("invalid argument"));
                },
            }

            i += 1;
        }

        Ok(Self {
            http_sockaddr,
            linuxd_sockaddr,
            sandbox_sockaddr,
            console_file,
            keep_alive_timeout,
        })
    }

    pub fn usage(program_name: &str) {
        println!(
            "Usage: {} {} <sockaddr> {} <sockaddr> {} <sockaddr> {} <file> {} <duration>",
            program_name,
            Self::OPT_HTTP_SOCKADDR,
            Self::OPT_LINUXD_SOCKADDR,
            Self::OPT_SANDBOX_SOCKADDR,
            Self::OPT_CONSOLE_FILE,
            Self::OPT_KEEP_ALIVE_TIMEOUT
        );
    }

    pub fn http_sockaddr(&self) -> &str {
        &self.http_sockaddr
    }

    pub fn linuxd_sockaddr(&self) -> &str {
        &self.linuxd_sockaddr
    }

    pub fn sandbox_sockaddr(&self) -> &str {
        &self.sandbox_sockaddr
    }

    pub fn nanvix_console(&self) -> &str {
        &self.console_file
    }

    pub fn keep_alive_timeout(&self) -> Duration {
        self.keep_alive_timeout
    }
}
