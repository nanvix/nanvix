// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox cache configuration management.
//!
//! This module provides structures for configuring the sandbox cache within the Nanvix Daemon.
//! It handles socket types, file paths, hardware topology settings, and deployment modes
//! (L2 VM support) that apply to all sandboxes managed by the daemon.

use ::hwloc::HwLoc;
use ::syscomm::SocketType;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Configuration for the sandbox cache.
///
/// This structure holds all global configuration parameters that apply to sandboxes managed
/// by the Nanvix Daemon, including socket types, file paths, hardware topology, and deployment
/// mode settings.
///
#[derive(Debug, Clone)]
pub struct SandboxCacheConfig {
    /// Socket type for control plane communication between nanvixd and linuxd.
    control_plane_socket_type: SocketType,
    /// Socket type for gateway communication between external clients and linuxd for stdin/stdout.
    gateway_socket_type: SocketType,
    /// Socket type for system VM communication between linuxd and user VMs.
    system_vm_socket_type: SocketType,
    /// Optional file path for redirecting console output.
    console_file: Option<String>,
    /// Optional hardware locality configuration for CPU affinity and topology information.
    hwloc: Option<HwLoc>,
    /// Path to the binary directory containing Nanvix binaries.
    binary_directory: String,
    /// Path to the toolchain binary directory containing cloud-hypervisor and other tools.
    toolchain_binary_directory: String,
    /// Directory path for writing log files.
    log_directory: String,
    /// Flag indicating whether to deploy linuxd inside an L2 VM (using cloud-hypervisor).
    l2: bool,
    /// Path to the temporary directory for Unix sockets and transient files.
    tmp_directory: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SandboxCacheConfig {
    ///
    /// # Description
    ///
    /// Creates a new sandbox cache configuration with the specified parameters.
    ///
    /// # Parameters
    ///
    /// - `control_plane_socket_type`: Socket type for control plane communication.
    /// - `gateway_socket_type`: Socket type for gateway communication.
    /// - `system_vm_socket_type`: Socket type for system VM communication.
    /// - `console_file`: Optional file path for redirecting console output.
    /// - `hwloc`: Optional hardware locality configuration.
    /// - `binary_directory`: Path to the binary directory.
    /// - `toolchain_binary_directory`: Path to the toolchain binary directory.
    /// - `log_directory`: Path to the log directory.
    /// - `l2`: Flag to deploy linuxd inside an L2 VM.
    /// - `tmp_directory`: Path to the temporary directory.
    ///
    /// # Returns
    ///
    /// A new sandbox cache configuration.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        control_plane_socket_type: SocketType,
        gateway_socket_type: SocketType,
        system_vm_socket_type: SocketType,
        console_file: Option<String>,
        hwloc: Option<HwLoc>,
        binary_directory: &str,
        toolchain_binary_directory: &str,
        log_directory: &str,
        l2: bool,
        tmp_directory: &str,
    ) -> Self {
        Self {
            control_plane_socket_type,
            gateway_socket_type,
            system_vm_socket_type,
            console_file,
            hwloc,
            binary_directory: binary_directory.to_string(),
            toolchain_binary_directory: toolchain_binary_directory.to_string(),
            log_directory: log_directory.to_string(),
            l2,
            tmp_directory: tmp_directory.to_string(),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the socket type configured for control plane communication.
    ///
    /// # Returns
    ///
    /// The socket type for the control plane.
    ///
    pub fn control_plane_sockaddr_type(&self) -> SocketType {
        self.control_plane_socket_type
    }

    ///
    /// # Description
    ///
    /// Returns the socket type configured for gateway communication.
    ///
    /// # Returns
    ///
    /// The socket type for the gateway.
    ///
    pub fn gateway_sockaddr_type(&self) -> SocketType {
        self.gateway_socket_type
    }

    ///
    /// # Description
    ///
    /// Returns the socket type configured for system VM communication.
    ///
    /// # Returns
    ///
    /// The socket type for system VM communication.
    ///
    pub fn system_vm_sockaddr_type(&self) -> SocketType {
        self.system_vm_socket_type
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
    /// Returns the flag indicating whether linuxd should be deployed inside an L2 VM.
    ///
    /// # Returns
    ///
    /// `true` if L2 deployment is enabled; `false` otherwise.
    ///
    pub fn l2(&self) -> bool {
        self.l2
    }

    ///
    /// # Description
    ///
    /// Returns the path to the temporary directory.
    ///
    /// # Returns
    ///
    /// The path to the temporary directory.
    ///
    pub fn tmp_directory(&self) -> &str {
        &self.tmp_directory
    }
}
