// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::net_backend::{
    HostFilter,
    Ipv4Cidr,
};
use ::syscomm::SocketType;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Command-line arguments accepted by the decoupled `networkd` binary.
///
/// `networkd` serves exactly one user VM (standalone-like), so it only needs to know where to
/// listen for that user VM, how to filter the guest's host egress, and how to configure logging.
///
pub struct Args {
    /// Socket address that `networkd` binds to and accepts the user VM connection on.
    user_vm_bind_sockaddr: String,
    /// Socket address type of the user VM bind socket (defaults to Unix).
    user_vm_bind_sockaddr_type: Option<String>,
    /// Allowlist of IPv4/CIDR egress destinations (`-allow-host`). When non-empty, only these
    /// destinations are reachable. Mutually exclusive with `block_hosts`.
    allow_hosts: Vec<String>,
    /// Blocklist of IPv4/CIDR egress destinations (`-block-host`). When non-empty, all destinations
    /// except these are reachable. Mutually exclusive with `allow_hosts`.
    block_hosts: Vec<String>,
    /// Whether log output is written to a file.
    log_to_file: bool,
    /// Directory that log files are written to when `log_to_file` is set.
    log_directory: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    pub const OPT_HELP: &'static str = "-help";
    /// Command-line option for setting the user VM bind socket address.
    pub const OPT_USER_VM_BIND_SOCKADDR: &'static str = "-user-vm-bind-addr";
    /// Command-line option for setting the user VM bind socket address type.
    pub const OPT_USER_VM_BIND_SOCKET_TYPE: &'static str = "-user-vm-bind-socket-type";
    /// Command-line option (repeatable) adding an IPv4/CIDR to the egress allowlist.
    pub const OPT_ALLOW_HOST: &'static str = "-allow-host";
    /// Command-line option (repeatable) adding an IPv4/CIDR to the egress blocklist.
    pub const OPT_BLOCK_HOST: &'static str = "-block-host";
    /// Command-line option for redirecting log output to a file.
    pub const OPT_LOGFILE: &'static str = "-log-to-file";
    /// Command-line option for setting the log file directory.
    pub const OPT_LOGDIR: &'static str = "-log-dir";

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
    /// On success, the parsed arguments. On failure, a human-readable error.
    ///
    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut user_vm_bind_sockaddr: String = String::new();
        let mut user_vm_bind_sockaddr_type: Option<String> = None;
        let mut allow_hosts: Vec<String> = Vec::new();
        let mut block_hosts: Vec<String> = Vec::new();
        let mut log_to_file: bool = false;
        let mut log_directory: Option<String> = None;

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    return Err(::anyhow::anyhow!("help message"));
                },
                Self::OPT_USER_VM_BIND_SOCKADDR => {
                    i += 1;
                    user_vm_bind_sockaddr = Self::value(&args, i, Self::OPT_USER_VM_BIND_SOCKADDR)?;
                },
                Self::OPT_USER_VM_BIND_SOCKET_TYPE => {
                    i += 1;
                    user_vm_bind_sockaddr_type =
                        Some(Self::value(&args, i, Self::OPT_USER_VM_BIND_SOCKET_TYPE)?);
                },
                Self::OPT_ALLOW_HOST => {
                    i += 1;
                    let entry: String = Self::value(&args, i, Self::OPT_ALLOW_HOST)?;
                    if Ipv4Cidr::parse(&entry).is_none() {
                        return Err(::anyhow::anyhow!(
                            "invalid {} value (expected IPv4 or CIDR): {entry}",
                            Self::OPT_ALLOW_HOST
                        ));
                    }
                    allow_hosts.push(entry);
                },
                Self::OPT_BLOCK_HOST => {
                    i += 1;
                    let entry: String = Self::value(&args, i, Self::OPT_BLOCK_HOST)?;
                    if Ipv4Cidr::parse(&entry).is_none() {
                        return Err(::anyhow::anyhow!(
                            "invalid {} value (expected IPv4 or CIDR): {entry}",
                            Self::OPT_BLOCK_HOST
                        ));
                    }
                    block_hosts.push(entry);
                },
                Self::OPT_LOGFILE => {
                    log_to_file = true;
                },
                Self::OPT_LOGDIR => {
                    i += 1;
                    log_directory = Some(Self::value(&args, i, Self::OPT_LOGDIR)?);
                },
                invalid_arg => {
                    return Err(::anyhow::anyhow!("invalid argument: {invalid_arg}"));
                },
            }

            i += 1;
        }

        if user_vm_bind_sockaddr.is_empty() {
            return Err(::anyhow::anyhow!(
                "user VM bind socket address not set (use {})",
                Self::OPT_USER_VM_BIND_SOCKADDR
            ));
        }

        // An allowlist and a blocklist are mutually exclusive: `HostFilter::from_lists` gives the
        // allowlist precedence, so accepting both would silently ignore the blocklist.
        if !allow_hosts.is_empty() && !block_hosts.is_empty() {
            return Err(::anyhow::anyhow!(
                "{} and {} are mutually exclusive",
                Self::OPT_ALLOW_HOST,
                Self::OPT_BLOCK_HOST
            ));
        }

        // The log directory is only meaningful when logging to file; otherwise it is unused.
        let log_directory: String = match (log_to_file, log_directory) {
            (true, Some(path)) => path,
            (true, None) => String::from(::syslog::DEFAULT_LOG_DIRECTORY),
            (false, _) => String::new(),
        };

        Ok(Self {
            user_vm_bind_sockaddr,
            user_vm_bind_sockaddr_type,
            allow_hosts,
            block_hosts,
            log_to_file,
            log_directory,
        })
    }

    ///
    /// # Description
    ///
    /// Reads the value that follows an option, erroring out if it is missing.
    ///
    fn value(args: &[String], index: usize, option: &str) -> Result<String> {
        args.get(index)
            .cloned()
            .ok_or_else(|| ::anyhow::anyhow!("missing value for {option} option"))
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
            "Usage: {} {} <user-vm-sockaddr> [{} <user-vm-socktype>] [{} <ipv4/cidr>]... [{} \
             <ipv4/cidr>]... [{} [{} <log-file-dir>]]",
            program_name,
            Self::OPT_USER_VM_BIND_SOCKADDR,
            Self::OPT_USER_VM_BIND_SOCKET_TYPE,
            Self::OPT_ALLOW_HOST,
            Self::OPT_BLOCK_HOST,
            Self::OPT_LOGFILE,
            Self::OPT_LOGDIR,
        );
    }

    ///
    /// # Description
    ///
    /// Returns the socket address that `networkd` binds to for the user VM.
    ///
    pub fn user_vm_bind_sockaddr(&self) -> &str {
        &self.user_vm_bind_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the socket address type of the user VM bind socket, defaulting to Unix.
    ///
    pub fn user_vm_bind_socket_type(&self) -> &str {
        self.user_vm_bind_sockaddr_type
            .as_deref()
            .unwrap_or(SocketType::UNIX_STR)
    }

    ///
    /// # Description
    ///
    /// Builds the host egress [`HostFilter`] from the allow / block lists. Entries were already
    /// validated at parse time, so no entry is silently dropped here. The DNS carve-out is enabled
    /// for allowlists so that name resolution keeps working, mirroring the standalone path.
    ///
    pub fn host_filter(&self) -> HostFilter {
        HostFilter::from_lists(&self.allow_hosts, &self.block_hosts, true)
    }

    ///
    /// # Description
    ///
    /// Returns whether log output is written to a file.
    ///
    pub fn log_to_file(&self) -> bool {
        self.log_to_file
    }

    ///
    /// # Description
    ///
    /// Returns the log file directory.
    ///
    pub fn log_directory(&self) -> String {
        self.log_directory.clone()
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an argument vector with a leading (ignored) program name.
    fn argv(rest: &[&str]) -> Vec<String> {
        ::std::iter::once("networkd")
            .chain(rest.iter().copied())
            .map(String::from)
            .collect()
    }

    /// The bind address is mandatory.
    #[test]
    fn parse_requires_bind_address() {
        assert!(Args::parse(argv(&[])).is_err());
    }

    /// A minimal invocation parses and defaults to an unrestricted (allow-all) egress policy.
    #[test]
    fn parse_minimal_defaults_to_allow_all() {
        let args: Args = Args::parse(argv(&["-user-vm-bind-addr", "/tmp/networkd.sock"]))
            .expect("minimal args should parse");
        assert_eq!(args.user_vm_bind_sockaddr(), "/tmp/networkd.sock");
        assert_eq!(args.user_vm_bind_socket_type(), SocketType::UNIX_STR);
        assert!(matches!(args.host_filter(), HostFilter::AllowAll));
    }

    /// Repeated `-allow-host` entries produce an allowlist filter.
    #[test]
    fn parse_allow_hosts_builds_allowlist() {
        let args: Args = Args::parse(argv(&[
            "-user-vm-bind-addr",
            "/tmp/networkd.sock",
            "-allow-host",
            "10.0.0.0/8",
            "-allow-host",
            "192.168.1.1",
        ]))
        .expect("allow-host args should parse");
        assert!(matches!(args.host_filter(), HostFilter::Allow { .. }));
    }

    /// Repeated `-block-host` entries produce a blocklist filter.
    #[test]
    fn parse_block_hosts_builds_blocklist() {
        let args: Args = Args::parse(argv(&[
            "-user-vm-bind-addr",
            "/tmp/networkd.sock",
            "-block-host",
            "10.0.0.0/8",
        ]))
        .expect("block-host args should parse");
        assert!(matches!(args.host_filter(), HostFilter::Block(_)));
    }

    /// Allow and block lists are mutually exclusive.
    #[test]
    fn parse_rejects_allow_and_block_together() {
        assert!(Args::parse(argv(&[
            "-user-vm-bind-addr",
            "/tmp/networkd.sock",
            "-allow-host",
            "10.0.0.0/8",
            "-block-host",
            "192.168.0.0/16",
        ]))
        .is_err());
    }

    /// A malformed egress entry is rejected at parse time.
    #[test]
    fn parse_rejects_invalid_host_entry() {
        assert!(Args::parse(argv(&[
            "-user-vm-bind-addr",
            "/tmp/networkd.sock",
            "-allow-host",
            "not-an-ip",
        ]))
        .is_err());
        assert!(Args::parse(argv(&[
            "-user-vm-bind-addr",
            "/tmp/networkd.sock",
            "-block-host",
            "10.0.0.0/33",
        ]))
        .is_err());
    }

    /// A dangling option with no value is rejected.
    #[test]
    fn parse_rejects_missing_host_value() {
        assert!(Args::parse(argv(&["-user-vm-bind-addr", "/tmp/networkd.sock", "-allow-host",]))
            .is_err());
    }
}
