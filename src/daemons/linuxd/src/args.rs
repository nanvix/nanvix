// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::syscomm::SocketType;
use ::syslog::DEFAULT_LOG_DIRECTORY;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure packs the command-line arguments that were passed to the program.
///
pub struct Args {
    /// Unique identifier of the tenant associated with this linuxd instance.
    tenant_id: Option<String>,
    /// Socket address linuxd listens to for messages from the control plane.
    control_plane_sockaddr: String,
    /// Control plane socket address type.
    control_plane_sockaddr_type: Option<String>,
    /// Socket address linuxd listens to for messages from User VM.
    user_vm_bind_sockaddr: String,
    /// Server socket address type.
    user_vm_bind_sockaddr_type: Option<String>,
    /// Log to file?
    log_to_file: bool,
    /// Log file directory.
    log_directory: String,
    /// Deployed in an L2 VM?
    l2: bool,
    /// Whether networking system calls are enabled.
    networking_enabled: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    pub const OPT_HELP: &'static str = "-help";
    /// Command-line option for setting the tenant identifier.
    pub const OPT_TENANT_ID: &'static str = "-tenant-id";
    /// Command-line option for setting the control-plane socket address.
    pub const OPT_CONTROL_PLANE_SOCKADDR: &'static str = "-control-plane-addr";
    /// Command-line option for setting the socket address type of the bind socket.
    pub const OPT_CONTROL_PLANE_SOCKET_TYPE: &'static str = "-control-plane-socket-type";
    /// Command-line option for setting bind socket address.
    pub const OPT_USER_VM_BIND_SOCKADDR: &'static str = "-user-vm-bind-addr";
    /// Command-line option for setting the socket address type of the bind socket.
    pub const OPT_USER_VM_BIND_SOCKET_TYPE: &'static str = "-user-vm-bind-socket-type";
    /// Command-line option for log redirecting.
    pub const OPT_LOGFILE: &'static str = "-log-to-file";
    /// Command-line option for setting the log file directory.
    pub const OPT_LOGDIR: &'static str = "-log-dir";
    /// Command-line option for signaling deployment in an L2 VM.
    pub const OPT_L2: &'static str = "-l2";
    /// Command-line option for enabling networking system calls.
    pub const OPT_NETWORKING_ENABLED: &'static str = "-networking-enabled";

    // Command-line options for restoring linuxd from a snapshot using cloud-hypervisor. They are
    // only used when using linuxd as a library, so we allow dead code when building the binary.
    /// Command-line option to indicate the API socket path.
    #[allow(dead_code)]
    pub const OPT_CLH_API_SOCKET: &'static str = "--api-socket";
    /// Command-line option to indicate the restore operation.
    #[allow(dead_code)]
    pub const OPT_CLH_RESTORE: &'static str = "--restore";
    /// Command-line option to indicate the seccomp option.
    #[allow(dead_code)]
    pub const OPT_CLH_SECCOMP: &'static str = "--seccomp";
    /// Command-line option to indicate the resume operation to ch-remote.
    #[allow(dead_code)]
    pub const OPT_CH_REMOTE_RESUME: &'static str = "resume";

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
        let mut tenant_id: Option<String> = None;
        let mut control_plane_sockaddr: String = String::new();
        let mut control_plane_sockaddr_type: Option<String> = None;
        let mut user_vm_bind_sockaddr: String = String::new();
        let mut user_vm_bind_sockaddr_type: Option<String> = None;
        let mut log_to_file: bool = false;
        let mut log_directory: Option<String> = None;
        let mut l2: bool = false;
        let mut networking_enabled: bool = false;

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    return Err(anyhow::anyhow!("help message"));
                },
                Self::OPT_TENANT_ID => {
                    i += 1;
                    if i >= args.len() {
                        return Err(anyhow::anyhow!(
                            "missing value for {} option",
                            Self::OPT_TENANT_ID
                        ));
                    }
                    tenant_id = Some(args[i].clone());
                },
                Self::OPT_CONTROL_PLANE_SOCKADDR => {
                    i += 1;
                    control_plane_sockaddr = args[i].clone();
                },
                Self::OPT_CONTROL_PLANE_SOCKET_TYPE => {
                    i += 1;
                    control_plane_sockaddr_type = Some(args[i].clone());
                },
                Self::OPT_USER_VM_BIND_SOCKADDR => {
                    i += 1;
                    user_vm_bind_sockaddr = args[i].clone();
                },
                Self::OPT_USER_VM_BIND_SOCKET_TYPE => {
                    i += 1;
                    user_vm_bind_sockaddr_type = Some(args[i].clone());
                },
                Self::OPT_LOGFILE => {
                    log_to_file = true;
                },
                Self::OPT_LOGDIR => {
                    i += 1;
                    log_directory = Some(args[i].clone());
                },
                Self::OPT_L2 => {
                    l2 = true;
                },
                Self::OPT_NETWORKING_ENABLED => {
                    networking_enabled = true;
                },
                invalid_arg => {
                    return Err(anyhow::anyhow!("invalid argument: {invalid_arg}"));
                },
            }

            i += 1;
        }

        // Mandatory arguments validation.
        if control_plane_sockaddr.is_empty() {
            return Err(anyhow::anyhow!(
                "control-plane socket address not set (use {})",
                Self::OPT_CONTROL_PLANE_SOCKADDR
            ));
        }

        if user_vm_bind_sockaddr.is_empty() {
            return Err(anyhow::anyhow!(
                "user VM bind socket address not set (use {})",
                Self::OPT_USER_VM_BIND_SOCKADDR
            ));
        }

        // Check if log file directory was set if logging to file is enabled. Set the default directory if not.
        let log_directory: String = match (log_to_file, log_directory) {
            (true, Some(path)) => path,
            // default to log dir relative to the CWD, make the directory path absolute
            (true, None) => {
                let mut abs_path = std::env::current_dir().map_err(|e| {
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
        // If it does not exist, try to create it.
        if log_to_file {
            let path = std::path::Path::new(&log_directory);
            if !path.exists() {
                std::fs::create_dir_all(path).map_err(|e| {
                    anyhow::anyhow!(
                        "failed to create log file directory (path={}, error={:?})",
                        path.display(),
                        e
                    )
                })?;
            }
            // check if we can create and write a file in the directory
            let test_file_path = path.join("test.log");
            match std::fs::File::create(&test_file_path) {
                Ok(_) => {
                    std::fs::remove_file(&test_file_path).ok(); // clean up the test file
                },
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "failed to create log file (path={}, error={:?})",
                        test_file_path.display(),
                        e
                    ));
                },
            }
        }

        Ok(Self {
            tenant_id,
            control_plane_sockaddr,
            control_plane_sockaddr_type,
            user_vm_bind_sockaddr,
            user_vm_bind_sockaddr_type,
            log_to_file,
            log_directory,
            l2,
            networking_enabled,
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
            "Usage: {} [{} <tenant-id>] {} <control-plane-sockaddr> {} <control-plane-socktype> \
             {} <user-vm-sockaddr> {} <user-vm-socktype> [{} [{} <log-file-dir>]] {}",
            program_name,
            Self::OPT_TENANT_ID,
            Self::OPT_CONTROL_PLANE_SOCKADDR,
            Self::OPT_CONTROL_PLANE_SOCKET_TYPE,
            Self::OPT_USER_VM_BIND_SOCKADDR,
            Self::OPT_USER_VM_BIND_SOCKET_TYPE,
            Self::OPT_LOGFILE,
            Self::OPT_LOGDIR,
            Self::OPT_L2
        );
    }

    ///
    /// # Description
    ///
    /// Returns the tenant identifier associated to this linuxd instance.
    ///
    /// # Returns
    ///
    /// The optional tenant identifier.
    ///
    pub fn tenant_id(&self) -> Option<&str> {
        self.tenant_id.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the socket address to connect to the control plane.
    ///
    /// # Returns
    ///
    /// The socket address of the bind socket.
    ///
    pub fn control_plane_sockaddr(&self) -> &str {
        &self.control_plane_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the socket address to connect to the control plane.
    ///
    /// # Returns
    ///
    /// The socket address of the bind socket.
    ///
    pub fn control_plane_socket_type(&self) -> &str {
        self.control_plane_sockaddr_type
            .as_deref()
            .unwrap_or(SocketType::UNIX_STR)
    }

    ///
    /// # Description
    ///
    /// Returns the bind socket address for the User VM.
    ///
    /// # Returns
    ///
    /// The socket address of the bind socket.
    ///
    pub fn user_vm_bind_sockaddr(&self) -> &str {
        &self.user_vm_bind_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the socket address type of the bind socket for the User VM.
    ///
    /// # Returns
    ///
    /// The socket address type of the bind socket.
    ///
    pub fn user_vm_bind_socket_type(&self) -> &str {
        self.user_vm_bind_sockaddr_type
            .as_deref()
            .unwrap_or(SocketType::UNIX_STR)
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

    ///
    /// # Description
    ///
    /// Returns the log file directory.
    /// # Returns
    ///
    /// The log file directory.
    ///
    pub fn log_file_dir(&self) -> String {
        self.log_directory.clone()
    }

    ///
    /// # Description
    ///
    /// Returns whether we are deployed inside an L2 VM.
    ///
    /// # Returns
    ///
    /// If deployed inside an L2 VM.
    ///
    pub fn l2(&self) -> bool {
        self.l2
    }

    ///
    /// # Description
    ///
    /// Returns whether networking system calls are enabled.
    ///
    /// # Returns
    ///
    /// `true` if networking is enabled; `false` otherwise.
    ///
    pub fn networking_enabled(&self) -> bool {
        self.networking_enabled
    }
}
