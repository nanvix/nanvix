// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config;
use ::anyhow::Result;

//==================================================================================================
// Structures
//==================================================================================================

pub struct Args {
    http_sockaddr: String,
    tmp_directory: String,
    console_file: Option<String>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    const OPT_HELP: &'static str = "-help";
    const OPT_HTTP_SOCKADDR: &'static str = "-http-addr";
    const OPT_TMP_DIRECTORY: &'static str = "-tmp-dir";
    const OPT_CONSOLE_FILE: &'static str = "-console-file";

    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut http_sockaddr: String = String::new();
        let mut tmp_directory: String = config::DEFAULT_TMP_DIRECTORY.to_string();
        let mut console_file: Option<String> = None;

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
                Self::OPT_TMP_DIRECTORY => {
                    i += 1;
                    tmp_directory = args[i].clone();
                },
                Self::OPT_CONSOLE_FILE => {
                    i += 1;
                    console_file = Some(args[i].clone());
                },
                arg => {
                    return Err(anyhow::anyhow!("invalid argument: {arg}"));
                },
            }

            i += 1;
        }

        Ok(Self {
            http_sockaddr,
            tmp_directory,
            console_file,
        })
    }

    pub fn usage(program_name: &str) {
        println!(
            "Usage: {} {} <sockaddr> [{} <file>] [{} <tmp_dir>]",
            program_name,
            Self::OPT_HTTP_SOCKADDR,
            Self::OPT_CONSOLE_FILE,
            Self::OPT_TMP_DIRECTORY
        );
    }

    pub fn http_sockaddr(&self) -> &str {
        &self.http_sockaddr
    }

    pub fn tmp_directory(&self) -> &str {
        &self.tmp_directory
    }

    pub fn nanvix_console(&self) -> Option<String> {
        self.console_file.clone()
    }
}
