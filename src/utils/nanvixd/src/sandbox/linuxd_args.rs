// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Arguments for spawning Linux Daemon instances.
//!
//! This module defines the `LinuxDaemonArgs` structure which encapsulates all configuration
//! parameters required to spawn a Linux Daemon instance. This includes socket information,
//! file paths, hardware topology settings, and optional system call table configuration.

//==================================================================================================
// Imports
//==================================================================================================

use ::linuxd::syscalls::SyscallTable;
use ::std::sync::Arc;
use ::syscomm::SocketType;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Arguments for spawning a Linux Daemon instance.
///
pub struct LinuxDaemonArgs {
    /// Information on control plane socket (address, socket type).
    control_plane_socket_info: (String, SocketType),
    /// Information on System VM socket (address, socket type).
    system_vm_socket_info: (String, SocketType),
    /// Optional hardware locality configuration for CPU affinity and topology information.
    hwloc: Option<hwloc::HwLoc>,
    /// Path to the binary directory containing Nanvix binaries.
    binary_directory: String,
    /// Path to the toolchain binary directory containing cloud-hypervisor and other tools.
    toolchain_binary_directory: String,
    /// Directory path for writing log files.
    log_directory: String,
    /// Temporary directory path for Unix sockets and transient files.
    tmp_directory: String,
    /// Flag to deploy linuxd inside an L2 VM (using cloud-hypervisor).
    l2: bool,
    /// Optional system call table for overriding default system call behavior.
    syscall_table: Option<Arc<SyscallTable>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemonArgs {
    ///
    /// # Description
    ///
    /// Creates a new `LinuxDaemonArgs` instance with the specified parameters.
    ///
    /// # Parameters
    ///
    /// - `control_plane_socket_info`: Information on control plane socket (address, socket type).
    /// - `system_vm_socket_info`: Information on System VM socket (address, socket type).
    /// - `hwloc`: Optional hardware locality configuration for CPU affinity and topology information.
    /// - `binary_directory`: Path to the binary directory containing Nanvix binaries.
    /// - `toolchain_binary_directory`: Path to the toolchain binary directory containing cloud-hypervisor and other tools.
    /// - `log_directory`: Directory path for writing log files.
    /// - `tmp_directory`: Temporary directory path for Unix sockets and transient files.
    /// - `l2`: Flag to deploy linuxd inside an L2 VM (using cloud-hypervisor).
    /// - `syscall_table`: Optional system call table for overriding default system call behavior.
    ///
    /// # Returns
    ///
    /// A new `LinuxDaemonArgs` instance.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        control_plane_socket_info: (String, SocketType),
        system_vm_socket_info: (String, SocketType),
        hwloc: Option<hwloc::HwLoc>,
        binary_directory: String,
        toolchain_binary_directory: String,
        log_directory: String,
        tmp_directory: String,
        l2: bool,
        syscall_table: Option<Arc<SyscallTable>>,
    ) -> Self {
        Self {
            control_plane_socket_info,
            system_vm_socket_info,
            hwloc,
            binary_directory,
            toolchain_binary_directory,
            log_directory,
            tmp_directory,
            l2,
            syscall_table,
        }
    }

    ///
    /// # Description
    ///
    /// Returns information on the control plane socket (address and socket type).
    ///
    /// # Returns
    ///
    /// A reference to the control plane socket information.
    ///
    pub fn control_plane_socket_info(&self) -> &(String, SocketType) {
        &self.control_plane_socket_info
    }

    ///
    /// # Description
    ///
    /// Returns information on the System VM socket (address and socket type).
    ///
    /// # Returns
    ///
    /// A reference to the System VM socket information.
    ///
    pub fn system_vm_socket_info(&self) -> &(String, SocketType) {
        &self.system_vm_socket_info
    }

    ///
    /// # Description
    ///
    /// Returns the optional hardware locality configuration.
    ///
    /// # Returns
    ///
    /// An optional reference to the hardware locality configuration.
    ///
    pub fn hwloc(&self) -> Option<&hwloc::HwLoc> {
        self.hwloc.as_ref()
    }

    ///
    /// # Description
    ///
    /// Returns the path to the binary directory containing Nanvix binaries.
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
    /// Returns the path to the toolchain binary directory containing cloud-hypervisor and other tools.
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
    /// Returns the directory path for writing log files.
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
    /// Returns the temporary directory path for Unix sockets and transient files.
    ///
    /// # Returns
    ///
    /// The path to the temporary directory.
    ///
    pub fn tmp_directory(&self) -> &str {
        &self.tmp_directory
    }

    ///
    /// # Description
    ///
    /// Returns the flag indicating whether to deploy linuxd inside an L2 VM.
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
    /// Returns the optional system call table for overriding default system call behavior.
    ///
    /// # Returns
    ///
    /// An optional reference to the system call table.
    ///
    pub fn syscall_table(&self) -> Option<&Arc<SyscallTable>> {
        self.syscall_table.as_ref()
    }
}
