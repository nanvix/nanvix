// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config;
use ::anyhow::Result;
use ::hwloc::HwLoc;
use ::std::{
    fs::File,
    io::BufReader,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone)]
pub struct Args {
    http_sockaddr: String,
    tmp_directory: String,
    binary_directory: String,
    toolchain_binary_directory: String,
    console_file: Option<String>,
    hwloc: Option<HwLoc>,
    /// Whether to log to a file instead of stdout/stderr.
    log_to_file: bool,
    /// Whether linuxd must be deployed in an L2 VM or not.
    l2: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    pub const OPT_HELP: &'static str = "-help";
    pub const OPT_HTTP_SOCKADDR: &'static str = "-http-addr";
    pub const OPT_TMP_DIRECTORY: &'static str = "-tmp-dir";
    pub const OPT_BIN_DIRECTORY: &'static str = "-bin-dir";
    pub const OPT_TOOLCHAIN_BIN_DIRECTORY: &'static str = "-toolchain-bin-dir";
    pub const OPT_CONSOLE_FILE: &'static str = "-console-file";
    pub const OPT_HWLOC: &'static str = "-hwloc";
    pub const OPT_LOG_TO_FILE: &'static str = "--log-to-file";
    pub const OPT_L2: &'static str = "-l2";

    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut http_sockaddr: String = String::new();
        let mut tmp_directory: String = config::DEFAULT_TMP_DIRECTORY.to_string();
        let mut binary_directory: String = config::DEFAULT_BIN_DIRECTORY.to_string();
        let mut toolchain_binary_directory: String =
            config::DEFAULT_TOOLCHAIN_BIN_DIRECTORY.to_string();
        let mut console_file: Option<String> = None;
        let mut hwloc: Option<HwLoc> = None;
        let mut log_to_file: bool = false;
        let mut l2: bool = false;

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
                Self::OPT_BIN_DIRECTORY => {
                    i += 1;
                    binary_directory = args[i].clone();
                },
                Self::OPT_TOOLCHAIN_BIN_DIRECTORY => {
                    i += 1;
                    toolchain_binary_directory = args[i].clone();
                },
                Self::OPT_CONSOLE_FILE => {
                    i += 1;
                    console_file = Some(args[i].clone());
                },
                Self::OPT_HWLOC => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_HWLOC));
                    }

                    // Parse hwloc from JSON file.
                    let hwloc_file = File::open(args[i].clone())?;
                    let hwloc_reader = BufReader::new(hwloc_file);
                    hwloc = Some(serde_json::from_reader(hwloc_reader)?);
                },
                Self::OPT_L2 => {
                    l2 = true;
                },
                Self::OPT_LOG_TO_FILE => {
                    log_to_file = true;
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
            binary_directory,
            toolchain_binary_directory,
            console_file,
            hwloc,
            log_to_file,
            l2,
        })
    }

    pub fn usage(program_name: &str) {
        println!(
            "Usage: {} {} <sockaddr> [{} <file>] [{} <tmp_dir>] [{} <bin_dir>] [{} \
             <toolchain_bin_dir>] [{} <hwloc.json>] [{}] [{}]",
            program_name,
            Self::OPT_HTTP_SOCKADDR,
            Self::OPT_CONSOLE_FILE,
            Self::OPT_TMP_DIRECTORY,
            Self::OPT_BIN_DIRECTORY,
            Self::OPT_TOOLCHAIN_BIN_DIRECTORY,
            Self::OPT_HWLOC,
            Self::OPT_LOG_TO_FILE,
            Self::OPT_L2
        );
    }

    pub fn http_sockaddr(&self) -> &str {
        &self.http_sockaddr
    }

    pub fn tmp_directory(&self) -> &str {
        &self.tmp_directory
    }

    pub fn binary_directory(&self) -> &str {
        &self.binary_directory
    }

    pub fn toolchain_binary_directory(&self) -> &str {
        &self.toolchain_binary_directory
    }

    pub fn console_file(&self) -> Option<String> {
        self.console_file.clone()
    }

    pub fn hwloc(&self) -> Option<HwLoc> {
        self.hwloc.clone()
    }

    pub fn l2(&self) -> bool {
        self.l2
    }

    pub fn log_to_file(&self) -> bool {
        self.log_to_file
    }
}
