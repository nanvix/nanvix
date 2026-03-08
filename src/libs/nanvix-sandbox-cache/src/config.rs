// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox cache configuration management.
//!
//! This module provides structures for configuring the sandbox cache within the Nanvix Daemon.
//! It handles socket types, file paths, hardware topology settings, and deployment modes
//! (L2 VM support) that apply to all sandboxes managed by the daemon.

//==================================================================================================
// Imports
//==================================================================================================

use ::nanvix_sandbox::{
    syscomm::SocketType,
    HwLoc,
};
use ::std::marker::PhantomData;

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
/// # Type Parameters
///
/// - `T`: Custom state type. Use `()` if no custom state is required.
///
#[derive(Clone)]
pub struct SandboxCacheConfig<T> {
    /// Socket type for control plane communication between nanvixd and linuxd.
    control_plane_socket_type: SocketType,
    /// Socket type for gateway communication between external clients and linuxd for stdin/stdout.
    gateway_socket_type: SocketType,
    /// Socket type for system VM communication between linuxd and user VMs.
    system_vm_socket_type: SocketType,
    /// Optional file path for redirecting console output.
    console_file: Option<String>,
    /// Optional RAM filesystem image that should be exposed to user VMs.
    ramfs_filename: Option<String>,
    /// Optional hardware locality configuration for CPU affinity and topology information.
    hwloc: Option<HwLoc>,
    /// Number of network namespaces to prefill in the pool (0 enables lazy initialization).
    netns_pool_size: usize,
    /// Path to kernel binary.
    kernel_binary_path: String,
    /// Path to the Linux Daemon binary.
    linuxd_binary_path: String,
    /// Path to the User VM binary.
    uservm_binary_path: String,
    /// Path to the toolchain binary directory containing cloud-hypervisor and other tools.
    toolchain_binary_directory: String,
    /// Directory path for writing log files.
    log_directory: String,
    /// Flag indicating whether to deploy linuxd inside an L2 VM (using cloud-hypervisor).
    l2: bool,
    /// Path to the base snapshot file in an L2 deployment.
    l2_snapshot_path: String,
    /// Path to the temporary directory for Unix sockets and transient files.
    tmp_directory: String,
    /// Phantom data to maintain the generic type parameter `T` in the structure.
    _phantom: PhantomData<T>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T: Sync + Send + Default + 'static> SandboxCacheConfig<T> {
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
    /// - `ramfs_filename`: Optional RAM filesystem image filename.
    /// - `hwloc`: Optional hardware locality configuration.
    /// - `netns_pool_size`: Number of network namespaces to prefill (0 for lazy initialization).
    /// - `kernel_binary_path`: Path to kernel binary.
    /// - `linuxd_binary_path`: Path to the Linux Daemon binary.
    /// - `uservm_binary_path`: Path to the User VM binary.
    /// - `toolchain_binary_directory`: Path to the toolchain binary directory.
    /// - `log_directory`: Path to the log directory.
    /// - `l2`: Flag to deploy linuxd inside an L2 VM.
    /// - `l2_snapshot_path`: Path to the L2 VM's base snapshot.
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
        ramfs_filename: Option<String>,
        hwloc: Option<HwLoc>,
        netns_pool_size: usize,
        kernel_binary_path: &str,
        linuxd_binary_path: &str,
        uservm_binary_path: &str,
        toolchain_binary_directory: &str,
        log_directory: &str,
        l2: bool,
        l2_snapshot_path: &str,
        tmp_directory: &str,
    ) -> Self {
        Self {
            control_plane_socket_type,
            gateway_socket_type,
            system_vm_socket_type,
            console_file,
            ramfs_filename,
            hwloc,
            netns_pool_size,
            kernel_binary_path: kernel_binary_path.to_string(),
            linuxd_binary_path: linuxd_binary_path.to_string(),
            uservm_binary_path: uservm_binary_path.to_string(),
            toolchain_binary_directory: toolchain_binary_directory.to_string(),
            log_directory: log_directory.to_string(),
            l2,
            l2_snapshot_path: l2_snapshot_path.to_string(),
            tmp_directory: tmp_directory.to_string(),
            _phantom: PhantomData,
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
    /// Returns the optional RAM filesystem filename exposed to user VMs.
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
    /// Returns the size of the prefilled network namespace pool.
    ///
    /// # Returns
    ///
    /// The number of namespaces to prefill (0 for lazy initialization).
    ///
    pub fn netns_pool_size(&self) -> usize {
        self.netns_pool_size
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
    /// Returns the path to the Linux Daemon binary.
    ///
    /// # Returns
    ///
    /// The path to the Linux Daemon binary.
    ///
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
    pub fn uservm_binary_path(&self) -> &str {
        &self.uservm_binary_path
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
    /// Returns the L2 base snapshot path.
    ///
    /// # Returns
    ///
    /// The L2 base snapshot path.
    ///
    pub fn l2_snapshot_path(&self) -> &str {
        &self.l2_snapshot_path
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
