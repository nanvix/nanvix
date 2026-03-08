// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Configuration for the simplified sandbox cache.
//!
//! This module provides a configuration structure for the single-process sandbox cache. It only
//! stores the parameters relevant to single-process deployments, deliberately omitting
//! multi-process concepts such as external binary paths, L2 deployment, and network namespaces.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    syscomm::SocketType,
    HwLoc,
    SyscallTable,
};
use ::std::sync::Arc;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Configuration for the simplified (single-process) sandbox cache.
///
/// # Type Parameters
///
/// - `T`: Custom state type for the syscall table. Use `()` if no custom state is required.
///
#[derive(Clone)]
pub struct SimpleSandboxCacheConfig<T> {
    /// Socket type for control plane communication.
    control_plane_socket_type: SocketType,
    /// Socket type for gateway communication.
    gateway_socket_type: SocketType,
    /// Socket type for system VM communication.
    system_vm_socket_type: SocketType,
    /// Optional file path for redirecting console output.
    console_file: Option<String>,
    /// Optional RAM filesystem image that should be exposed to user VMs.
    ramfs_filename: Option<String>,
    /// Optional hardware locality configuration.
    hwloc: Option<HwLoc>,
    /// Path to kernel binary.
    kernel_binary_path: String,
    /// System call table.
    syscall_table: Option<Arc<SyscallTable<T>>>,
    /// Path to the toolchain binary directory.
    toolchain_binary_directory: String,
    /// Directory path for writing log files.
    log_directory: String,
    /// Path to the temporary directory for Unix sockets and transient files.
    tmp_directory: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T: Sync + Send + Default + 'static> SimpleSandboxCacheConfig<T> {
    ///
    /// # Description
    ///
    /// Creates a new single-process sandbox cache configuration.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        control_plane_socket_type: SocketType,
        gateway_socket_type: SocketType,
        system_vm_socket_type: SocketType,
        console_file: Option<String>,
        ramfs_filename: Option<String>,
        hwloc: Option<HwLoc>,
        kernel_binary_path: &str,
        syscall_table: Option<Arc<SyscallTable<T>>>,
        toolchain_binary_directory: &str,
        log_directory: &str,
        tmp_directory: &str,
    ) -> Self {
        Self {
            control_plane_socket_type,
            gateway_socket_type,
            system_vm_socket_type,
            console_file,
            ramfs_filename,
            hwloc,
            kernel_binary_path: kernel_binary_path.to_string(),
            syscall_table,
            toolchain_binary_directory: toolchain_binary_directory.to_string(),
            log_directory: log_directory.to_string(),
            tmp_directory: tmp_directory.to_string(),
        }
    }

    /// Returns the socket type configured for control plane communication.
    pub fn control_plane_sockaddr_type(&self) -> SocketType {
        self.control_plane_socket_type
    }

    /// Returns the socket type configured for gateway communication.
    pub fn gateway_sockaddr_type(&self) -> SocketType {
        self.gateway_socket_type
    }

    /// Returns the socket type configured for system VM communication.
    pub fn system_vm_sockaddr_type(&self) -> SocketType {
        self.system_vm_socket_type
    }

    /// Returns the optional file path for console output redirection.
    pub fn console_file(&self) -> Option<&str> {
        self.console_file.as_deref()
    }

    /// Returns the optional RAM filesystem filename exposed to user VMs.
    pub fn ramfs_filename(&self) -> Option<&str> {
        self.ramfs_filename.as_deref()
    }

    /// Returns the hardware locality configuration if available.
    pub fn hwloc(&self) -> Option<HwLoc> {
        self.hwloc.clone()
    }

    /// Returns the path to the kernel binary.
    pub fn kernel_binary_path(&self) -> &str {
        &self.kernel_binary_path
    }

    /// Returns a handle to the system call table.
    pub fn syscall_table(&self) -> Option<Arc<SyscallTable<T>>> {
        self.syscall_table.clone()
    }

    /// Returns the path to the toolchain binary directory.
    pub fn toolchain_binary_directory(&self) -> &str {
        &self.toolchain_binary_directory
    }

    /// Returns the log directory.
    pub fn log_directory(&self) -> &str {
        &self.log_directory
    }

    /// Returns the path to the temporary directory.
    pub fn tmp_directory(&self) -> &str {
        &self.tmp_directory
    }
}
