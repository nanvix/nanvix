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

use ::anyhow::Result;
use ::std::{
    env,
    path::{
        Path,
        PathBuf,
    },
    process,
};
use ::syslog::DEFAULT_LOG_DIRECTORY;
#[cfg(target_os = "linux")]
use ::user_vm_api::UserVmIdentifier;

/// Minimal cross-platform replacement for [`user_vm_api::UserVmIdentifier`] on non-Linux targets.
#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserVmIdentifier(u32);

#[cfg(not(target_os = "linux"))]
impl UserVmIdentifier {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

#[cfg(not(target_os = "linux"))]
impl From<UserVmIdentifier> for u32 {
    fn from(id: UserVmIdentifier) -> Self {
        id.0
    }
}

#[cfg(not(target_os = "linux"))]
impl std::fmt::Display for UserVmIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

//==================================================================================================
// Public Structures
//==================================================================================================

///
/// # Description
///
/// This structure packs the command-line arguments that were passed to the program.
///
pub struct Args {
    /// Unique identifier for this VM.
    user_vm_id: UserVmIdentifier,
    /// Kernel filename.
    kernel_filename: String,
    /// Initrd filename.
    initrd_filename: Option<String>,
    /// Ramfs filename.
    ramfs_filename: Option<String>,
    /// Arguments to be passed to the initrd.
    initrd_args: Option<String>,
    /// Arguments to be passed to the kernel.
    kernel_args: Option<String>,
    /// Standard error.
    vm_stderr: Option<String>,
    /// System VM address.
    system_vm_addr: String,
    /// Control-plane address.
    control_plane_addr: String,
    /// Socket address exposed in the system VM for users to connect to the user VM's stdin/stdout.
    gateway_addr: String,
    /// Log to file?
    log_to_file: bool,
    /// Log directory.
    log_directory: String,
    /// Socket address type of the system VM socket.
    system_vm_socket_type: String,
    /// Socket address type of the control-plane socket.
    control_plane_socket_type: String,
    /// Socket address type of the gateway socket.
    gateway_socket_type: String,
    /// Standalone mode: run without system VM, control-plane, or gateway connections.
    standalone: bool,
    /// Optional snapshot path: when set, restore from snapshot instead of cold-booting.
    snapshot_path: Option<String>,
    /// Enable host networking for the guest. Only meaningful in standalone mode.
    host_networking: bool,
    /// Optional decoupled `networkd` address. When set (standalone mode only), socket system calls
    /// are forwarded to an external `networkd` process instead of an in-process daemon.
    networkd_addr: Option<String>,
    /// Socket type used to reach the decoupled `networkd`. Defaults to Unix when unset. Meaningful
    /// only alongside `networkd_addr`.
    networkd_socket_type: Option<String>,
    /// Optional GDB server port: when set, start a GDB RSP server on this TCP port and wait for a
    /// debugger to connect before running the guest. Requires standalone mode.
    #[cfg(feature = "gdb")]
    gdb_port: Option<u16>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    pub const OPT_HELP: &'static str = "-help";
    /// Command-line option for id.
    pub const OPT_USER_VM_ID: &'static str = "-user-vm-id";
    /// Command-line option for initrd file.
    pub const OPT_INITRD: &'static str = "-initrd";
    /// Command-line option for the kernel file.
    pub const OPT_KERNEL: &'static str = "-kernel";
    /// Command-line option for the standard error.
    pub const OPT_STDERR: &'static str = "-stderr";
    /// Command-line option for system VM address.
    pub const OPT_SYSTEM_VM_SOCKADDR: &'static str = "-system-vm-addr";
    /// Command-line option for the system VM socket type.
    pub const OPT_SYSTEM_VM_SOCKET_TYPE: &'static str = "-system-vm-socket-type";
    /// Command-line option for control-plane address.
    pub const OPT_CONTROL_PLANE_SOCKADDR: &'static str = "-control-plane-addr";
    /// Command-line option for the control-plane socket type.
    pub const OPT_CONTROL_PLANE_SOCKET_TYPE: &'static str = "-control-plane-socket-type";
    /// Command-line option for setting socket address of the gateway.
    pub const OPT_GATEWAY_SOCKADDR: &'static str = "-gateway-addr";
    /// Command-line option for setting the socket address type of the gateway socket.
    pub const OPT_GATEWAY_SOCKET_TYPE: &'static str = "-gateway-bind-socket-type";
    /// Command-line option for specifying arguments to be passed to the initrd.
    pub const OPT_INITRD_ARGS: &'static str = "-initrd_args";
    /// Command-line option for specifying arguments to be passed to the kernel.
    pub const OPT_KERNEL_ARGS: &'static str = "-kernel-args";
    /// Command-line option for the ramfs file.
    pub const OPT_RAMFS: &'static str = "-ramfs";
    /// Log to file.
    pub const OPT_LOGFILE: &'static str = "-log-to-file";
    /// Log directory
    pub const OPT_LOGDIR: &'static str = "-log-dir";
    /// Command-line option for standalone mode.
    pub const OPT_STANDALONE: &'static str = "-standalone";
    /// Command-line option for snapshot restore path.
    pub const OPT_SNAPSHOT: &'static str = "-snapshot";
    /// Command-line option that enables host networking for the guest (standalone mode only).
    pub const OPT_ALLOW_HOST_NETWORKING: &'static str = "-allow-host-networking";
    /// Command-line option that sets the decoupled `networkd` address (standalone mode only).
    pub const OPT_NETWORKD_ADDR: &'static str = "-networkd-addr";
    /// Command-line option that sets the decoupled `networkd` socket type (standalone mode only).
    pub const OPT_NETWORKD_SOCKET_TYPE: &'static str = "-networkd-socket-type";
    /// Command-line option for GDB server port (standalone mode only).
    #[cfg(feature = "gdb")]
    pub const OPT_GDB_PORT: &'static str = "-gdb-port";

    /// Program name.
    const PROGRAM_NAME: &'static str = env!("CARGO_PKG_NAME");

    /// Test log file name for validation.
    const TEST_LOG_FILENAME: &'static str = "test.log";

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
        let mut user_vm_id_raw: Option<u32> = None;
        let mut kernel_filename: String = String::new();
        let mut initrd_filename: Option<String> = None;
        let mut ramfs_filename: Option<String> = None;
        let mut initrd_args: Option<String> = None;
        let mut kernel_args: Option<String> = None;
        let mut vm_stderr: Option<String> = None;
        let mut system_vm_addr: String = String::new();
        let mut control_plane_addr: String = String::new();
        let mut gateway_addr: String = String::new();
        let mut log_to_file: bool = false;
        let mut log_directory: Option<String> = None;
        let mut system_vm_socket_type: String = String::new();
        let mut control_plane_socket_type: String = String::new();
        let mut gateway_socket_type: String = String::new();
        let mut standalone: bool = false;
        let mut snapshot_path: Option<String> = None;
        let mut host_networking: bool = false;
        let mut networkd_addr: Option<String> = None;
        let mut networkd_socket_type: Option<String> = None;
        #[cfg(feature = "gdb")]
        let mut gdb_port: Option<u16> = None;

        // Parse command-line arguments.
        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                // Print help message and exit.
                Self::OPT_HELP => {
                    Self::usage();
                    process::exit(0);
                },
                // Parse user VM ID.
                Self::OPT_USER_VM_ID if i + 1 < args.len() => {
                    let user_vm_id_arg: &String = &args[i + 1];

                    // Parse memory size.
                    user_vm_id_raw = match user_vm_id_arg.parse::<u32>() {
                        Ok(id) => Some(id),
                        Err(e) => {
                            anyhow::bail!("invalid user vm id (arg={user_vm_id_arg}, error={e:?})");
                        },
                    };
                    i += 1;
                },
                // Set initrd file.
                Self::OPT_INITRD if i + 1 < args.len() => {
                    initrd_filename = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set initrd arguments.
                Self::OPT_INITRD_ARGS if i + 1 < args.len() => {
                    initrd_args = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set kernel arguments.
                Self::OPT_KERNEL_ARGS if i + 1 < args.len() => {
                    kernel_args = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set ramfs file.
                Self::OPT_RAMFS if i + 1 < args.len() => {
                    ramfs_filename = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set kernel file.
                Self::OPT_KERNEL if i + 1 < args.len() => {
                    kernel_filename = args[i + 1].clone();
                    i += 1;
                },
                // Set error file.
                Self::OPT_STDERR if i + 1 < args.len() => {
                    vm_stderr = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set system VM address.
                Self::OPT_SYSTEM_VM_SOCKADDR if i + 1 < args.len() => {
                    system_vm_addr = args[i + 1].clone();
                    i += 1;
                },
                // Set system VM socket type.
                Self::OPT_SYSTEM_VM_SOCKET_TYPE if i + 1 < args.len() => {
                    system_vm_socket_type = args[i + 1].clone();
                    i += 1;
                },
                // Set control-plane address.
                Self::OPT_CONTROL_PLANE_SOCKADDR if i + 1 < args.len() => {
                    control_plane_addr = args[i + 1].clone();
                    i += 1;
                },
                // Set control-plane socket type.
                Self::OPT_CONTROL_PLANE_SOCKET_TYPE if i + 1 < args.len() => {
                    control_plane_socket_type = args[i + 1].clone();
                    i += 1;
                },
                // Set gateway address.
                Self::OPT_GATEWAY_SOCKADDR if i + 1 < args.len() => {
                    gateway_addr = args[i + 1].clone();
                    i += 1;
                },
                // Set gateway socket type.
                Self::OPT_GATEWAY_SOCKET_TYPE if i + 1 < args.len() => {
                    gateway_socket_type = args[i + 1].clone();
                    i += 1;
                },
                // Set standalone mode.
                Self::OPT_STANDALONE => {
                    standalone = true;
                },
                // Set snapshot path.
                Self::OPT_SNAPSHOT if i + 1 < args.len() => {
                    snapshot_path = Some(args[i + 1].clone());
                    i += 1;
                },
                // Enable host networking (standalone mode only).
                Self::OPT_ALLOW_HOST_NETWORKING => {
                    host_networking = true;
                },
                // Set decoupled networkd address (standalone mode only).
                Self::OPT_NETWORKD_ADDR if i + 1 < args.len() => {
                    networkd_addr = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set decoupled networkd socket type (standalone mode only).
                Self::OPT_NETWORKD_SOCKET_TYPE if i + 1 < args.len() => {
                    let socket_type: &str = &args[i + 1];
                    if !matches!(socket_type.to_lowercase().as_str(), "unix" | "tcp") {
                        Self::usage();
                        anyhow::bail!(
                            "invalid {} value (expected 'unix' or 'tcp'): {}",
                            Self::OPT_NETWORKD_SOCKET_TYPE,
                            socket_type
                        );
                    }
                    networkd_socket_type = Some(socket_type.to_string());
                    i += 1;
                },
                // Set GDB server port (standalone mode only).
                #[cfg(feature = "gdb")]
                Self::OPT_GDB_PORT if i + 1 < args.len() => {
                    gdb_port = Some(args[i + 1].parse::<u16>().map_err(|e| {
                        anyhow::anyhow!("invalid GDB port (arg={}, error={e:?})", args[i + 1])
                    })?);
                    i += 1;
                },
                // Set log to file flag.
                Self::OPT_LOGFILE => {
                    log_to_file = true;
                },
                // Set log directory
                Self::OPT_LOGDIR if i + 1 < args.len() => {
                    log_directory = Some(args[i + 1].clone());
                    i += 1;
                },
                // Invalid argument.
                arg => {
                    Self::usage();
                    anyhow::bail!("invalid argument {}", arg);
                },
            }

            i += 1;
        }

        // Parse user VM ID. In standalone mode, default to 0 when not provided.
        let user_vm_id: UserVmIdentifier = match (user_vm_id_raw, standalone) {
            (Some(id), _) => UserVmIdentifier::new(id),
            (None, true) => UserVmIdentifier::new(0),
            (None, false) => {
                Self::usage();
                anyhow::bail!("user vm id is missing");
            },
        };

        // Check if kernel file is missing.
        if kernel_filename.is_empty() {
            Self::usage();
            anyhow::bail!("kernel file is missing");
        }

        // In non-standalone mode, all socket addresses and types are required.
        if !standalone {
            // Check if gateway address is missing.
            if gateway_addr.is_empty() {
                Self::usage();
                anyhow::bail!("gateway address is missing");
            }

            // Check if gateway socket type is missing.
            if gateway_socket_type.is_empty() {
                Self::usage();
                anyhow::bail!("gateway socket type is missing");
            }

            // Check if control-plane address is missing.
            if control_plane_addr.is_empty() {
                Self::usage();
                anyhow::bail!("control-plane address is missing");
            }

            // Check if control-plane socket type is missing.
            if control_plane_socket_type.is_empty() {
                Self::usage();
                anyhow::bail!("control-plane socket type is missing");
            }

            // Check if system VM address is missing.
            if system_vm_addr.is_empty() {
                Self::usage();
                anyhow::bail!("system VM address is missing");
            }

            // Check if system VM socket type is missing.
            if system_vm_socket_type.is_empty() {
                Self::usage();
                anyhow::bail!("system VM socket type is missing");
            }
        }

        // Check if log file directory was set if logging to file is enabled. Set the default directory if not.
        let log_directory: String = match (log_to_file, log_directory) {
            (true, Some(path)) => path,
            // Default to log dir relative to the current working directory, make the directory path absolute.
            (true, None) => {
                let mut abs_path: PathBuf = std::env::current_dir().map_err(|e| {
                    anyhow::anyhow!("failed to get current directory (error={:?})", e)
                })?;
                abs_path.push(DEFAULT_LOG_DIRECTORY);
                abs_path.to_str().map(|s| s.to_string()).ok_or_else(|| {
                    anyhow::anyhow!("failed to convert log directory path to string")
                })?
            },
            (false, _) => String::new(),
        };

        // Validate that the path to the log file exists if logging to file is enabled.
        if log_to_file {
            let path: &Path = Path::new(&log_directory);
            // Create the directory if it does not exist.
            if !path.exists() {
                std::fs::create_dir_all(path).map_err(|e| {
                    anyhow::anyhow!(
                        "failed to create log file directory (path={}, error={:?})",
                        path.display(),
                        e
                    )
                })?;
            }
            // Check if we can create and write a file in the directory.
            // TODO: Use a random string for the test file name to avoid collisions.
            let test_file_path: PathBuf = path.join(Self::TEST_LOG_FILENAME);
            match std::fs::File::create(&test_file_path) {
                Ok(_file) => {
                    // Clean up the test file.
                    std::fs::remove_file(&test_file_path).ok();
                },
                Err(error) => {
                    return Err(anyhow::anyhow!(
                        "failed to create log file (path={}, error={:?})",
                        test_file_path.display(),
                        error
                    ));
                },
            }
        }

        // Validate that GDB port is only used in standalone mode.
        #[cfg(feature = "gdb")]
        if gdb_port.is_some() && !standalone {
            Self::usage();
            anyhow::bail!("-gdb-port requires -standalone mode");
        }

        // A networkd socket type only makes sense alongside an address to connect to. Reject it on
        // its own rather than silently ignoring it.
        if networkd_socket_type.is_some() && networkd_addr.is_none() {
            Self::usage();
            anyhow::bail!(
                "{} requires {}",
                Self::OPT_NETWORKD_SOCKET_TYPE,
                Self::OPT_NETWORKD_ADDR
            );
        }

        // Forwarding to a decoupled networkd is a networking feature -- it is meaningless without
        // host networking enabled.
        if networkd_addr.is_some() && !host_networking {
            Self::usage();
            anyhow::bail!(
                "{} requires {}",
                Self::OPT_NETWORKD_ADDR,
                Self::OPT_ALLOW_HOST_NETWORKING
            );
        }

        // Host networking is only wired into the standalone I/O path. In managed mode the flags
        // would be silently ignored, so reject them to avoid a false sense of configuration.
        if host_networking && !standalone {
            Self::usage();
            anyhow::bail!(
                "{} requires {} mode",
                Self::OPT_ALLOW_HOST_NETWORKING,
                Self::OPT_STANDALONE
            );
        }

        Ok(Self {
            user_vm_id,
            kernel_filename,
            initrd_filename,
            ramfs_filename,
            initrd_args,
            kernel_args,
            vm_stderr,
            system_vm_addr,
            control_plane_addr,
            gateway_addr,
            log_to_file,
            log_directory,
            system_vm_socket_type,
            control_plane_socket_type,
            gateway_socket_type,
            standalone,
            snapshot_path,
            host_networking,
            networkd_addr,
            networkd_socket_type,
            #[cfg(feature = "gdb")]
            gdb_port,
        })
    }

    ///
    /// # Description
    ///
    /// Prints program usage.
    ///
    pub fn usage() {
        eprintln!(
            "Usage: {} [{} <id>] {} <kernel> [{} <file>] [{} <file>] [{}] [{} <system-vm-addr> {} \
             <control-plane-addr> {} <gateway-addr>] [{} [{} <dir>]] [{} <args>] [{} <args>] [{} \
             <file>] [{} <path>] [{} [{} <addr> [{} <type>]]]{}",
            Self::PROGRAM_NAME,
            Self::OPT_USER_VM_ID,
            Self::OPT_KERNEL,
            Self::OPT_INITRD,
            Self::OPT_STDERR,
            Self::OPT_STANDALONE,
            Self::OPT_SYSTEM_VM_SOCKADDR,
            Self::OPT_CONTROL_PLANE_SOCKADDR,
            Self::OPT_GATEWAY_SOCKADDR,
            Self::OPT_LOGFILE,
            Self::OPT_LOGDIR,
            Self::OPT_INITRD_ARGS,
            Self::OPT_KERNEL_ARGS,
            Self::OPT_RAMFS,
            Self::OPT_SNAPSHOT,
            Self::OPT_ALLOW_HOST_NETWORKING,
            Self::OPT_NETWORKD_ADDR,
            Self::OPT_NETWORKD_SOCKET_TYPE,
            if cfg!(feature = "gdb") {
                " [-gdb-port <port>]"
            } else {
                ""
            },
        );
    }

    ///
    /// # Description
    ///
    /// Returns the user VM ID that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The ID of the user VM.
    ///
    pub fn user_vm_id(&self) -> UserVmIdentifier {
        self.user_vm_id
    }

    ///
    /// # Description
    ///
    /// Returns the initrd filename that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The initrd filename that was passed as a command-line argument to the program. If no initrd
    /// filename was passed, this method returns `None`.
    ///
    pub fn initrd_filename(&mut self) -> Option<String> {
        self.initrd_filename.take()
    }

    ///
    /// # Description
    ///
    /// Returns the ramfs filename that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The ramfs filename that was passed as a command-line argument to the program. If no ramfs
    /// filename was passed, this method returns `None`.
    ///
    pub fn ramfs_filename(&mut self) -> Option<String> {
        self.ramfs_filename.take()
    }

    ///
    /// # Description
    ///
    /// Returns the initrd arguments that were passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The initrd arguments that were passed as a command-line argument to the program. If no
    /// initrd arguments were passed, this method returns `None`.
    ///
    pub fn initrd_args(&mut self) -> Option<String> {
        self.initrd_args.take()
    }

    ///
    /// # Description
    ///
    /// Returns the kernel arguments that were passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The kernel arguments that were passed as a command-line argument to the program. If no
    /// kernel arguments were passed, this method returns `None`.
    ///
    pub fn kernel_args(&mut self) -> Option<String> {
        self.kernel_args.take()
    }

    ///
    /// # Description
    ///
    /// Returns the kernel filename that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The kernel filename that was passed as a command-line argument to the program.
    ///
    pub fn kernel_filename(&self) -> &str {
        &self.kernel_filename
    }

    ///
    /// # Description
    ///
    /// Returns the name of the standard error file that was passed as a command-line argument to the
    /// program.
    ///
    /// # Returns
    ///
    /// The name of standard error file that was passed as a command-line argument to the program. If
    /// no standard error file was passed, this method returns `None`.
    ///
    pub fn take_vm_stderr(&mut self) -> Option<String> {
        self.vm_stderr.take()
    }

    ///
    /// # Description
    ///
    /// Returns the address of the system VM that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The system VM address that was passed as a command-line argument to the program.
    ///
    pub fn system_vm_addr(&self) -> &str {
        &self.system_vm_addr
    }

    ///
    /// # Description
    ///
    /// Returns the address of the control-plane that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The control-plane address that was passed as a command-line argument to the program.
    ///
    pub fn control_plane_addr(&self) -> &str {
        &self.control_plane_addr
    }

    ///
    /// # Description
    ///
    /// Returns the address of the gateway that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The gateway address that was passed as a command-line argument to the program.
    ///
    pub fn gateway_addr(&self) -> &str {
        &self.gateway_addr
    }

    ///
    /// # Description
    ///
    /// Returns whether the program should log to a file or to the standard output.
    ///
    /// # Returns
    ///
    /// Whether the program should log to a file or to the standard output.
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
    pub fn log_directory(&self) -> String {
        self.log_directory.clone()
    }

    ///
    /// # Description
    ///
    /// Returns the socket address type of the system VM socket that was passed as a command-line
    /// argument to the program.
    ///
    /// # Returns
    ///
    /// The socket address type of the system VM socket that was passed as a command-line argument to
    /// the program.
    ///
    pub fn system_vm_socket_type(&self) -> &str {
        &self.system_vm_socket_type
    }

    ///
    /// # Description
    ///
    /// Returns the socket address type of the control-plane socket that was passed as a command-line
    /// argument to the program.
    ///
    /// # Returns
    ///
    /// The socket address type of the control-plane socket that was passed as a command-line argument to
    /// the program.
    ///
    pub fn control_plane_socket_type(&self) -> &str {
        &self.control_plane_socket_type
    }

    ///
    /// # Description
    ///
    /// Returns the socket address type of the gateway socket that was passed as a command-line
    /// argument to the program.
    ///
    /// # Returns
    ///
    /// The socket address type of the gateway socket that was passed as a command-line argument to
    /// the program.
    ///
    pub fn gateway_socket_type(&self) -> &str {
        &self.gateway_socket_type
    }

    ///
    /// # Description
    ///
    /// Returns whether the program was launched in standalone mode.
    ///
    /// # Returns
    ///
    /// `true` if the user VM should run without connecting to a system VM, control-plane, or
    /// gateway. `false` otherwise.
    ///
    pub fn standalone(&self) -> bool {
        self.standalone
    }

    ///
    /// # Description
    ///
    /// Takes the snapshot path that was passed as a command-line argument.
    /// After this call, the internal snapshot path is set to `None`.
    ///
    /// # Returns
    ///
    /// The snapshot path, or `None` if not provided.
    ///
    pub fn take_snapshot_path(&mut self) -> Option<String> {
        self.snapshot_path.take()
    }

    ///
    /// # Description
    ///
    /// Returns whether host networking is enabled for the guest.
    ///
    /// # Returns
    ///
    /// `true` if `-allow-host-networking` was passed. `false` otherwise.
    ///
    pub fn host_networking_enabled(&self) -> bool {
        self.host_networking
    }

    ///
    /// # Description
    ///
    /// Returns the decoupled `networkd` address that was passed as a command-line argument.
    ///
    /// # Returns
    ///
    /// The `networkd` address, or `None` if the network daemon should run in-process.
    ///
    pub fn networkd_addr(&self) -> Option<&str> {
        self.networkd_addr.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the socket type used to reach the decoupled `networkd`.
    ///
    /// # Returns
    ///
    /// The `networkd` socket type, or `None` when unspecified (callers default to Unix).
    ///
    pub fn networkd_socket_type(&self) -> Option<&str> {
        self.networkd_socket_type.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional GDB server port.
    ///
    /// # Returns
    ///
    /// The TCP port for the GDB server, or `None` if GDB debugging was not requested.
    ///
    #[cfg(feature = "gdb")]
    pub fn gdb_port(&self) -> Option<u16> {
        self.gdb_port
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::anyhow::Result as AnyResult;
    use ::std::{
        env,
        fs,
        path::PathBuf,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    fn build_base_args() -> Vec<String> {
        vec![
            String::from("uservm"),
            Args::OPT_USER_VM_ID.to_string(),
            String::from("7"),
            Args::OPT_KERNEL.to_string(),
            String::from("kernel.elf"),
            Args::OPT_SYSTEM_VM_SOCKADDR.to_string(),
            String::from("127.0.0.1:7000"),
            Args::OPT_SYSTEM_VM_SOCKET_TYPE.to_string(),
            String::from("tcp"),
            Args::OPT_CONTROL_PLANE_SOCKADDR.to_string(),
            String::from("127.0.0.1:8000"),
            Args::OPT_CONTROL_PLANE_SOCKET_TYPE.to_string(),
            String::from("tcp"),
            Args::OPT_GATEWAY_SOCKADDR.to_string(),
            String::from("127.0.0.1:9000"),
            Args::OPT_GATEWAY_SOCKET_TYPE.to_string(),
            String::from("tcp"),
        ]
    }

    fn unique_log_dir() -> AnyResult<(String, PathBuf)> {
        let base_dir: PathBuf = env::temp_dir();
        let timestamp_nanos: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| ::anyhow::anyhow!("failed to compute timestamp (error={:?})", error))?
            .as_nanos();
        let dir_name: String = format!("nanvix-uservm-args-test-{}", timestamp_nanos);
        let log_dir: PathBuf = base_dir.join(dir_name);
        if log_dir.exists() {
            fs::remove_dir_all(&log_dir).ok();
        }
        let log_dir_str: String = log_dir.to_string_lossy().into_owned();
        Ok((log_dir_str, log_dir))
    }

    #[test]
    fn parse_returns_expected_values() -> AnyResult<()> {
        let mut args_vec: Vec<String> = build_base_args();
        let (log_dir_str, log_dir_path) = unique_log_dir()?;
        args_vec.push(Args::OPT_INITRD.to_string());
        args_vec.push(String::from("initrd.img"));
        args_vec.push(Args::OPT_INITRD_ARGS.to_string());
        args_vec.push(String::from("--flag=value"));
        args_vec.push(Args::OPT_KERNEL_ARGS.to_string());
        args_vec.push(String::from("feature1 feature2"));
        args_vec.push(Args::OPT_RAMFS.to_string());
        args_vec.push(String::from("ramfs.img"));
        args_vec.push(Args::OPT_STDERR.to_string());
        args_vec.push(String::from("stderr.log"));
        args_vec.push(Args::OPT_LOGFILE.to_string());
        args_vec.push(Args::OPT_LOGDIR.to_string());
        args_vec.push(log_dir_str.clone());

        let mut parsed_args: Args = Args::parse(args_vec)?;

        assert_eq!(format!("{}", parsed_args.user_vm_id()), "7");
        assert_eq!(parsed_args.kernel_filename(), "kernel.elf");
        let initrd: Option<String> = parsed_args.initrd_filename();
        assert!(matches!(initrd, Some(ref value) if value == "initrd.img"));
        let ramfs: Option<String> = parsed_args.ramfs_filename();
        assert!(matches!(ramfs, Some(ref value) if value == "ramfs.img"));
        let initrd_args: Option<String> = parsed_args.initrd_args();
        assert!(matches!(initrd_args, Some(ref value) if value == "--flag=value"));
        let kernel_args: Option<String> = parsed_args.kernel_args();
        assert!(matches!(kernel_args, Some(ref value) if value == "feature1 feature2"));
        let stderr_path: Option<String> = parsed_args.take_vm_stderr();
        assert!(matches!(stderr_path, Some(ref value) if value == "stderr.log"));
        assert_eq!(parsed_args.system_vm_addr(), "127.0.0.1:7000");
        assert_eq!(parsed_args.system_vm_socket_type(), "tcp");
        assert_eq!(parsed_args.control_plane_addr(), "127.0.0.1:8000");
        assert_eq!(parsed_args.control_plane_socket_type(), "tcp");
        assert_eq!(parsed_args.gateway_addr(), "127.0.0.1:9000");
        assert_eq!(parsed_args.gateway_socket_type(), "tcp");
        assert!(parsed_args.log_to_file());
        assert_eq!(parsed_args.log_directory(), log_dir_str);

        fs::remove_dir_all(log_dir_path).ok();

        Ok(())
    }

    #[test]
    fn parse_standalone_skips_socket_validation() -> AnyResult<()> {
        let args_vec: Vec<String> = vec![
            String::from("uservm"),
            Args::OPT_KERNEL.to_string(),
            String::from("kernel.elf"),
            Args::OPT_STANDALONE.to_string(),
        ];

        let parsed_args: Args = Args::parse(args_vec)?;

        assert!(parsed_args.standalone(), "standalone flag should be true");
        assert_eq!(format!("{}", parsed_args.user_vm_id()), "0");
        assert_eq!(parsed_args.kernel_filename(), "kernel.elf");
        assert!(parsed_args.system_vm_addr().is_empty());
        assert!(parsed_args.control_plane_addr().is_empty());
        assert!(parsed_args.gateway_addr().is_empty());

        Ok(())
    }

    #[test]
    fn parse_non_standalone_requires_socket_addrs() {
        let args_vec: Vec<String> = vec![
            String::from("uservm"),
            Args::OPT_USER_VM_ID.to_string(),
            String::from("0"),
            Args::OPT_KERNEL.to_string(),
            String::from("kernel.elf"),
        ];

        let result = Args::parse(args_vec);
        assert!(result.is_err(), "non-standalone mode should require socket addresses");
    }

    #[test]
    fn parse_standalone_is_false_by_default() -> AnyResult<()> {
        let args_vec: Vec<String> = build_base_args();
        let parsed_args: Args = Args::parse(args_vec)?;
        assert!(!parsed_args.standalone(), "standalone should default to false");
        Ok(())
    }

    #[cfg(feature = "gdb")]
    #[test]
    fn parse_gdb_port_success() -> AnyResult<()> {
        let args_vec: Vec<String> = vec![
            String::from("uservm"),
            Args::OPT_KERNEL.to_string(),
            String::from("kernel.elf"),
            Args::OPT_STANDALONE.to_string(),
            Args::OPT_GDB_PORT.to_string(),
            String::from("1234"),
        ];

        let parsed_args: Args = Args::parse(args_vec)?;
        assert_eq!(parsed_args.gdb_port(), Some(1234));
        Ok(())
    }

    #[cfg(feature = "gdb")]
    #[test]
    fn parse_gdb_port_invalid_value() {
        let args_vec: Vec<String> = vec![
            String::from("uservm"),
            Args::OPT_KERNEL.to_string(),
            String::from("kernel.elf"),
            Args::OPT_STANDALONE.to_string(),
            Args::OPT_GDB_PORT.to_string(),
            String::from("not-a-number"),
        ];

        let result = Args::parse(args_vec);
        assert!(result.is_err(), "invalid port value should fail");
    }

    #[cfg(feature = "gdb")]
    #[test]
    fn parse_gdb_port_missing_value() {
        let args_vec: Vec<String> = vec![
            String::from("uservm"),
            Args::OPT_KERNEL.to_string(),
            String::from("kernel.elf"),
            Args::OPT_STANDALONE.to_string(),
            Args::OPT_GDB_PORT.to_string(),
        ];

        let result = Args::parse(args_vec);
        assert!(result.is_err(), "missing port value should fail");
    }

    #[cfg(feature = "gdb")]
    #[test]
    fn parse_gdb_port_requires_standalone() {
        let mut args_vec: Vec<String> = build_base_args();
        args_vec.push(Args::OPT_GDB_PORT.to_string());
        args_vec.push(String::from("1234"));

        let result = Args::parse(args_vec);
        assert!(result.is_err(), "-gdb-port without -standalone should fail");
    }

    fn standalone_base_args() -> Vec<String> {
        vec![
            String::from("uservm"),
            Args::OPT_KERNEL.to_string(),
            String::from("kernel.elf"),
            Args::OPT_STANDALONE.to_string(),
        ]
    }

    #[test]
    fn parse_networking_defaults_to_disabled() -> AnyResult<()> {
        let parsed_args: Args = Args::parse(standalone_base_args())?;
        assert!(!parsed_args.host_networking_enabled(), "host networking should default off");
        assert!(parsed_args.networkd_addr().is_none(), "networkd address should default to none");
        assert!(parsed_args.networkd_socket_type().is_none(), "socket type should default to none");
        Ok(())
    }

    #[test]
    fn parse_allow_host_networking_enables_in_process() -> AnyResult<()> {
        let mut args_vec: Vec<String> = standalone_base_args();
        args_vec.push(Args::OPT_ALLOW_HOST_NETWORKING.to_string());

        let parsed_args: Args = Args::parse(args_vec)?;
        assert!(parsed_args.host_networking_enabled(), "host networking should be enabled");
        assert!(
            parsed_args.networkd_addr().is_none(),
            "no address means the network daemon runs in-process"
        );
        Ok(())
    }

    #[test]
    fn parse_decoupled_networkd_endpoint() -> AnyResult<()> {
        let mut args_vec: Vec<String> = standalone_base_args();
        args_vec.push(Args::OPT_ALLOW_HOST_NETWORKING.to_string());
        args_vec.push(Args::OPT_NETWORKD_ADDR.to_string());
        args_vec.push(String::from("/tmp/networkd.sock"));
        args_vec.push(Args::OPT_NETWORKD_SOCKET_TYPE.to_string());
        args_vec.push(String::from("unix"));

        let parsed_args: Args = Args::parse(args_vec)?;
        assert!(parsed_args.host_networking_enabled());
        assert_eq!(parsed_args.networkd_addr(), Some("/tmp/networkd.sock"));
        assert_eq!(parsed_args.networkd_socket_type(), Some("unix"));
        Ok(())
    }

    #[test]
    fn parse_networkd_addr_requires_host_networking() {
        let mut args_vec: Vec<String> = standalone_base_args();
        args_vec.push(Args::OPT_NETWORKD_ADDR.to_string());
        args_vec.push(String::from("/tmp/networkd.sock"));

        let result = Args::parse(args_vec);
        assert!(result.is_err(), "-networkd-addr without -allow-host-networking should fail");
    }

    #[test]
    fn parse_networkd_socket_type_requires_addr() {
        let mut args_vec: Vec<String> = standalone_base_args();
        args_vec.push(Args::OPT_ALLOW_HOST_NETWORKING.to_string());
        args_vec.push(Args::OPT_NETWORKD_SOCKET_TYPE.to_string());
        args_vec.push(String::from("unix"));

        let result = Args::parse(args_vec);
        assert!(result.is_err(), "-networkd-socket-type without -networkd-addr should fail");
    }

    #[test]
    fn parse_networkd_socket_type_rejects_unknown() {
        let mut args_vec: Vec<String> = standalone_base_args();
        args_vec.push(Args::OPT_ALLOW_HOST_NETWORKING.to_string());
        args_vec.push(Args::OPT_NETWORKD_ADDR.to_string());
        args_vec.push(String::from("/tmp/networkd.sock"));
        args_vec.push(Args::OPT_NETWORKD_SOCKET_TYPE.to_string());
        args_vec.push(String::from("carrier-pigeon"));

        let result = Args::parse(args_vec);
        assert!(result.is_err(), "unknown networkd socket type should fail");
    }

    #[test]
    fn parse_allow_host_networking_requires_standalone() {
        let mut args_vec: Vec<String> = build_base_args();
        args_vec.push(Args::OPT_ALLOW_HOST_NETWORKING.to_string());

        let result = Args::parse(args_vec);
        assert!(result.is_err(), "-allow-host-networking without -standalone should fail");
    }
}
