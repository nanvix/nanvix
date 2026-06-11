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

#[cfg(feature = "single-process")]
use crate::config::DEFAULT_CONSOLE_FILENAME;
use crate::config::{
    self,
    DEFAULT_TMP_DIRECTORY,
};
use ::anyhow::Result;
#[cfg(feature = "single-process")]
use ::chrono::Local;
use ::nanvix::{
    hwloc::HwLoc,
    log::DEFAULT_LOG_DIRECTORY,
    sandbox_config::{
        HostFilter,
        Ipv4Cidr,
        NetworkingMode,
    },
    syscomm::SocketType,
};
use ::std::{
    fs::File,
    io::BufReader,
};

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
    /// Optional HTTP server socket address (host:port). If present, enables HTTP mode.
    http_sockaddr: Option<String>,
    /// Directory path containing Nanvix binaries.
    binary_directory: String,
    /// Path to the cloud-hypervisor binary directory.
    clh_bin_path: String,
    /// Optional file path for redirecting console output.
    console_file: Option<String>,
    /// Optional RAM filesystem image exposed to the guest.
    ramfs_filename: Option<String>,
    /// Optional hardware locality configuration for CPU affinity and topology.
    hwloc: Option<HwLoc>,
    /// Number of network namespaces to prefill in the pool (0 enables lazy initialization).
    netns_pool_size: usize,
    /// Directory path for writing log files when log_to_file is enabled.
    log_directory: String,
    /// Flag indicating whether to deploy linuxd inside an L2 VM.
    l2: bool,
    /// File path for the L2 snapshot.
    l2_snapshot_path: String,
    /// Optional socket type for control plane communication (nanvixd <-> linuxd).
    control_plane_socket_type: Option<SocketType>,
    /// Optional socket type for gateway communication (client <-> linuxd stdin/stdout).
    gateway_socket_type: Option<SocketType>,
    /// Optional socket type for system VM communication (linuxd <-> uservm).
    system_vm_socket_type: Option<SocketType>,
    /// Program name for interactive mode (first word after `--` separator).
    program_name: Option<String>,
    /// Program arguments for interactive mode (remaining words after `--` separator).
    program_args: Vec<String>,
    /// Base directory path for creating temporary directories.
    tmp_directory: String,
    /// Optional snapshot path: when set, restore from snapshot instead of cold-booting.
    snapshot_path: Option<String>,
    /// Optional host directory to mount on the guest (standalone mode only).
    mount_directory: Option<String>,
    /// Optional kernel arguments written to guest control registers (standalone mode only).
    kernel_args: Option<String>,
    /// Optional GDB server port: when set, the uservm starts a GDB RSP server on this TCP port.
    #[cfg(feature = "gdb")]
    gdb_port: Option<u16>,
    /// Networking mode (applies to all deployment modes).
    networking_mode: NetworkingMode,
    /// Allowlist of IPv4/CIDR destinations (`-allow-host`). When non-empty, only
    /// these destinations are reachable. Mutually exclusive with `block_hosts`.
    allow_hosts: Vec<String>,
    /// Blocklist of IPv4/CIDR destinations (`-block-host`). When non-empty, all
    /// destinations except these are reachable. Mutually exclusive with `allow_hosts`.
    block_hosts: Vec<String>,
    /// When `true`, route nanvixd's logs to stdout instead of a auto-named file.
    log_to_stdout: bool,
    /// Optional path of the standalone gateway endpoint -- the host-side
    /// rendezvous point where a consumer reads the guest's stdout/stderr
    /// and writes to its stdin. UDS path on Unix, named pipe path on
    /// Windows. Defaults to a per-process auto path when omitted.
    gateway_sockaddr: Option<String>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line flag that prints usage information.
    pub const OPT_HELP: &'static str = "-help";
    /// Command-line option that sets the HTTP socket address.
    pub const OPT_HTTP_SOCKADDR: &'static str = "-http-addr";
    /// Command-line option that sets the binary directory path.
    pub const OPT_BIN_DIRECTORY: &'static str = "-bin-dir";
    /// Command-line option that sets the cloud-hypervisor binary directory path.
    pub const OPT_CLH_BIN_PATH: &'static str = "-clh-bin-path";
    /// Command-line option that sets the L2 snapshot path.
    pub const OPT_L2_SNAPSHOT_PATH: &'static str = "-l2-snapshot-path";
    /// Command-line option that redirects the console output to a file.
    pub const OPT_CONSOLE_FILE: &'static str = "-console-file";
    /// Command-line option that loads the serialized CPU topology.
    pub const OPT_HWLOC: &'static str = "-hwloc";
    /// Command-line option that sets the log directory path.
    pub const OPT_LOG_DIRECTORY: &'static str = "-log-dir";
    /// Command-line option that sets the network namespace pool size.
    pub const OPT_NETNS_POOL_SIZE: &'static str = "-netns-pool-size";
    /// Command-line option that sets the RAM filesystem image filename.
    pub const OPT_RAMFS_FILENAME: &'static str = "-ramfs";
    /// Default netns pool size for prefill mode.
    pub const DEFAULT_NETNS_POOL_SIZE: usize = 128;
    /// Command-line flag that enables L2 deployment mode.
    pub const OPT_L2: &'static str = "-l2";
    /// Command-line option that sets the control plane socket type.
    pub const OPT_CONTROL_PLANE_SOCKET_TYPE: &'static str = "-control-plane-socket-type";
    /// Command-line option that sets the gateway socket type.
    pub const OPT_GATEWAY_SOCKET_TYPE: &'static str = "-gateway-socket-type";
    /// Command-line option that sets the system VM socket type.
    pub const OPT_SYSTEM_VM_SOCKET_TYPE: &'static str = "-system-vm-socket-type";
    /// Command-line separator for interactive mode program and arguments.
    pub const OPT_SEPARATOR: &'static str = "--";
    /// Command-line option that sets the base temporary directory path.
    pub const OPT_TMP_DIRECTORY: &'static str = "-tmp-dir";
    /// Command-line option for snapshot path.
    pub const OPT_SNAPSHOT: &'static str = "-snapshot";
    /// Command-line option for host directory to mount on the guest (standalone mode only).
    pub const OPT_MOUNT_DIRECTORY: &'static str = "-mount";
    /// Command-line option for kernel arguments (standalone mode only).
    pub const OPT_KERNEL_ARGS: &'static str = "-kernel-args";
    /// Command-line option for GDB server port (standalone mode only).
    #[cfg(feature = "gdb")]
    pub const OPT_GDB_PORT: &'static str = "-gdb-port";
    /// Command-line flag that enables host networking for the guest.
    pub const OPT_ALLOW_HOST_NETWORKING: &'static str = "-allow-host-networking";
    /// Command-line option (repeatable) adding an IPv4/CIDR to the egress allowlist.
    pub const OPT_ALLOW_HOST: &'static str = "-allow-host";
    /// Command-line option (repeatable) adding an IPv4/CIDR to the egress blocklist.
    pub const OPT_BLOCK_HOST: &'static str = "-block-host";
    /// Command-line flag that routes nanvixd's logs to stdout instead of the auto-named file.
    pub const OPT_LOG_TO_STDOUT: &'static str = "-log-to-stdout";
    /// Command-line option for the standalone gateway endpoint (UDS
    /// path on Unix, named pipe path on Windows).
    pub const OPT_GATEWAY_SOCKADDR: &'static str = "-gateway-sockaddr";

    ///
    /// # Description
    ///
    /// Parses command-line arguments into an Args structure.
    ///
    /// This function processes all supported command-line flags and options, validates them,
    /// and enforces constraints such as requiring TCP sockets for L2 deployment mode.
    /// It also enforces mutual exclusivity between HTTP mode and interactive mode.
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
        let mut http_sockaddr: Option<String> = None;
        let mut binary_directory: String = config::DEFAULT_BIN_DIRECTORY.to_string();
        let mut clh_bin_path: String = config::DEFAULT_CLH_BIN_PATH.to_string();
        #[cfg(not(feature = "single-process"))]
        let mut console_file: Option<String> = None;
        #[cfg(feature = "single-process")]
        let mut console_file: Option<String> = Some(format!(
            "{}/{}_{}.log",
            DEFAULT_LOG_DIRECTORY,
            DEFAULT_CONSOLE_FILENAME,
            Local::now().format("%Y_%m_%d_%H_%M")
        ));
        let mut ramfs_filename: Option<String> = None;
        let mut hwloc: Option<HwLoc> = None;
        let mut netns_pool_size: usize = Self::DEFAULT_NETNS_POOL_SIZE;
        let mut log_directory: String = DEFAULT_LOG_DIRECTORY.to_string();
        let mut log_directory_set: bool = false;
        let mut l2: bool = false;
        let mut l2_snapshot_path: String = String::new();
        let mut control_plane_socket_type: Option<SocketType> = None;
        let mut gateway_socket_type: Option<SocketType> = None;
        let mut system_vm_socket_type: Option<SocketType> = None;
        let mut program_name: Option<String> = None;
        let mut program_args: Vec<String> = Vec::new();
        let mut tmp_directory: String = DEFAULT_TMP_DIRECTORY.to_string();
        let mut snapshot_path: Option<String> = None;
        let mut mount_directory: Option<String> = None;
        let mut kernel_args: Option<String> = None;
        #[cfg(feature = "gdb")]
        let mut gdb_port: Option<u16> = None;
        let mut networking_mode: NetworkingMode = NetworkingMode::Disabled;
        let mut allow_hosts: Vec<String> = Vec::new();
        let mut block_hosts: Vec<String> = Vec::new();
        let mut log_to_stdout: bool = false;
        let mut gateway_sockaddr: Option<String> = None;

        let mut i: usize = 1;
        while i < args.len() {
            // Check for separator.
            if args[i] == Self::OPT_SEPARATOR {
                i += 1;
                // The first word after separator is the program name.
                if i < args.len() {
                    program_name = Some(args[i].clone());
                    i += 1;
                    // Remaining words are program arguments.
                    while i < args.len() {
                        program_args.push(args[i].clone());
                        i += 1;
                    }
                }
                break;
            }
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    return Err(anyhow::anyhow!("wrong usage"));
                },
                Self::OPT_HTTP_SOCKADDR => {
                    i += 1;
                    http_sockaddr = Some(args[i].clone());
                },
                Self::OPT_BIN_DIRECTORY => {
                    i += 1;
                    binary_directory = args[i].clone();
                },
                Self::OPT_CLH_BIN_PATH => {
                    i += 1;
                    clh_bin_path = args[i].clone();
                },
                Self::OPT_CONSOLE_FILE => {
                    i += 1;
                    console_file = Some(args[i].clone());
                },
                Self::OPT_GATEWAY_SOCKADDR => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "missing value for: {}",
                            Self::OPT_GATEWAY_SOCKADDR
                        ));
                    }
                    gateway_sockaddr = Some(args[i].clone());
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
                Self::OPT_L2_SNAPSHOT_PATH => {
                    i += 1;
                    l2_snapshot_path = args[i].clone();
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
                Self::OPT_LOG_DIRECTORY => {
                    i += 1;
                    log_directory = args[i].clone();
                    log_directory_set = true;
                },
                Self::OPT_NETNS_POOL_SIZE => {
                    i += 1;
                    netns_pool_size = args[i].parse()?;
                },
                Self::OPT_RAMFS_FILENAME => {
                    i += 1;
                    ramfs_filename = Some(args[i].clone());
                },
                Self::OPT_TMP_DIRECTORY => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "missing value for: {}",
                            Self::OPT_TMP_DIRECTORY
                        ));
                    }
                    tmp_directory = args[i].clone();
                },
                Self::OPT_SNAPSHOT => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_SNAPSHOT));
                    }
                    let path: &str = &args[i];
                    let metadata = ::std::fs::metadata(path)
                        .map_err(|_| anyhow::anyhow!("snapshot path does not exist: {}", path))?;
                    if !metadata.is_file() {
                        return Err(anyhow::anyhow!("snapshot path is not a file: {}", path));
                    }
                    snapshot_path = Some(args[i].clone());
                },
                Self::OPT_MOUNT_DIRECTORY => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "missing value for: {}",
                            Self::OPT_MOUNT_DIRECTORY
                        ));
                    }
                    let path: &str = &args[i];
                    let metadata = ::std::fs::metadata(path)
                        .map_err(|_| anyhow::anyhow!("mount directory does not exist: {}", path))?;
                    if !metadata.is_dir() {
                        return Err(anyhow::anyhow!("mount path is not a directory: {}", path));
                    }
                    mount_directory = Some(args[i].clone());
                },
                Self::OPT_KERNEL_ARGS => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "missing value for: {}",
                            Self::OPT_KERNEL_ARGS
                        ));
                    }
                    kernel_args = Some(args[i].clone());
                },
                // Set GDB server port (standalone mode only).
                #[cfg(feature = "gdb")]
                Self::OPT_GDB_PORT => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_GDB_PORT));
                    }
                    gdb_port = Some(args[i].parse::<u16>().map_err(|e| {
                        anyhow::anyhow!("invalid GDB port (arg={}, error={e:?})", args[i])
                    })?);
                },
                Self::OPT_ALLOW_HOST_NETWORKING => {
                    networking_mode = NetworkingMode::Enabled;
                },
                Self::OPT_ALLOW_HOST => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_ALLOW_HOST));
                    }
                    if Ipv4Cidr::parse(&args[i]).is_none() {
                        return Err(anyhow::anyhow!(
                            "invalid {} value (expected IPv4 or CIDR): {}",
                            Self::OPT_ALLOW_HOST,
                            args[i]
                        ));
                    }
                    allow_hosts.push(args[i].clone());
                },
                Self::OPT_BLOCK_HOST => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_BLOCK_HOST));
                    }
                    if Ipv4Cidr::parse(&args[i]).is_none() {
                        return Err(anyhow::anyhow!(
                            "invalid {} value (expected IPv4 or CIDR): {}",
                            Self::OPT_BLOCK_HOST,
                            args[i]
                        ));
                    }
                    block_hosts.push(args[i].clone());
                },
                Self::OPT_LOG_TO_STDOUT => {
                    log_to_stdout = true;
                },
                arg => {
                    return Err(anyhow::anyhow!("invalid argument: {arg}"));
                },
            }

            i += 1;
        }

        // -log-to-stdout and -log-dir are mutually exclusive: -log-to-stdout routes nanvixd's
        // logs to stdout, making an explicit log directory meaningless.
        if log_to_stdout && log_directory_set {
            anyhow::bail!(
                "{} and {} are mutually exclusive",
                Self::OPT_LOG_TO_STDOUT,
                Self::OPT_LOG_DIRECTORY,
            );
        }

        // Host egress filtering is all-or-list, never both: an allowlist
        // (deny-by-default) and a blocklist (allow-by-default) express opposite
        // postures and cannot be combined.
        if !allow_hosts.is_empty() && !block_hosts.is_empty() {
            anyhow::bail!(
                "{} and {} are mutually exclusive",
                Self::OPT_ALLOW_HOST,
                Self::OPT_BLOCK_HOST,
            );
        }

        // A host filter is meaningless without host networking enabled -- the
        // guest has no egress to filter. Reject rather than silently ignore.
        if (!allow_hosts.is_empty() || !block_hosts.is_empty()) && !networking_mode.is_enabled() {
            anyhow::bail!(
                "{} / {} require {}",
                Self::OPT_ALLOW_HOST,
                Self::OPT_BLOCK_HOST,
                Self::OPT_ALLOW_HOST_NETWORKING,
            );
        }

        // Host egress filtering is only consulted on the standalone network
        // daemon path. In single-/multi-process builds the flags would be
        // silently ignored, giving a false sense of policy -- reject them so the
        // operator is not misled into believing egress is restricted.
        #[cfg(not(feature = "standalone"))]
        if !allow_hosts.is_empty() || !block_hosts.is_empty() {
            anyhow::bail!(
                "{} / {} are only supported in standalone builds",
                Self::OPT_ALLOW_HOST,
                Self::OPT_BLOCK_HOST,
            );
        }

        // If we set the l2 snapshot path, but do not enable l2, we have an invalid configuration.
        if !l2_snapshot_path.is_empty() && !l2 {
            anyhow::bail!(
                "{} must be used together with {}",
                Self::OPT_L2_SNAPSHOT_PATH,
                Self::OPT_L2,
            );
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

            // If we enable L2 deployment, and don't set a snapshot path, revert to the default
            // path.
            if l2_snapshot_path.is_empty() {
                l2_snapshot_path = config::default_l2_snapshot_path();
            }
        }

        // Reject -snapshot in non-standalone builds where it would silently do nothing.
        #[cfg(not(feature = "standalone"))]
        if snapshot_path.is_some() {
            anyhow::bail!("{} is only supported in standalone builds", Self::OPT_SNAPSHOT);
        }

        // Reject -mount in non-standalone builds.
        #[cfg(not(feature = "standalone"))]
        if mount_directory.is_some() {
            anyhow::bail!("{} is only supported in standalone builds", Self::OPT_MOUNT_DIRECTORY);
        }

        // Reject -kernel-args in non-standalone builds.
        #[cfg(not(feature = "standalone"))]
        if kernel_args.is_some() {
            anyhow::bail!("{} is only supported in standalone builds", Self::OPT_KERNEL_ARGS);
        }

        // Determine operation mode: HTTP mode is active if -http-addr is provided,
        // interactive mode is active if `--` separator with program name is provided.
        let http_mode: bool = http_sockaddr.is_some();
        let interactive_mode: bool = program_name.is_some();

        // Ensure exactly one mode is active.
        if http_mode && interactive_mode {
            anyhow::bail!(
                "cannot use both HTTP mode ({}) and interactive mode ({}) simultaneously",
                Self::OPT_HTTP_SOCKADDR,
                Self::OPT_SEPARATOR
            );
        }

        if !http_mode && !interactive_mode {
            anyhow::bail!(
                "must specify either HTTP mode ({} <sockaddr>) or interactive mode ({} <program> \
                 [<args>...])",
                Self::OPT_HTTP_SOCKADDR,
                Self::OPT_SEPARATOR
            );
        }

        Ok(Self {
            http_sockaddr,
            binary_directory,
            clh_bin_path,
            l2_snapshot_path,
            console_file,
            ramfs_filename,
            hwloc,
            netns_pool_size,
            log_directory,
            l2,
            control_plane_socket_type,
            gateway_socket_type,
            system_vm_socket_type,
            program_name,
            program_args,
            tmp_directory,
            snapshot_path,
            mount_directory,
            kernel_args,
            #[cfg(feature = "gdb")]
            gdb_port,
            networking_mode,
            allow_hosts,
            block_hosts,
            log_to_stdout,
            gateway_sockaddr,
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
        let http_usage: String = format!(
            "\nUsage (HTTP mode):\n  {program_name} {} <sockaddr> [OPTIONS]\n",
            Self::OPT_HTTP_SOCKADDR,
        );

        println!(
            "\
Nanvix Daemon - System-level service and VM orchestration daemon for Nanvix OS.
{http_usage}
Usage (Interactive mode):
  {program_name} [OPTIONS] {separator} <program> [<args>...]

Options:
  {console_file} <file>                     Redirect console output to a file.
  {ramfs_filename} <file>                   Attach a RAM filesystem image to spawned user VMs.
  {bin_dir} <bin_dir>                       Directory containing Nanvix binaries.
  {clh_bin_path} <clh_bin_path>             Path to the cloud-hypervisor binary directory.
  {hwloc} <hwloc.json>                      Hardware locality configuration file for CPU \
             affinity/topology.
  {log_dir} <log_dir>                       Directory for log files (Default: \
             {DEFAULT_LOG_DIRECTORY}).
  {netns_pool_size} <size>                  Netns pool prefill size (Default: \
             {default_netns_pool_size}; 0 enables lazy initialization).
  {control_plane_socket_type} <socket_type> Socket type for control plane communication (nanvixd \
             <-> linuxd).
  {gateway_socket_type} <socket_type>       Socket type for gateway communication (client <-> \
             linuxd).
  {system_vm_socket_type} <socket_type>     Socket type for system VM communication (linuxd <-> \
             uservm).
  {l2}                                      Deploy linuxd inside an L2 VM (forces TCP sockets).
  {l2_snapshot_path} <l2_snapshot_path>     Path to the L2 snapshot.
  {tmp_dir} <tmp_dir>                       Base directory for temporary files (Default: \
             {DEFAULT_TMP_DIRECTORY}).
  {snapshot} <path>                         Restore VM from snapshot instead of cold-booting \
             (standalone mode only).
  {mount} <host-dir>                       Mount a host directory on the guest at /mnt (standalone \
             mode only).
  {kernel_args} <args>                      Pass kernel arguments to guest control registers \
             (standalone mode only).
  {allow_host_networking}                   Enable host networking for the guest (disabled when \
             omitted).
  {allow_host} <ip|cidr>                    (Repeatable) Permit egress only to this IPv4/CIDR \
             (allowlist; requires {allow_host_networking}; mutually exclusive with {block_host}).
  {block_host} <ip|cidr>                    (Repeatable) Deny egress to this IPv4/CIDR (blocklist; \
             requires {allow_host_networking}; mutually exclusive with {allow_host}).
  {log_to_stdout}                          Route nanvixd's own logrus output to stdout instead of \
             a file in {log_dir} (file logger is otherwise the default).
  {gateway_sockaddr} <path>                 (Standalone) Path at which to expose the gateway \
             endpoint -- the host-side rendezvous point where a consumer (e.g. the containerd \
             shim) reads the guest's stdout/stderr and writes to its stdin. UDS path on Unix, \
             named pipe path on Windows. Defaults to a per-process auto path when \
             omitted.{gdb_port_line}
",
            http_usage = http_usage,
            program_name = program_name,
            separator = Self::OPT_SEPARATOR,
            console_file = Self::OPT_CONSOLE_FILE,
            ramfs_filename = Self::OPT_RAMFS_FILENAME,
            bin_dir = Self::OPT_BIN_DIRECTORY,
            clh_bin_path = Self::OPT_CLH_BIN_PATH,
            hwloc = Self::OPT_HWLOC,
            log_dir = Self::OPT_LOG_DIRECTORY,
            netns_pool_size = Self::OPT_NETNS_POOL_SIZE,
            default_netns_pool_size = Self::DEFAULT_NETNS_POOL_SIZE,
            control_plane_socket_type = Self::OPT_CONTROL_PLANE_SOCKET_TYPE,
            gateway_socket_type = Self::OPT_GATEWAY_SOCKET_TYPE,
            system_vm_socket_type = Self::OPT_SYSTEM_VM_SOCKET_TYPE,
            l2 = Self::OPT_L2,
            l2_snapshot_path = Self::OPT_L2_SNAPSHOT_PATH,
            tmp_dir = Self::OPT_TMP_DIRECTORY,
            snapshot = Self::OPT_SNAPSHOT,
            mount = Self::OPT_MOUNT_DIRECTORY,
            kernel_args = Self::OPT_KERNEL_ARGS,
            allow_host_networking = Self::OPT_ALLOW_HOST_NETWORKING,
            allow_host = Self::OPT_ALLOW_HOST,
            block_host = Self::OPT_BLOCK_HOST,
            log_to_stdout = Self::OPT_LOG_TO_STDOUT,
            gateway_sockaddr = Self::OPT_GATEWAY_SOCKADDR,
            gdb_port_line = if cfg!(feature = "gdb") {
                "\n  -gdb-port <port>                         GDB server port (standalone mode \
                 only)."
            } else {
                ""
            },
        );
    }

    ///
    /// # Description
    ///
    /// Returns the HTTP socket address if HTTP mode is enabled.
    ///
    /// # Returns
    ///
    /// The HTTP socket address if present; `None` otherwise.
    ///
    pub fn http_sockaddr(&self) -> Option<&str> {
        self.http_sockaddr.as_deref()
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
    /// Returns the cloud-hypervisor binary directory path.
    ///
    /// # Returns
    ///
    /// The cloud-hypervisor binary directory path.
    ///
    pub fn clh_bin_path(&self) -> &str {
        &self.clh_bin_path
    }

    ///
    /// # Description
    ///
    /// Returns the L2 snapshot path.
    ///
    /// # Returns
    ///
    /// The L2 snapshot path.
    ///
    pub fn l2_snapshot_path(&self) -> &str {
        &self.l2_snapshot_path
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

    /// Returns the optional standalone gateway endpoint path (UDS on
    /// Unix, named pipe on Windows). See [`Self::OPT_GATEWAY_SOCKADDR`].
    pub fn gateway_sockaddr(&self) -> Option<&str> {
        self.gateway_sockaddr.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional RAM filesystem filename.
    ///
    /// # Returns
    ///
    /// The RAM filesystem filename, if present.
    ///
    pub fn ramfs_filename(&self) -> Option<&str> {
        self.ramfs_filename.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional snapshot path.
    ///
    /// # Returns
    ///
    /// The snapshot path, if present.
    ///
    pub fn snapshot_path(&self) -> Option<&str> {
        self.snapshot_path.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional host directory to mount on the guest.
    ///
    /// # Returns
    ///
    /// The mount directory path, if present.
    ///
    pub fn mount_directory(&self) -> Option<&str> {
        self.mount_directory.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional kernel arguments string.
    ///
    /// # Returns
    ///
    /// The kernel arguments string, if present.
    ///
    pub fn kernel_args(&self) -> Option<&str> {
        self.kernel_args.as_deref()
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
    /// Returns the netns pool prefill size.
    ///
    /// # Returns
    ///
    /// The prefill size (0 for lazy initialization).
    ///
    pub fn netns_pool_size(&self) -> usize {
        self.netns_pool_size
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

    ///
    /// # Description
    ///
    /// Indicates whether interactive mode is enabled.
    ///
    /// # Returns
    ///
    /// `true` if interactive mode is enabled; `false` otherwise.
    ///
    pub fn interactive_mode(&self) -> bool {
        self.program_name.is_some()
    }

    ///
    /// # Description
    ///
    /// Returns the program name for interactive mode.
    ///
    /// # Returns
    ///
    /// The program name if specified; `None` otherwise.
    ///
    pub fn program_name(&self) -> Option<&str> {
        self.program_name.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the program arguments for interactive mode.
    ///
    /// # Returns
    ///
    /// A reference to the vector of program arguments.
    ///
    pub fn program_args(&self) -> &[String] {
        &self.program_args
    }

    ///
    /// # Description
    ///
    /// Returns the base temporary directory path.
    ///
    /// # Returns
    ///
    /// The base temporary directory path.
    ///
    pub fn tmp_directory(&self) -> &str {
        &self.tmp_directory
    }

    ///
    /// # Description
    ///
    /// Returns the GDB server port.
    ///
    /// # Returns
    ///
    /// The GDB server port, if specified.
    ///
    #[cfg(feature = "gdb")]
    pub fn gdb_port(&self) -> Option<u16> {
        self.gdb_port
    }

    /// Returns the networking mode.
    pub fn networking_mode(&self) -> NetworkingMode {
        self.networking_mode
    }

    /// Returns the host egress filter built from the `-allow-host` /
    /// `-block-host` lists. Returns [`HostFilter::AllowAll`] when neither is set.
    pub fn host_filter(&self) -> HostFilter {
        HostFilter::from_lists(&self.allow_hosts, &self.block_hosts)
    }

    /// When `true`, nanvixd should route its logrus output to stdout
    /// instead of the file logger. See [`Self::OPT_LOG_TO_STDOUT`].
    pub fn log_to_stdout(&self) -> bool {
        self.log_to_stdout
    }
}

//==================================================================================================
// Unit tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(extras: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = vec!["nanvixd".to_string()];
        for s in extras {
            v.push((*s).to_string());
        }
        v
    }

    #[test]
    fn log_to_stdout_defaults_to_false() {
        let args = Args::parse(argv(&["--", "/bin/foo"])).expect("parse");
        assert!(!args.log_to_stdout());
    }

    #[test]
    fn log_to_stdout_flag_sets_true() {
        let args = Args::parse(argv(&["-log-to-stdout", "--", "/bin/foo"])).expect("parse");
        assert!(args.log_to_stdout());
    }

    #[test]
    fn log_to_stdout_composes_with_interactive_mode() {
        let args = Args::parse(argv(&["-log-to-stdout", "--", "/bin/foo", "arg1"])).expect("parse");
        assert!(args.log_to_stdout());
        assert!(args.interactive_mode());
        assert_eq!(args.program_name(), Some("/bin/foo"));
    }

    #[test]
    fn log_to_stdout_and_log_dir_are_mutually_exclusive() {
        let err = Args::parse(argv(&[
            "-log-to-stdout",
            "-log-dir",
            "/tmp/somewhere",
            "--",
            "/bin/foo",
        ]))
        .expect_err("parse should fail when both -log-to-stdout and -log-dir are provided");
        let msg = format!("{err}");
        assert!(msg.contains(Args::OPT_LOG_TO_STDOUT), "unexpected error: {msg}");
        assert!(msg.contains(Args::OPT_LOG_DIRECTORY), "unexpected error: {msg}");
    }

    #[test]
    fn log_dir_alone_is_accepted() {
        let args =
            Args::parse(argv(&["-log-dir", "/tmp/somewhere", "--", "/bin/foo"])).expect("parse");
        assert!(!args.log_to_stdout());
        assert_eq!(args.log_directory(), "/tmp/somewhere");
    }

    #[test]
    fn parses_http_mode_with_sockaddr() {
        let args = Args::parse(argv(&["-http-addr", "127.0.0.1:9999"])).expect("parse");
        assert_eq!(args.http_sockaddr(), Some("127.0.0.1:9999"));
        assert!(!args.interactive_mode());
        assert_eq!(args.program_name(), None);
    }

    #[test]
    fn parses_interactive_mode() {
        let args = Args::parse(argv(&["--", "/bin/foo", "arg1"])).expect("parse");
        assert_eq!(args.http_sockaddr(), None);
        assert!(args.interactive_mode());
        assert_eq!(args.program_name(), Some("/bin/foo"));
    }

    #[test]
    fn rejects_both_http_and_interactive() {
        let res = Args::parse(argv(&["-http-addr", "127.0.0.1:9999", "--", "/bin/foo"]));
        assert!(res.is_err(), "expected error when both modes are set");
        let msg: String = format!("{:#}", res.err().unwrap());
        assert!(msg.contains("cannot use both HTTP mode"), "unexpected error: {msg}");
    }

    #[test]
    fn rejects_neither_http_nor_interactive() {
        let res = Args::parse(argv(&[]));
        assert!(res.is_err(), "expected error when no mode is set");
        let msg: String = format!("{:#}", res.err().unwrap());
        assert!(msg.contains("must specify either HTTP mode"), "unexpected error: {msg}");
    }

    #[test]
    fn parses_gateway_sockaddr_flag() {
        let args = Args::parse(argv(&[
            "-http-addr",
            "127.0.0.1:9999",
            "-gateway-sockaddr",
            "/tmp/test-gw.sock",
        ]))
        .expect("parse");
        assert_eq!(args.gateway_sockaddr(), Some("/tmp/test-gw.sock"));
    }

    #[test]
    fn gateway_sockaddr_is_none_when_unset() {
        let args = Args::parse(argv(&["-http-addr", "127.0.0.1:9999"])).expect("parse");
        assert_eq!(args.gateway_sockaddr(), None);
    }

    #[test]
    fn rejects_gateway_sockaddr_without_value() {
        let res = Args::parse(argv(&["-gateway-sockaddr"]));
        assert!(res.is_err(), "expected error when -gateway-sockaddr has no value");
        let msg: String = format!("{:#}", res.err().unwrap());
        assert!(msg.contains("missing value for: -gateway-sockaddr"), "unexpected error: {msg}");
    }

    #[test]
    fn allow_host_builds_allowlist_filter() {
        let args = Args::parse(argv(&[
            "-allow-host-networking",
            "-allow-host",
            "1.2.3.4",
            "-allow-host",
            "10.0.0.0/8",
            "--",
            "/bin/foo",
        ]))
        .expect("parse");
        let filter = args.host_filter();
        assert!(filter.permits([1, 2, 3, 4]));
        assert!(filter.permits([10, 9, 8, 7]));
        assert!(!filter.permits([8, 8, 8, 8]));
    }

    #[test]
    fn block_host_builds_blocklist_filter() {
        let args = Args::parse(argv(&[
            "-allow-host-networking",
            "-block-host",
            "8.8.8.8",
            "--",
            "/bin/foo",
        ]))
        .expect("parse");
        let filter = args.host_filter();
        assert!(!filter.permits([8, 8, 8, 8]));
        assert!(filter.permits([1, 2, 3, 4]));
    }

    #[test]
    fn allow_and_block_host_are_mutually_exclusive() {
        let res = Args::parse(argv(&[
            "-allow-host-networking",
            "-allow-host",
            "1.2.3.4",
            "-block-host",
            "8.8.8.8",
            "--",
            "/bin/foo",
        ]));
        let msg: String = format!("{:#}", res.expect_err("expected mutual-exclusion error"));
        assert!(msg.contains(Args::OPT_ALLOW_HOST), "unexpected error: {msg}");
        assert!(msg.contains(Args::OPT_BLOCK_HOST), "unexpected error: {msg}");
    }

    #[test]
    fn host_filter_requires_networking_enabled() {
        let res = Args::parse(argv(&["-allow-host", "1.2.3.4", "--", "/bin/foo"]));
        let msg: String = format!("{:#}", res.expect_err("expected requires-networking error"));
        assert!(msg.contains(Args::OPT_ALLOW_HOST_NETWORKING), "unexpected error: {msg}");
    }

    #[test]
    fn rejects_invalid_host_entry() {
        let res = Args::parse(argv(&[
            "-allow-host-networking",
            "-allow-host",
            "not-an-ip",
            "--",
            "/bin/foo",
        ]));
        let msg: String = format!("{:#}", res.expect_err("expected invalid-entry error"));
        assert!(msg.contains("invalid"), "unexpected error: {msg}");
    }

    #[test]
    fn host_filter_defaults_to_allow_all() {
        let args = Args::parse(argv(&["-allow-host-networking", "--", "/bin/foo"])).expect("parse");
        assert!(matches!(args.host_filter(), HostFilter::AllowAll));
    }
}
