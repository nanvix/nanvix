// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Arguments for spawning User VM instances.
//!
//! This module defines the `UserVmArgs` structure which encapsulates all configuration
//! parameters required to spawn a User VM instance. This includes socket information for
//! communication channels, program details, and execution environment settings.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "standalone"))]
use ::syscomm::SocketType;
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Arguments for spawning a User VM instance.
///
#[derive(Debug)]
pub struct UserVmArgs {
    /// Information on control plane connect socket (address, socket type) for nanvixd <-> linuxd communication.
    #[cfg(not(feature = "standalone"))]
    control_plane_connect_socket_info: (String, SocketType),
    /// Information on gateway socket (address, socket type) for client <-> linuxd stdin/stdout communication.
    #[cfg(not(feature = "standalone"))]
    gateway_socket_info: (String, SocketType),
    /// Information on System VM socket (address, socket type) for linuxd <-> uservm communication.
    #[cfg(not(feature = "standalone"))]
    system_vm_socket_info: (String, SocketType),
    /// Path to the guest program binary to execute inside the User VM.
    program: String,
    /// Optional command-line arguments to pass to the program.
    program_args: Option<String>,
    /// Optional RAM filesystem image that should be exposed to the guest.
    ramfs_filename: Option<String>,
    /// Optional file path for redirecting console output.
    console_file: Option<String>,
    /// Optional hardware locality configuration for CPU affinity and topology information.
    hwloc: Option<hwloc::HwLoc>,
    /// Path to kernel binary.
    kernel_binary_path: String,
    /// Path to the User VM binary.
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    uservm_binary_path: String,
    /// Directory path for writing log files.
    log_directory: String,
    /// Unique identifier for this User VM instance.
    uservm_id: UserVmIdentifier,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl UserVmArgs {
    ///
    /// # Description
    ///
    /// Creates a new User VM arguments configuration with the specified parameters.
    ///
    /// # Parameters
    ///
    /// - `control_plane_connect_socket_info`: Socket information for control plane communication.
    /// - `gateway_socket_info`: Socket information for gateway communication.
    /// - `system_vm_socket_info`: Socket information for system VM communication.
    /// - `program`: Path to the guest program binary.
    /// - `program_args`: Optional command-line arguments for the program.
    /// - `ramfs_filename`: Optional RAM filesystem image filename passed to the guest.
    /// - `console_file`: Optional file path for redirecting console output.
    /// - `hwloc`: Optional hardware locality configuration.
    /// - `kernel_binary_path`: Path to kernel binary.
    /// - `uservm_binary_path`: Path to the User VM binary (only if not in single-process mode).
    /// - `log_directory`: Path to the log directory.
    /// - `uservm_id`: Unique identifier for this User VM instance.
    ///
    /// # Returns
    ///
    /// A new User VM arguments configuration.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        #[cfg(not(feature = "standalone"))] control_plane_connect_socket_info: (String, SocketType),
        #[cfg(not(feature = "standalone"))] gateway_socket_info: (String, SocketType),
        #[cfg(not(feature = "standalone"))] system_vm_socket_info: (String, SocketType),
        program: String,
        program_args: Option<String>,
        ramfs_filename: Option<String>,
        console_file: Option<String>,
        hwloc: Option<hwloc::HwLoc>,
        kernel_binary_path: String,
        #[cfg(not(any(feature = "single-process", feature = "standalone")))]
        uservm_binary_path: String,
        log_directory: String,
        uservm_id: UserVmIdentifier,
    ) -> Self {
        Self {
            #[cfg(not(feature = "standalone"))]
            control_plane_connect_socket_info,
            #[cfg(not(feature = "standalone"))]
            gateway_socket_info,
            #[cfg(not(feature = "standalone"))]
            system_vm_socket_info,
            program,
            program_args,
            ramfs_filename,
            console_file,
            hwloc,
            kernel_binary_path,
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            uservm_binary_path,
            log_directory,
            uservm_id,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the control plane connect socket information.
    ///
    /// # Returns
    ///
    /// A reference to the control plane connect socket information tuple.
    ///
    #[cfg(not(feature = "standalone"))]
    pub fn control_plane_connect_socket_info(&self) -> &(String, SocketType) {
        &self.control_plane_connect_socket_info
    }

    ///
    /// # Description
    ///
    /// Returns the gateway socket information.
    ///
    /// # Returns
    ///
    /// A reference to the gateway socket information tuple.
    ///
    #[cfg(not(feature = "standalone"))]
    pub fn gateway_socket_info(&self) -> &(String, SocketType) {
        &self.gateway_socket_info
    }

    ///
    /// # Description
    ///
    /// Returns the system VM socket information.
    ///
    /// # Returns
    ///
    /// A reference to the system VM socket information tuple.
    ///
    #[cfg(not(feature = "standalone"))]
    pub fn system_vm_socket_info(&self) -> &(String, SocketType) {
        &self.system_vm_socket_info
    }

    ///
    /// # Description
    ///
    /// Returns the path to the guest program binary.
    ///
    /// # Returns
    ///
    /// The path to the program binary.
    ///
    pub fn program(&self) -> &str {
        &self.program
    }

    ///
    /// # Description
    ///
    /// Returns the optional command-line arguments for the program.
    ///
    /// # Returns
    ///
    /// An optional reference to the program arguments.
    ///
    pub fn program_args(&self) -> Option<&str> {
        self.program_args.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional RAM filesystem image filename passed to the guest.
    ///
    /// # Returns
    ///
    /// An optional reference to the RAM filesystem filename.
    ///
    pub fn ramfs_filename(&self) -> Option<&str> {
        self.ramfs_filename.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional file path for console output redirection.
    ///
    /// # Returns
    ///
    /// An optional reference to the console file path.
    ///
    pub fn console_file(&self) -> Option<&str> {
        self.console_file.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the hardware locality configuration if available.
    ///
    /// # Returns
    ///
    /// An optional clone of the hardware locality configuration.
    ///
    pub fn hwloc(&self) -> Option<hwloc::HwLoc> {
        self.hwloc.clone()
    }

    ///
    /// # Description
    ///
    /// Returns the path to the kernel binary.
    ///
    /// # Returns
    ///
    /// The path to the kernel binary.
    ///
    pub fn kernel_binary_path(&self) -> &str {
        &self.kernel_binary_path
    }

    ///
    /// # Description
    ///
    /// Returns the path to the User VM binary.
    ///
    /// # Returns
    ///
    /// The path to the User VM binary.
    ///
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    pub fn uservm_binary_path(&self) -> &str {
        &self.uservm_binary_path
    }

    ///
    /// # Description
    ///
    /// Returns the log directory.
    ///
    /// # Returns
    ///
    /// The path to the log directory.
    ///
    pub fn log_directory(&self) -> &str {
        &self.log_directory
    }

    ///
    /// # Description
    ///
    /// Returns the unique identifier for this User VM instance.
    ///
    /// # Returns
    ///
    /// The User VM identifier.
    ///
    pub fn uservm_id(&self) -> UserVmIdentifier {
        self.uservm_id
    }
}
