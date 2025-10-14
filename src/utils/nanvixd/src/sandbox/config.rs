// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::sandbox::tcp_port::TcpPort;
use hwloc::HwLoc;

//==================================================================================================
// Structures
//==================================================================================================

/// Packs configuration for a sandbox.
#[derive(Debug)]
pub struct SandboxConfig {
    /// Socket address for the control plane nanvixd <-> linuxd.
    control_plane_sockaddr: String,
    /// Socket type for the control plane nanvixd <-> linuxd communication.
    control_plane_socket_type: String,
    /// Socket address to interact with the user VM stdin/stdout client <-> linuxd.
    gateway_sockaddr: String,
    /// Socket type to interact with the user VM stdin/stdout client <-> linuxd.
    gateway_socket_type: String,
    /// Socket address for the linuxd <-> user VM communication.
    user_vm_sockaddr: String,
    /// Socket type for the linuxd <-> user VM communication.
    system_vm_socket_type: String,
    /// Path to the program to run.
    program: String,
    /// Argv for the program to run.
    program_args: Option<String>,
    /// File for console output.
    console_file: Option<String>,
    /// Hardware locality configuration.
    hwloc: Option<HwLoc>,
    /// Path to the binary directory.
    binary_directory: String,
    /// Path to the toolchain binary directory.
    toolchain_binary_directory: String,
    /// Directory for log files.
    log_directory: String,
    /// Flag to deploy linuxd in an L2 VM.
    l2: bool,
    /// TCP port for the gateway in the L2 VM if L2 deployment enabled.
    // The value is never read, but we keep it around to trigger a port
    // release upon drop.
    _gateway_l2_port: Option<TcpPort>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SandboxConfig {
    ///
    /// # Description
    ///
    /// Creates a new sandbox configuration.
    ///
    /// # Parameters
    ///
    /// - `control_plane_sockaddr`: Socket address for the control plane.
    /// - `control_plane_socket_type`: Socket type for the control plane.
    /// - `gateway_sockaddr`: Socket address for the gateway.
    /// - `gateway_socket_type`: Socket type for the gateway.
    /// - `user_vm_sockaddr`: Socket address for the user VM.
    /// - `system_vm_socket_type`: Socket type for the user VM to linuxd channel.
    /// - `program`: Path to the program to run.
    /// - `program_args`: Arguments for the program.
    /// - `console_file`: File for console output.
    /// - `hwloc`: Hardware locality configuration.
    /// - `binary_directory`: Path to the binary directory.
    /// - `toolchain_binary_directory`: Path to the toolchain binary directory.
    /// - `log_directory`: Path to the log directory.
    /// - `l2`: Flag to deploy linuxd in an L2 VM.
    /// - `gateway_l2_port`: Port for the gateway in the L2 VM.
    ///
    /// # Returns
    ///
    /// A new sandbox configuration.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        control_plane_sockaddr: &str,
        control_plane_socket_type: &str,
        gateway_sockaddr: &str,
        gateway_socket_type: &str,
        user_vm_sockaddr: &str,
        system_vm_socket_type: &str,
        program: &str,
        program_args: Option<String>,
        console_file: Option<String>,
        hwloc: Option<HwLoc>,
        binary_directory: &str,
        toolchain_binary_directory: &str,
        log_directory: &str,
        l2: bool,
        gateway_l2_port: Option<TcpPort>,
    ) -> Self {
        Self {
            control_plane_sockaddr: control_plane_sockaddr.to_string(),
            control_plane_socket_type: control_plane_socket_type.to_string(),
            gateway_sockaddr: gateway_sockaddr.to_string(),
            gateway_socket_type: gateway_socket_type.to_string(),
            user_vm_sockaddr: user_vm_sockaddr.to_string(),
            system_vm_socket_type: system_vm_socket_type.to_string(),
            program: program.to_string(),
            program_args,
            console_file,
            hwloc,
            binary_directory: binary_directory.to_string(),
            toolchain_binary_directory: toolchain_binary_directory.to_string(),
            log_directory: log_directory.to_string(),
            l2,
            _gateway_l2_port: gateway_l2_port,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the socket address for the control plane.
    ///
    /// # Returns
    ///
    /// The socket address for the control plane.
    ///
    pub fn control_plane_sockaddr(&self) -> &str {
        &self.control_plane_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the socket type for the control plane.
    ///
    /// # Returns
    ///
    /// The socket type for the control plane.
    ///
    pub fn control_plane_sockaddr_type(&self) -> &str {
        &self.control_plane_socket_type
    }

    ///
    /// # Description
    ///
    /// Returns the socket address for linuxd's gateway socket.
    ///
    /// # Returns
    ///
    /// The socket address for linuxd's gateway.
    ///
    pub fn gateway_sockaddr(&self) -> &str {
        &self.gateway_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the socket type for linuxd's gateway socket.
    ///
    /// # Returns
    ///
    /// The socket type for linuxd's gateway.
    ///
    pub fn gateway_sockaddr_type(&self) -> &str {
        &self.gateway_socket_type
    }

    ///
    /// # Description
    ///
    /// Returns the file path of the sandbox's program.
    ///
    /// # Returns
    ///
    /// The file path of the main program.
    ///
    pub fn program(&self) -> &str {
        &self.program
    }

    ///
    /// # Description
    ///
    /// Returns the argv of the sandbox's program.
    ///
    /// # Returns
    ///
    /// The argv of the main program.
    ///
    pub fn program_args(&self) -> Option<&str> {
        self.program_args.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the socket address for the user VM to linuxd communication.
    ///
    /// # Returns
    ///
    /// The socket address for the user VM.
    ///
    pub fn user_vm_sockaddr(&self) -> &str {
        &self.user_vm_sockaddr
    }

    ///
    /// # Description
    ///
    /// Returns the socket type for the user VM communication.
    ///
    /// # Returns
    ///
    /// The socket type for the user VM communication.
    ///
    pub fn system_vm_sockaddr_type(&self) -> &str {
        &self.system_vm_socket_type
    }

    ///
    /// # Description
    ///
    /// Returns the file for console output.
    ///
    /// # Returns
    ///
    /// The file for console output.
    ///
    pub fn console_file(&self) -> Option<&str> {
        self.console_file.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the hardware locality configuration.
    ///
    /// # Returns
    ///
    /// The hardware locality configuration.
    ///
    pub fn hwloc(&self) -> Option<HwLoc> {
        self.hwloc.clone()
    }

    ///
    /// # Description
    ///
    /// Returns the path to the binary directory.
    ///
    /// # Returns
    ///
    /// The path to the binary directory.
    ///
    pub fn binary_directory(&self) -> &str {
        &self.binary_directory
    }

    ///
    /// # Description
    ///
    /// Returns the path to the toolchain binary directory.
    ///
    /// # Returns
    ///
    /// The path to the toolchain binary directory.
    ///
    pub fn toolchain_binary_directory(&self) -> &str {
        &self.toolchain_binary_directory
    }

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
    /// Returns the l2 flag.
    ///
    /// # Returns
    ///
    /// The flag to enable deployment in an L2 VM.
    ///
    pub fn l2(&self) -> bool {
        self.l2
    }
}
