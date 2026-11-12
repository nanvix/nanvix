// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Command-line argument parsing for Nanvix Daemon.
//!
//! This module provides functionality to parse and validate command-line arguments passed to
//! the Nanvix Daemon. It handles various configuration options including network settings,
//! directory paths and guest execution settings, which may also be loaded from a configuration
//! file (see [`Args::OPT_CONFIG_FILE`]).

//==================================================================================================
// Imports
//==================================================================================================

use crate::config::{
    self,
    ConfigFile,
};
use ::anyhow::Result;
use ::nanvix::{
    log::DEFAULT_LOG_DIRECTORY,
    sandbox_config::{
        HostFilter,
        Ipv4Cidr,
        NetworkingMode,
    },
};
use ::std::path::Path;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Stores parsed command-line arguments for nanvixd.
///
/// This structure holds all configuration parameters provided via command-line arguments and the
/// configuration file, including network settings, directory paths, hardware topology
/// information, socket types, and workload execution flags.
///
#[derive(Debug, Clone)]
pub struct Args {
    /// Optional path of the configuration file from which options were loaded.
    config_path: Option<String>,
    /// Optional HTTP server socket address (host:port). If present, enables HTTP mode.
    http_sockaddr: Option<String>,
    /// Directory path containing Nanvix binaries.
    binary_directory: String,
    /// Optional file path for redirecting console output.
    console_file: Option<String>,
    /// Optional RAM filesystem image exposed to the guest.
    ramfs_filename: Option<String>,
    /// Directory path for writing log files when log_to_file is enabled.
    log_directory: String,
    /// Program name for interactive mode (first word after `--` separator).
    program_name: Option<String>,
    /// Program arguments for interactive mode (remaining words after `--` separator).
    program_args: Vec<String>,
    /// Optional snapshot path: when set, restore from snapshot instead of cold-booting.
    snapshot_path: Option<String>,
    /// Optional host directory to mount on the guest.
    mount_directory: Option<String>,
    /// Optional kernel arguments written to guest control registers.
    kernel_args: Option<String>,
    /// Optional GDB server port: when set, the uservm starts a GDB RSP server on this TCP port.
    #[cfg(feature = "gdb")]
    gdb_port: Option<u16>,
    /// Networking mode.
    networking_mode: NetworkingMode,
    /// Host egress filter built from the `-allow-host` / `-block-host` rules.
    /// Every rule is validated at parse time, so this is always the complete
    /// policy the user asked for.
    host_filter: HostFilter,
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
    /// Command-line option that loads options from a configuration file.
    pub const OPT_CONFIG_FILE: &'static str = "-config";
    /// Command-line option that sets the HTTP socket address.
    pub const OPT_HTTP_SOCKADDR: &'static str = "-http-addr";
    /// Command-line option that sets the binary directory path.
    pub const OPT_BIN_DIRECTORY: &'static str = "-bin-dir";
    /// Command-line option that redirects the console output to a file.
    pub const OPT_CONSOLE_FILE: &'static str = "-console-file";
    /// Command-line option that sets the log directory path.
    pub const OPT_LOG_DIRECTORY: &'static str = "-log-dir";
    /// Command-line option that sets the RAM filesystem image filename.
    pub const OPT_RAMFS_FILENAME: &'static str = "-ramfs";
    /// Command-line separator for interactive mode program and arguments.
    pub const OPT_SEPARATOR: &'static str = "--";
    /// Command-line option for snapshot path.
    pub const OPT_SNAPSHOT: &'static str = "-snapshot";
    /// Command-line option for host directory to mount on the guest.
    pub const OPT_MOUNT_DIRECTORY: &'static str = "-mount";
    /// Command-line option for kernel arguments.
    pub const OPT_KERNEL_ARGS: &'static str = "-kernel-args";
    /// Command-line option for GDB server port.
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
    /// and enforces mutual exclusivity between HTTP mode and interactive mode.
    ///
    /// Options may also be loaded from a configuration file given with [`Self::OPT_CONFIG_FILE`].
    /// Options given on the command line take precedence over the configuration file, with the
    /// following refinements:
    ///
    /// - The operation mode and the log destination each count as a single option. Selecting
    ///   interactive mode on the command line overrides the `http_addr` key of the configuration
    ///   file, and [`Self::OPT_LOG_DIRECTORY`] or [`Self::OPT_LOG_TO_STDOUT`] override both the
    ///   `log_dir` and `log_to_stdout` keys.
    /// - Host rules accumulate: the rules of the configuration file apply along with the ones
    ///   given on the command line.
    /// - Flags that the configuration file turns on cannot be turned off on the command line.
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
        let mut config_path: Option<String> = None;
        let mut http_sockaddr: Option<String> = None;
        let mut binary_directory: Option<String> = None;
        let mut console_file: Option<String> = None;
        let mut ramfs_filename: Option<String> = None;
        let mut log_directory: String = DEFAULT_LOG_DIRECTORY.to_string();
        let mut log_directory_set: bool = false;
        let mut program_name: Option<String> = None;
        let mut program_args: Vec<String> = Vec::new();
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
                Self::OPT_CONFIG_FILE => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "missing value for: {}",
                            Self::OPT_CONFIG_FILE
                        ));
                    }
                    config_path = Some(args[i].clone());
                },
                Self::OPT_HTTP_SOCKADDR => {
                    i += 1;
                    http_sockaddr = Some(args[i].clone());
                },
                Self::OPT_BIN_DIRECTORY => {
                    i += 1;
                    binary_directory = Some(args[i].clone());
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
                Self::OPT_LOG_DIRECTORY => {
                    i += 1;
                    log_directory = args[i].clone();
                    log_directory_set = true;
                },
                Self::OPT_RAMFS_FILENAME => {
                    i += 1;
                    ramfs_filename = Some(args[i].clone());
                },
                Self::OPT_SNAPSHOT => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_SNAPSHOT));
                    }
                    config::validate_snapshot_path(&args[i])?;
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
                    config::validate_mount_directory(&args[i])?;
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
                // Set GDB server port.
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

        // Load the configuration file, if any. Options given on the command line take precedence
        // over the ones that it sets.
        let config_file: ConfigFile = match config_path.as_deref() {
            Some(path) => ConfigFile::load(Path::new(path))?,
            None => ConfigFile::default(),
        };

        // The operation mode is a single option: when the command line selects a mode, it
        // overrides the HTTP socket address of the configuration file.
        if http_sockaddr.is_none() && program_name.is_none() {
            http_sockaddr = config_file.http_addr;
        }

        // The log destination is a single option as well: when the command line sets one, it
        // overrides the one of the configuration file.
        if !log_to_stdout && !log_directory_set {
            log_to_stdout = config_file.log_to_stdout;
            if let Some(directory) = config_file.log_dir {
                log_directory = directory;
            }
        }

        let binary_directory: Option<String> = binary_directory.or(config_file.bin_dir);
        let console_file: Option<String> = console_file.or(config_file.console_file);
        let ramfs_filename: Option<String> = ramfs_filename.or(config_file.ramfs);
        let snapshot_path: Option<String> = snapshot_path.or(config_file.snapshot);
        let mount_directory: Option<String> = mount_directory.or(config_file.mount);
        let kernel_args: Option<String> = kernel_args.or(config_file.kernel_args);
        #[cfg(feature = "gdb")]
        let gdb_port: Option<u16> = gdb_port.or(config_file.gdb_port);
        let gateway_sockaddr: Option<String> = gateway_sockaddr.or(config_file.gateway_sockaddr);
        if config_file.allow_host_networking {
            networking_mode = NetworkingMode::Enabled;
        }

        // A host filter is meaningless without host networking enabled -- the
        // guest has no egress to filter. Reject rather than silently ignore.
        if !networking_mode.is_enabled() {
            if !allow_hosts.is_empty() || !block_hosts.is_empty() {
                anyhow::bail!(
                    "{} / {} require {}",
                    Self::OPT_ALLOW_HOST,
                    Self::OPT_BLOCK_HOST,
                    Self::OPT_ALLOW_HOST_NETWORKING,
                );
            }
            if !config_file.allow_hosts.is_empty() || !config_file.block_hosts.is_empty() {
                anyhow::bail!(
                    "allow_hosts / block_hosts of the configuration file require \
                     allow_host_networking or {}",
                    Self::OPT_ALLOW_HOST_NETWORKING,
                );
            }
        }

        // Host rules accumulate, like repeated command-line options: the rules of the
        // configuration file apply along with the ones given on the command line.
        let allow_hosts: Vec<String> = config_file
            .allow_hosts
            .into_iter()
            .chain(allow_hosts)
            .collect();
        let block_hosts: Vec<String> = config_file
            .block_hosts
            .into_iter()
            .chain(block_hosts)
            .collect();

        // Build the policy only after every rule has been validated above. Any
        // allow rule (`-allow-host` or `allow_hosts`) selects deny-by-default,
        // otherwise the default is allow; block rules (`-block-host` or
        // `block_hosts`) always deny and override allow rules, regardless of the
        // order in which they were given. With allow rules, the DNS exception is
        // opted into so name resolution keeps working for the allowed hosts,
        // matching the help text; it never overrides a block rule.
        let host_filter: HostFilter = HostFilter::from_lists(&allow_hosts, &block_hosts, true)?;

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
                "must specify either HTTP mode ({} <sockaddr>, or http_addr in the configuration \
                 file) or interactive mode ({} <program> [<args>...])",
                Self::OPT_HTTP_SOCKADDR,
                Self::OPT_SEPARATOR
            );
        }

        // Unless overridden, use the directory that contains this executable, so that Nanvix
        // binaries shipped alongside it are picked up regardless of the working directory.
        let binary_directory: String =
            binary_directory.unwrap_or_else(config::default_binary_directory);

        Ok(Self {
            config_path,
            http_sockaddr,
            binary_directory,
            console_file,
            ramfs_filename,
            log_directory,
            program_name,
            program_args,
            snapshot_path,
            mount_directory,
            kernel_args,
            #[cfg(feature = "gdb")]
            gdb_port,
            networking_mode,
            host_filter,
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
  {config} <file>                           Load options from a TOML configuration file. Options \
             given on the command line take precedence.
  {console_file} <file>                     Redirect console output to a file.
  {ramfs_filename} <file>                   Attach a RAM filesystem image to spawned user VMs.
  {bin_dir} <bin_dir>                       Directory containing Nanvix binaries (Default: \
             directory containing this executable).
  {log_dir} <log_dir>                       Directory for log files (Default: \
             {DEFAULT_LOG_DIRECTORY}).
  {snapshot} <path>                         Restore VM from snapshot instead of cold-booting .
  {mount} <host-dir>                        Mount a host directory on the guest at /mnt.
  {kernel_args} <args>                      Pass kernel arguments to guest control registers .
  {allow_host_networking}                   Enable host networking for the guest (disabled when \
             omitted).
    {allow_host} <ip|cidr>                    (Repeatable) Permit egress to this IPv4/CIDR \
             (requires {allow_host_networking}). When given, all other destinations are denied \
             except DNS on port 53.
    {block_host} <ip|cidr>                    (Repeatable) Deny egress to this IPv4/CIDR (requires \
             {allow_host_networking}). Always overrides {allow_host} and the DNS exception. \
             Without {allow_host}, all other destinations are permitted.
  {log_to_stdout}                          Route nanvixd's own logrus output to stdout instead of \
             a file in {log_dir} (file logger is otherwise the default).
    {gateway_sockaddr} <path>                 Path at which to expose the gateway endpoint -- the \
             host-side rendezvous point where a consumer (e.g. the containerd shim) reads the \
             guest's stdout/stderr and writes to its stdin. UDS path on Unix, named pipe path on \
             Windows. Defaults to a per-process auto path when omitted.{gdb_port_line}
",
            http_usage = http_usage,
            program_name = program_name,
            separator = Self::OPT_SEPARATOR,
            config = Self::OPT_CONFIG_FILE,
            console_file = Self::OPT_CONSOLE_FILE,
            ramfs_filename = Self::OPT_RAMFS_FILENAME,
            bin_dir = Self::OPT_BIN_DIRECTORY,
            log_dir = Self::OPT_LOG_DIRECTORY,
            snapshot = Self::OPT_SNAPSHOT,
            mount = Self::OPT_MOUNT_DIRECTORY,
            kernel_args = Self::OPT_KERNEL_ARGS,
            allow_host_networking = Self::OPT_ALLOW_HOST_NETWORKING,
            allow_host = Self::OPT_ALLOW_HOST,
            block_host = Self::OPT_BLOCK_HOST,
            log_to_stdout = Self::OPT_LOG_TO_STDOUT,
            gateway_sockaddr = Self::OPT_GATEWAY_SOCKADDR,
            gdb_port_line = if cfg!(feature = "gdb") {
                "\n  -gdb-port <port>                         GDB server port."
            } else {
                ""
            },
        );
    }

    ///
    /// # Description
    ///
    /// Returns the path of the configuration file from which options were loaded.
    ///
    /// # Returns
    ///
    /// The path given with [`Self::OPT_CONFIG_FILE`], if any.
    ///
    pub fn config_path(&self) -> Option<&str> {
        self.config_path.as_deref()
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
    /// This is the directory given with [`Self::OPT_BIN_DIRECTORY`], if any. Otherwise, it is the
    /// directory that contains the running executable (see
    /// [`config::default_binary_directory()`]).
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
    /// `-block-host` rules. Returns [`HostFilter::allow_all`] when neither is set.
    pub fn host_filter(&self) -> HostFilter {
        self.host_filter.clone()
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
mod tests;
