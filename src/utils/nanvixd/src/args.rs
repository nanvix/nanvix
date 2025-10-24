// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Command-line argument parsing for Nanvix Daemon.
//!
//! This module provides functionality to parse and validate command-line arguments passed to
//! the Nanvix Daemon. It handles various configuration options including network settings,
//! directory paths, hardware topology, and deployment modes.

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
use ::syscomm::SocketType;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Stores parsed command-line arguments for nanvixd.
///
/// This structure holds all configuration parameters provided via command-line arguments,
/// including network settings, directory paths, hardware topology information, socket types,
/// and deployment mode flags.
///
#[derive(Debug, Clone)]
pub struct Args {
    /// HTTP server socket address (host:port).
    http_sockaddr: String,
    /// Directory path for temporary files and Unix sockets.
    tmp_directory: String,
    /// Directory path containing Nanvix binaries.
    binary_directory: String,
    /// Directory path containing toolchain binaries (cloud-hypervisor, etc.).
    toolchain_binary_directory: String,
    /// Optional file path for redirecting console output.
    console_file: Option<String>,
    /// Optional hardware locality configuration for CPU affinity and topology.
    hwloc: Option<HwLoc>,
    /// Flag indicating whether to log to files instead of stdout/stderr.
    log_to_file: bool,
    /// Directory path for writing log files when log_to_file is enabled.
    log_directory: String,
    /// Flag indicating whether to deploy linuxd inside an L2 VM.
    l2: bool,
    /// Optional socket type for control plane communication (nanvixd <-> linuxd).
    control_plane_socket_type: Option<SocketType>,
    /// Optional socket type for gateway communication (client <-> linuxd stdin/stdout).
    gateway_socket_type: Option<SocketType>,
    /// Optional socket type for system VM communication (linuxd <-> uservm).
    system_vm_socket_type: Option<SocketType>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line flag that prints usage information.
    pub const OPT_HELP: &'static str = "-help";
    /// Command-line option that sets the HTTP socket address.
    pub const OPT_HTTP_SOCKADDR: &'static str = "-http-addr";
    /// Command-line option that sets the temporary directory path.
    pub const OPT_TMP_DIRECTORY: &'static str = "-tmp-dir";
    /// Command-line option that sets the binary directory path.
    pub const OPT_BIN_DIRECTORY: &'static str = "-bin-dir";
    /// Command-line option that sets the toolchain binary directory path.
    pub const OPT_TOOLCHAIN_BIN_DIRECTORY: &'static str = "-toolchain-bin-dir";
    /// Command-line option that redirects the console output to a file.
    pub const OPT_CONSOLE_FILE: &'static str = "-console-file";
    /// Command-line option that loads the serialized CPU topology.
    pub const OPT_HWLOC: &'static str = "-hwloc";
    /// Command-line flag that enables logging to files.
    pub const OPT_LOG_TO_FILE: &'static str = "-log-to-file";
    /// Command-line option that sets the log directory path.
    pub const OPT_LOG_DIRECTORY: &'static str = "-log-dir";
    /// Command-line flag that enables L2 deployment mode.
    pub const OPT_L2: &'static str = "-l2";
    /// Command-line option that sets the control plane socket type.
    pub const OPT_CONTROL_PLANE_SOCKET_TYPE: &'static str = "-control-plane-socket-type";
    /// Command-line option that sets the gateway socket type.
    pub const OPT_GATEWAY_SOCKET_TYPE: &'static str = "-gateway-socket-type";
    /// Command-line option that sets the system VM socket type.
    pub const OPT_SYSTEM_VM_SOCKET_TYPE: &'static str = "-system-vm-socket-type";

    ///
    /// # Description
    ///
    /// Parses command-line arguments into an Args structure.
    ///
    /// This function processes all supported command-line flags and options, validates them,
    /// and enforces constraints such as requiring TCP sockets for L2 deployment mode.
    ///
    /// # Parameters
    ///
    /// - `args`: Vector of command-line arguments to parse.
    ///
    /// # Returns
    ///
    /// On success, returns the parsed arguments. On failure, returns an error describing
    /// the parsing issue or validation failure.
    ///
    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut http_sockaddr: String = String::new();
        let mut tmp_directory: String = config::DEFAULT_TMP_DIRECTORY.to_string();
        let mut binary_directory: String = config::DEFAULT_BIN_DIRECTORY.to_string();
        let mut toolchain_binary_directory: String =
            config::DEFAULT_TOOLCHAIN_BIN_DIRECTORY.to_string();
        let mut console_file: Option<String> = None;
        let mut hwloc: Option<HwLoc> = None;
        let mut log_to_file: bool = false;
        let mut log_directory: String = config::DEFAULT_LOG_DIRECTORY.to_string();
        let mut l2: bool = false;
        let mut control_plane_socket_type: Option<SocketType> = None;
        let mut gateway_socket_type: Option<SocketType> = None;
        let mut system_vm_socket_type: Option<SocketType> = None;

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
                Self::OPT_CONTROL_PLANE_SOCKET_TYPE => {
                    i += 1;
                    control_plane_socket_type = Some(args[i].parse()?);
                },
                Self::OPT_GATEWAY_SOCKET_TYPE => {
                    i += 1;
                    gateway_socket_type = Some(args[i].parse()?);
                },
                Self::OPT_SYSTEM_VM_SOCKET_TYPE => {
                    i += 1;
                    system_vm_socket_type = Some(args[i].parse()?);
                },
                Self::OPT_LOG_TO_FILE => {
                    log_to_file = true;
                },
                Self::OPT_LOG_DIRECTORY => {
                    i += 1;
                    log_directory = args[i].clone();
                },
                arg => {
                    return Err(anyhow::anyhow!("invalid argument: {arg}"));
                },
            }

            i += 1;
        }

        // If we deploy the Linux Daemon (linuxd) in an L2 VM, we need to make sure that all socket
        // types are set to TCP.
        if l2 {
            if control_plane_socket_type == Some(SocketType::Unix) {
                anyhow::bail!("control-plane must use a tcp socket in l2 deployments");
            }

            if gateway_socket_type == Some(SocketType::Unix) {
                anyhow::bail!("gateway must use a tcp socket in l2 deployments");
            }

            if system_vm_socket_type == Some(SocketType::Unix) {
                anyhow::bail!("system vm must use a tcp socket in l2 deployments");
            }

            control_plane_socket_type = Some(SocketType::Tcp);
            gateway_socket_type = Some(SocketType::Tcp);
            system_vm_socket_type = Some(SocketType::Tcp);
        }

        Ok(Self {
            http_sockaddr,
            tmp_directory,
            binary_directory,
            toolchain_binary_directory,
            console_file,
            hwloc,
            log_to_file,
            log_directory,
            l2,
            control_plane_socket_type,
            gateway_socket_type,
            system_vm_socket_type,
        })
    }

    ///
    /// # Description
    ///
    /// Prints program usage information to stdout.
    ///
    /// # Parameters
    ///
    /// - `program_name`: Name of the program executable.
    ///
    pub fn usage(program_name: &str) {
        println!(
            concat!(
                "Usage: {} {} <sockaddr> [{} <file>] [{} <tmp_dir>] [{} <bin_dir>] ",
                "[{} <toolchain_bin_dir>] [{} <hwloc.json>] [{} [{} <log_dir>]] ",
                "[{} <socket_type>] [{} <socket_type>] [{} <socket_type>] [{}]"
            ),
            program_name,
            Self::OPT_HTTP_SOCKADDR,
            Self::OPT_CONSOLE_FILE,
            Self::OPT_TMP_DIRECTORY,
            Self::OPT_BIN_DIRECTORY,
            Self::OPT_TOOLCHAIN_BIN_DIRECTORY,
            Self::OPT_HWLOC,
            Self::OPT_LOG_TO_FILE,
            Self::OPT_LOG_DIRECTORY,
            Self::OPT_CONTROL_PLANE_SOCKET_TYPE,
            Self::OPT_GATEWAY_SOCKET_TYPE,
            Self::OPT_SYSTEM_VM_SOCKET_TYPE,
            Self::OPT_L2
        );
    }

    ///
    /// # Description
    ///
    /// Returns the HTTP socket address.
    ///
    /// # Returns
    ///
    /// The HTTP socket address.
    ///
    pub fn http_sockaddr(&self) -> &str {
        &self.http_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the temporary directory path.
    ///
    /// # Returns
    ///
    /// The temporary directory path.
    ///
    pub fn tmp_directory(&self) -> &str {
        &self.tmp_directory
    }

    ///
    /// # Description
    ///
    /// Returns the binary directory path.
    ///
    /// # Returns
    ///
    /// The binary directory path.
    ///
    pub fn binary_directory(&self) -> &str {
        &self.binary_directory
    }

    ///
    /// # Description
    ///
    /// Returns the toolchain binary directory path.
    ///
    /// # Returns
    ///
    /// The toolchain binary directory path.
    ///
    pub fn toolchain_binary_directory(&self) -> &str {
        &self.toolchain_binary_directory
    }

    ///
    /// # Description
    ///
    /// Returns the console file path.
    ///
    /// # Returns
    ///
    /// The console file path.
    ///
    pub fn console_file(&self) -> Option<String> {
        self.console_file.clone()
    }

    ///
    /// # Description
    ///
    /// Returns the CPU topology.
    ///
    /// # Returns
    ///
    /// The CPU topology.
    ///
    pub fn hwloc(&self) -> Option<HwLoc> {
        self.hwloc.clone()
    }

    ///
    /// # Description
    ///
    /// Indicates whether linuxd must be deployed in an L2 VM or not.
    ///
    /// # Returns
    ///
    /// `true` if linuxd must be deployed in an L2 VM; `false` otherwise.
    ///
    pub fn l2(&self) -> bool {
        self.l2
    }

    ///
    /// # Description
    ///
    /// Indicates whether to log to a file instead of stdout/stderr.
    ///
    /// # Returns
    ///
    /// `true` if logging to a file; `false` otherwise.
    ///
    pub fn log_to_file(&self) -> bool {
        self.log_to_file
    }

    ///
    /// # Description
    ///
    /// Returns the log directory.
    ///
    /// # Returns
    ///
    /// The log directory.
    ///
    pub fn log_directory(&self) -> &str {
        &self.log_directory
    }

    ///
    /// # Description
    ///
    /// Returns the control plane socket type.
    ///
    /// # Returns
    ///
    /// The control plane socket type.
    ///
    pub fn control_plane_socket_type(&self) -> SocketType {
        self.control_plane_socket_type.unwrap_or(SocketType::Unix)
    }

    ///
    /// # Description
    ///
    /// Returns the gateway socket type.
    ///
    /// # Returns
    ///
    /// The gateway socket type.
    ///
    pub fn gateway_socket_type(&self) -> SocketType {
        self.gateway_socket_type.unwrap_or(SocketType::Unix)
    }

    ///
    /// # Description
    ///
    /// Returns the system VM socket type.
    ///
    /// # Returns
    ///
    /// The system VM socket type.
    ///
    pub fn system_vm_socket_type(&self) -> SocketType {
        self.system_vm_socket_type.unwrap_or(SocketType::Unix)
    }
}
