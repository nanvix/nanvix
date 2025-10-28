// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox configuration structures and utilities.
//!
//! This module defines the configuration structure for sandboxed execution environments.
//! It includes socket information, file paths, hardware topology settings, and optional
//! parameters for control plane and Linux Daemon initialization.

//==================================================================================================
// Imports
//==================================================================================================

use crate::tcp_port::TcpPort;
use ::syscomm::SocketType;
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Configuration for a sandbox.
///
/// This structure contains all parameters needed to configure and initialize a sandboxed
/// execution environment, including socket information, file paths, hardware topology,
/// and optional control plane configuration for when components are initialized separately.
///
pub struct SandboxConfig {
    /// Unique identifier for the User VM.
    uservm_id: UserVmIdentifier,
    /// Information on gateway socket (address, socket type, optional L2 TCP port).
    gateway_socket_info: (String, SocketType, Option<TcpPort>),
    /// Information on System VM socket (address, socket type).
    system_vm_socket_info: (String, SocketType),
    /// Optional file path for redirecting console output.
    console_file: Option<String>,
    /// Optional hardware locality configuration for CPU affinity and topology information.
    hwloc: Option<hwloc::HwLoc>,
    /// Path to kernel binary.
    kernel_binary_path: String,
    /// Path to the Linux Daemon binary.
    #[cfg(not(feature = "single-process"))]
    linuxd_binary_path: String,
    /// Path to the User VM binary.
    #[cfg(not(feature = "single-process"))]
    uservm_binary_path: String,
    /// Directory path for writing log files.
    log_directory: String,
    /// Optional system call table for overriding default system call behavior.
    #[cfg(feature = "single-process")]
    syscall_table: Option<::std::sync::Arc<::linuxd::syscalls::SyscallTable>>,

    /// Optional information on control plane socket (address, socket type).
    /// This must be provided if the control plane socket is not already initialized before
    /// sandbox initialization. If both socket and info are provided, the existing socket is used.
    control_plane_socket_info: Option<(String, SocketType)>,

    /// Optional path to the toolchain binary directory containing cloud-hypervisor and other tools.
    /// This must be provided if a Linux Daemon instance was not provided before sandbox initialization.
    toolchain_binary_directory: Option<String>,

    /// Optional path to the temporary directory for Unix sockets and transient files.
    /// This must be provided if a Linux Daemon instance was not provided before sandbox initialization.
    tmp_directory: Option<String>,

    /// Optional flag to deploy the Linux Daemon inside an L2 VM (using cloud-hypervisor).
    /// This must be provided if a Linux Daemon instance was not provided before sandbox initialization.
    l2: Option<bool>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SandboxConfig {
    ///
    /// # Description
    ///
    /// Creates a new sandbox configuration with the specified parameters.
    ///
    /// # Parameters
    ///
    /// - `uservm_id`: Unique identifier for the User VM.
    /// - `gateway_socket_info`: Information on gateway socket (address, socket type, optional L2 TCP port).
    /// - `system_vm_socket_info`: Information on System VM socket (address, socket type).
    /// - `console_file`: Optional file path for redirecting console output.
    /// - `hwloc`: Optional hardware locality configuration.
    /// - `kernel_binary_path`: Path to kernel binary.
    /// - `linuxd_binary_path`: Path to the Linux Daemon binary (only if not in single-process mode).
    /// - `uservm_binary_path`: Path to the User VM binary (only if not in single-process mode).
    /// - `log_directory`: Path to the log directory.
    /// - `syscall_table`: Optional system call table for overriding default system call behavior (only if in single-process mode).
    /// - `control_plane_socket_info`: Optional information on control plane socket (address, socket type).
    /// - `toolchain_binary_directory`: Optional path to the toolchain binary directory.
    /// - `tmp_directory`: Optional path to the temporary directory.
    /// - `l2`: Optional flag to deploy the Linux Daemon inside an L2 VM.
    ///
    /// # Returns
    ///
    /// A new sandbox configuration.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        uservm_id: UserVmIdentifier,
        gateway_socket_info: (String, SocketType, Option<TcpPort>),
        system_vm_socket_info: (String, SocketType),
        console_file: Option<String>,
        hwloc: Option<hwloc::HwLoc>,
        kernel_binary_path: &str,
        #[cfg(not(feature = "single-process"))] linuxd_binary_path: &str,
        #[cfg(not(feature = "single-process"))] uservm_binary_path: &str,
        log_directory: &str,
        #[cfg(feature = "single-process")] syscall_table: Option<
            ::std::sync::Arc<::linuxd::syscalls::SyscallTable>,
        >,
        control_plane_socket_info: Option<(String, SocketType)>,
        toolchain_binary_directory: Option<String>,
        tmp_directory: Option<String>,
        l2: Option<bool>,
    ) -> Self {
        Self {
            uservm_id,
            gateway_socket_info,
            system_vm_socket_info,
            console_file,
            hwloc,
            kernel_binary_path: kernel_binary_path.to_string(),
            #[cfg(not(feature = "single-process"))]
            linuxd_binary_path: linuxd_binary_path.to_string(),
            #[cfg(not(feature = "single-process"))]
            uservm_binary_path: uservm_binary_path.to_string(),
            log_directory: log_directory.to_string(),
            #[cfg(feature = "single-process")]
            syscall_table,
            control_plane_socket_info,
            toolchain_binary_directory,
            tmp_directory,
            l2,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the unique identifier for the User VM.
    ///
    /// # Returns
    ///
    /// The User VM identifier.
    ///
    pub fn uservm_id(&self) -> UserVmIdentifier {
        self.uservm_id
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
    pub fn gateway_socket_info(&self) -> &(String, SocketType, Option<TcpPort>) {
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
    pub fn system_vm_socket_info(&self) -> &(String, SocketType) {
        &self.system_vm_socket_info
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
    /// Returns the path to kernel binary.
    ///
    /// # Returns
    ///
    /// The path to kernel binary.
    ///
    pub fn kernel_binary_path(&self) -> &str {
        &self.kernel_binary_path
    }

    ///
    /// # Description
    ///
    /// Returns the path to the Linux Daemon binary.
    ///
    /// # Returns
    ///
    /// The path to the Linux Daemon binary.
    ///
    #[cfg(not(feature = "single-process"))]
    pub fn linuxd_binary_path(&self) -> &str {
        &self.linuxd_binary_path
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
    #[cfg(not(feature = "single-process"))]
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
    /// Returns the optional system call table.
    ///
    /// # Returns
    ///
    /// An optional clone of the system call table.
    ///
    #[cfg(feature = "single-process")]
    pub fn syscall_table(&self) -> Option<::std::sync::Arc<::linuxd::syscalls::SyscallTable>> {
        self.syscall_table.clone()
    }

    ///
    /// # Description
    ///
    /// Returns the optional control plane socket information.
    ///
    /// # Returns
    ///
    /// An optional reference to the control plane socket information tuple.
    ///
    pub fn control_plane_socket_info(&self) -> Option<&(String, SocketType)> {
        self.control_plane_socket_info.as_ref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional path to the toolchain binary directory.
    ///
    /// # Returns
    ///
    /// An optional reference to the toolchain binary directory path.
    ///
    pub fn toolchain_binary_directory(&self) -> Option<&str> {
        self.toolchain_binary_directory.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional path to the temporary directory.
    ///
    /// # Returns
    ///
    /// An optional reference to the temporary directory path.
    ///
    pub fn tmp_directory(&self) -> Option<&str> {
        self.tmp_directory.as_deref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional flag indicating whether the Linux Daemon should be deployed inside an L2 VM.
    ///
    /// # Returns
    ///
    /// An optional boolean flag for L2 VM deployment.
    ///
    pub fn l2(&self) -> Option<bool> {
        self.l2
    }

    ///
    /// # Description
    ///
    /// Consumes the configuration and returns the gateway socket information tuple.
    ///
    /// This method is useful when ownership of the TcpPort is needed.
    ///
    /// # Returns
    ///
    /// The gateway socket information tuple.
    ///
    pub fn into_gateway_socket_info(self) -> (String, SocketType, Option<TcpPort>) {
        self.gateway_socket_info
    }
}
