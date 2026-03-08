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
use ::build_utils::find_workspace_root;
use ::chrono::Local;
#[cfg(not(feature = "single-process"))]
use ::std::marker::PhantomData;
use ::std::path::PathBuf;
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
/// # Type Parameters
///
/// - `T`: Custom state type for the syscall table. This is passed to system call handlers in
///   single-process mode. Use `()` if no custom state is required.
///
pub struct SandboxConfig<T> {
    /// Unique identifier for the tenant.
    tenant_id: String,
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
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    linuxd_binary_path: String,
    /// Path to the User VM binary.
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    uservm_binary_path: String,
    /// Directory path for writing log files.
    log_directory: String,
    /// Optional system call table for overriding default system call behavior.
    #[cfg(feature = "single-process")]
    syscall_table: Option<::std::sync::Arc<::linuxd::syscalls::SyscallTable<T>>>,

    /// Optional information on the control plane listener socket (address, socket type).
    /// This must be provided if the control plane listener socket is not already initialized
    /// before sandbox initialization. There is one listener socket per deployment. If both socket
    /// and info are provided, the existing socket is used.
    control_plane_bind_socket_info: Option<(String, SocketType)>,

    /// Information on the control plane connecting socket (address, socket type).
    /// This changes for every sandbox, and thus must be provided every time.
    control_plane_connect_socket_info: (String, SocketType),

    /// Optional path to the toolchain binary directory containing cloud-hypervisor and other tools.
    /// This must be provided if a Linux Daemon instance was not provided before sandbox initialization.
    toolchain_binary_directory: Option<String>,

    /// Optional path to the temporary directory for Unix sockets and transient files.
    /// This must be provided if a Linux Daemon instance was not provided before sandbox initialization.
    tmp_directory: Option<String>,

    /// Optional flag to deploy the Linux Daemon inside an L2 VM (using cloud-hypervisor).
    /// This must be provided if a Linux Daemon instance was not provided before sandbox initialization.
    l2: Option<bool>,

    /// Phantom data to maintain the generic type parameter `T` in the structure.
    /// This is required because `T` is only used in single-process mode for the syscall table.
    #[cfg(not(feature = "single-process"))]
    _phantom: PhantomData<T>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T> SandboxConfig<T> {
    ///
    /// # Description
    ///
    /// Generates a default console file path for the sandbox when no explicit path is provided.
    /// The path is constructed from the log directory and includes the user VM identifier along
    /// with a timestamp to ensure uniqueness.
    ///
    /// # Arguments
    ///
    /// - `uservm_id`: user VM unique identifier.
    /// - `log_dir`: log directory for the sandbox.
    ///
    /// # Returns
    ///
    /// The path to the console file as a string.
    ///
    fn default_console_file(uservm_id: UserVmIdentifier, log_dir: &str) -> String {
        let console_file: PathBuf = PathBuf::from(log_dir)
            .join(format!("guest_{uservm_id}_{}.log", Local::now().format("%Y_%m_%d_%H_%M_%S_%f")));

        console_file.to_string_lossy().into_owned()
    }

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
    /// - `control_plane_bind_socket_info`: Optional information on control plane listener socket (address, socket type).
    /// - `control_plane_connect_socket_info`: Optional information on control plane connect socket (address, socket type).
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
        tenant_id: &str,
        uservm_id: UserVmIdentifier,
        gateway_socket_info: (String, SocketType, Option<TcpPort>),
        system_vm_socket_info: (String, SocketType),
        console_file: Option<String>,
        hwloc: Option<hwloc::HwLoc>,
        kernel_binary_path: &str,
        #[cfg(not(any(feature = "single-process", feature = "standalone")))]
        linuxd_binary_path: &str,
        #[cfg(not(any(feature = "single-process", feature = "standalone")))]
        uservm_binary_path: &str,
        log_directory: &str,
        #[cfg(feature = "single-process")] syscall_table: Option<
            ::std::sync::Arc<::linuxd::syscalls::SyscallTable<T>>,
        >,
        control_plane_bind_socket_info: Option<(String, SocketType)>,
        control_plane_connect_socket_info: (String, SocketType),
        toolchain_binary_directory: Option<String>,
        tmp_directory: Option<String>,
        l2: Option<bool>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            uservm_id,
            gateway_socket_info,
            system_vm_socket_info,
            console_file: Some(
                console_file
                    .unwrap_or_else(|| Self::default_console_file(uservm_id, log_directory)),
            ),
            hwloc,
            kernel_binary_path: kernel_binary_path.to_string(),
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            linuxd_binary_path: linuxd_binary_path.to_string(),
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            uservm_binary_path: uservm_binary_path.to_string(),
            log_directory: log_directory.to_string(),
            #[cfg(feature = "single-process")]
            syscall_table,
            control_plane_bind_socket_info,
            control_plane_connect_socket_info,
            toolchain_binary_directory,
            tmp_directory,
            l2,
            #[cfg(not(feature = "single-process"))]
            _phantom: PhantomData,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the unique tenant identifier for the sandbox.
    ///
    /// # Returns
    ///
    /// The tenant identifier.
    ///
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
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
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
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
    /// Returns the optional system call table.
    ///
    /// # Returns
    ///
    /// An optional clone of the system call table.
    ///
    #[cfg(feature = "single-process")]
    pub fn syscall_table(&self) -> Option<::std::sync::Arc<::linuxd::syscalls::SyscallTable<T>>> {
        self.syscall_table.clone()
    }

    ///
    /// # Description
    ///
    /// Returns the optional control plane listener socket information.
    ///
    /// # Returns
    ///
    /// An optional reference to the control plane listener socket information tuple.
    ///
    pub fn control_plane_bind_socket_info(&self) -> Option<&(String, SocketType)> {
        self.control_plane_bind_socket_info.as_ref()
    }

    ///
    /// # Description
    ///
    /// Returns the optional control plane connect socket information.
    ///
    /// # Returns
    ///
    /// An optional reference to the control plane connect socket information tuple.
    ///
    pub fn control_plane_connect_socket_info(&self) -> &(String, SocketType) {
        &self.control_plane_connect_socket_info
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
    /// Generates the linuxd log file in an L2 deployment. This log file is specified during
    /// restore of the L2 snapshot.
    ///
    /// Each sandbox generates a different log file (with a different timestamp), but the actual
    /// file name will be determined by the first sanbdox for the given tenant.
    ///
    /// # Returns
    ///
    /// The path to linuxd log file.
    ///
    pub fn l2_linuxd_log_file(&self) -> String {
        let mut console_file: PathBuf = PathBuf::from(self.log_directory.clone()).join(format!(
            "linuxd-l2_{}_{}.log",
            self.tenant_id,
            Local::now().format("%Y_%m_%d_%H_%M_%S_%f")
        ));
        if !console_file.is_absolute() {
            console_file = find_workspace_root().join(console_file);
        }

        console_file.to_string_lossy().into_owned()
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
