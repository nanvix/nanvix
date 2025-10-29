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

use ::anyhow::Result;
use ::nanvix_sandbox::{
    syscomm::SocketType,
    tcp_port::TcpPort,
    HwLoc,
};
use ::syslog::error;
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Suffix for Unix sockets in debug builds.
///
#[cfg(debug_assertions)]
const UNIX_SOCKET_SUFFIX: &str = ".debug.socket";

///
/// # Description
///
/// Suffix for Unix sockets in release builds.
///
#[cfg(not(debug_assertions))]
const UNIX_SOCKET_SUFFIX: &str = ".socket";

///
/// # Description
///
/// Default path to the temporary directory.
///
pub const DEFAULT_TMP_DIRECTORY: &str = "/tmp";

///
/// # Description
///
/// HTTP header name for message type identification.
///
pub const HTTP_HEADER_MESSAGE_TYPE: &str = "X-NVX-Message-Type";

///
/// # Description
///
/// Maximum length for a Unix socket name, including the null terminator.
///
/// This is a workaround for the fact that `libc::UNIX_PATH_MAX` is not available.
/// On Linux, this is defined in `<linux/un.h>`.
///
/// TODO: replace this with `libc::UNIX_PATH_MAX` when it becomes available.
///
const UNIX_PATH_MAX: usize = 108;

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
#[derive(Clone)]
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
    /// Path to kernel binary.
    kernel_binary_path: String,
    /// Path to the Linux Daemon binary.
    #[cfg(not(feature = "single-process"))]
    linuxd_binary_path: String,
    /// Path to the User VM binary.
    #[cfg(not(feature = "single-process"))]
    uservm_binary_path: String,
    /// System call table.
    #[cfg(feature = "single-process")]
    syscall_table: Option<::std::sync::Arc<::nanvix_sandbox::SyscallTable>>,
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
    /// - `kernel_binary_path`: Path to kernel binary.
    /// - `linuxd_binary_path`: Path to the Linux Daemon binary (only if not in single-process mode).
    /// - `uservm_binary_path`: Path to the User VM binary (only if not in single-process mode).
    /// - `syscall_table`: Optional system call table (only in single-process mode).
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
        kernel_binary_path: &str,
        #[cfg(not(feature = "single-process"))] linuxd_binary_path: &str,
        #[cfg(not(feature = "single-process"))] uservm_binary_path: &str,
        #[cfg(feature = "single-process")] syscall_table: Option<
            ::std::sync::Arc<::nanvix_sandbox::SyscallTable>,
        >,
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
            kernel_binary_path: kernel_binary_path.to_string(),
            #[cfg(not(feature = "single-process"))]
            linuxd_binary_path: linuxd_binary_path.to_string(),
            #[cfg(not(feature = "single-process"))]
            uservm_binary_path: uservm_binary_path.to_string(),
            #[cfg(feature = "single-process")]
            syscall_table,
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
    /// Returns a handle to the system call table.
    ///
    /// # Returns
    ///
    /// If a system call table is set, this function returns a handle to it. Otherwise, it returns
    /// empty.
    ///
    #[cfg(feature = "single-process")]
    pub fn syscall_table(&self) -> Option<::std::sync::Arc<::nanvix_sandbox::SyscallTable>> {
        self.syscall_table.clone()
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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds the control plane socket address for a given tenant ID. If nanvixd is configured to
/// spawn linuxd in an L2 VM, it will return a TCP socket address, otherwise a Unix socket one.
///
/// When binding to a TCP address we want to make sure that any L2 VM can connect to us, so we bind
/// to 0.0.0.0.
///
/// # Parameters
///
/// - `tmp_str`: Temporary directory path.
/// - `tenant_id`: Tenant ID.
/// - `l2`: Flag indicating whether to deploy linuxd inside an L2 VM.
///
/// # Returns
///
/// On success, returns the control plane socket address. On failure, returns an error.
///
pub(crate) fn control_plane_sockaddr_builder(
    tmp_str: &str,
    tenant_id: &str,
    l2: bool,
) -> Result<String> {
    if l2 {
        return Ok(format!("0.0.0.0:{}", config::linuxd::CONTROL_PLANE_PORT));
    }

    let unix_socket_name: String =
        format!("{tmp_str}/control-plane:{tenant_id}:cp{UNIX_SOCKET_SUFFIX}");

    // Check if socket name exceeds the maximum length.
    if unix_socket_name.len() > UNIX_PATH_MAX {
        let error: String = format!(
            "unix socket name '{unix_socket_name}' exceeds maximum length ({:?} > {:?})",
            unix_socket_name.len(),
            UNIX_PATH_MAX
        );
        error!("control_plane_sockaddr_builder(): {error}");
        anyhow::bail!(error);
    }

    Ok(unix_socket_name)
}

///
/// # Description
///
/// Builds the user VM socket address for a given tenant ID.
///
/// # Parameters
///
/// - `tmp_str`: Temporary directory path.
/// - `tenant_id`: Tenant ID.
/// - `l2`: Flag indicating whether to deploy linuxd inside an L2 VM.
///
/// # Returns
///
/// On success, returns the user VM socket address. On failure, returns an error.
///
pub(crate) fn user_vm_sockaddr_builder(tmp_str: &str, tenant_id: &str, l2: bool) -> Result<String> {
    if l2 {
        return Ok(format!(
            "{}:{}",
            config::linuxd::GUEST_TAP_IP_ADDRESS,
            config::linuxd::USER_VM_PORT
        ));
    }

    let unix_socket_name: String = format!("{tmp_str}/{tenant_id}:uvm{UNIX_SOCKET_SUFFIX}");

    // Check if socket name exceeds the maximum length.
    if unix_socket_name.len() > UNIX_PATH_MAX {
        let error: String = format!(
            "unix socket name '{unix_socket_name}' exceeds maximum length ({:?} > {:?})",
            unix_socket_name.len(),
            UNIX_PATH_MAX
        );
        error!("user_vm_sockaddr_builder(): {error}");
        anyhow::bail!(error);
    }

    Ok(unix_socket_name)
}

///
/// # Description
///
/// Builds the gateway socket address for a given tenant and sandbox ID.
///
/// # Parameters
///
/// - `tmp_str`: Temporary directory path.
/// - `tenant_id`: Tenant ID.
/// - `sandbox_id`: Sandbox ID.
/// - `l2_port`: Optional TCP port for the gateway in L2 deployment mode. If set, it indicates
///   deployment in an L2 VM and contains the TCP port for the gateway.
///
/// # Returns
///
/// On success, returns the gateway socket address. On failure, returns an error.
///
pub(crate) fn gateway_sockaddr_builder(
    tmp_str: &str,
    tenant_id: &str,
    sandbox_id: UserVmIdentifier,
    l2_port: &Option<TcpPort>,
) -> Result<String> {
    if let Some(l2_port) = l2_port {
        return Ok(format!("{}:{:?}", config::linuxd::GUEST_TAP_IP_ADDRESS, l2_port));
    }

    let sandbox_id: u32 = sandbox_id.into();
    let unix_socket_name: String =
        format!("{tmp_str}/{tenant_id}:gw-{sandbox_id}{UNIX_SOCKET_SUFFIX}");

    // Check if socket name exceeds the maximum length.
    if unix_socket_name.len() > UNIX_PATH_MAX {
        let error: String = format!(
            "unix socket name '{unix_socket_name}' exceeds maximum length ({:?} > {:?})",
            unix_socket_name.len(),
            UNIX_PATH_MAX
        );
        error!("gateway_sockaddr_builder(): {error}");
        anyhow::bail!(error);
    }

    Ok(unix_socket_name)
}
