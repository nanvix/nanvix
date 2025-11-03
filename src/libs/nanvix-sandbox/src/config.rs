// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Configuration constants for sandbox management.
//!
//! This module provides configuration constants used throughout the sandbox implementation,
//! including timeouts for various operations and path utilities for L2 deployment.

//==================================================================================================
// Imports
//==================================================================================================

use crate::tcp_port::TcpPort;
use ::anyhow::Result;
use ::syslog::error;
use ::tokio::time::Duration;
use ::user_vm_api::UserVmIdentifier;

#[cfg(not(feature = "single-process"))]
use ::std::{
    fs,
    path::PathBuf,
};

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Timeout for waiting for graceful shutdown of UserVM instances.
///
/// We use control-plane messages to synchronize the graceful shutdown of different components.
/// However, if components are faulty or hang, the sandbox cannot block. Instead, we wait for this
/// timeout and revert to non-graceful shutdowns if the timeout is met.
///
pub const CLEANUP_TIMEOUT: Duration = Duration::from_secs(1);

///
/// # Description
///
/// Timeout for accepting connections on the control plane.
///
pub const CONTROL_PLANE_ACCEPT_TIMEOUT: Duration = Duration::from_secs(60);

///
/// # Description
///
/// Timeout for connecting to gateway.
///
pub const GATEWAY_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

///
/// # Description
///
/// Provides the timeout we should use when waiting for Linux Daemon to shut down.
///
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(60);

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

///
/// # Description
///
/// Prefix for all named resources.
///
pub const NAMED_RESOURCE_PREFIX: &str = "nvx";

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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the absolute path for cloud-hypervisor's binary directory given a path (potentially
/// sym-linked) to the toolchain binary directory.
///
/// During toolchain build we set the CAP_NET_ADMIN to the cloud-hypervisor binary and, depending
/// on the file-system type, these capabilities do not propagate well through symbolic links.
///
/// # Parameters
///
/// - `toolchain_bin_dir`: Path to Nanvix's toolchain binary directory.
///
/// # Returns
///
/// On success, the absolute path to cloud-hypervisor's binary directory. On failure, an error is
/// returned instead.
///
#[cfg(not(feature = "single-process"))]
pub(crate) fn get_clh_bin_dir(toolchain_bin_dir: &str) -> Result<String> {
    let clh_bin_dir_path: PathBuf = PathBuf::from(toolchain_bin_dir);
    Ok(format!("{}", fs::canonicalize(clh_bin_dir_path)?.display()))
}

///
/// # Description
///
/// Gets the absolute path for cloud-hypervisor's snapshot directory.
///
/// # Returns
///
/// The absolute path to cloud-hypervisor's snapshot directory.
///
#[cfg(not(feature = "single-process"))]
pub(crate) fn get_clh_snapshot_path(l2_snapshot_path: &str) -> Result<String> {
    let l2_snapshot_path: PathBuf = PathBuf::from(l2_snapshot_path);
    Ok(format!("{}", fs::canonicalize(l2_snapshot_path)?.display()))
}

///
/// # Description
///
/// Gets the absolute path for cloud-hypervisor's API socket.
///
/// # Parameters
///
/// - `tmp_dir`: Temporary directory.
///
/// # Returns
///
/// The absolute path to cloud-hypervisor's API socket.
///
#[cfg(not(feature = "single-process"))]
pub(crate) fn get_clh_api_socket_path(tmp_dir: &str) -> String {
    format!("{tmp_dir}/nanvixd-clh{UNIX_SOCKET_SUFFIX}")
}

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
pub fn control_plane_sockaddr_builder(tmp_str: &str, tenant_id: &str, l2: bool) -> Result<String> {
    if l2 {
        return Ok(format!("0.0.0.0:{}", config::linuxd::CONTROL_PLANE_PORT));
    }

    let unix_socket_name: String =
        format!("{tmp_str}/{NAMED_RESOURCE_PREFIX}:{tenant_id}:cp{UNIX_SOCKET_SUFFIX}");

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
pub fn user_vm_sockaddr_builder(tmp_str: &str, tenant_id: &str, l2: bool) -> Result<String> {
    if l2 {
        return Ok(format!(
            "{}:{}",
            config::linuxd::GUEST_TAP_IP_ADDRESS,
            config::linuxd::USER_VM_PORT
        ));
    }

    let unix_socket_name: String =
        format!("{tmp_str}/{NAMED_RESOURCE_PREFIX}:{tenant_id}:uvm{UNIX_SOCKET_SUFFIX}");

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
pub fn gateway_sockaddr_builder(
    tmp_str: &str,
    tenant_id: &str,
    sandbox_id: UserVmIdentifier,
    l2_port: &Option<TcpPort>,
) -> Result<String> {
    if let Some(l2_port) = l2_port {
        return Ok(format!("{}:{:?}", config::linuxd::GUEST_TAP_IP_ADDRESS, l2_port));
    }

    let sandbox_id: u32 = sandbox_id.into();
    let unix_socket_name: String = format!(
        "{tmp_str}/{NAMED_RESOURCE_PREFIX}:{tenant_id}:gw-{sandbox_id}{UNIX_SOCKET_SUFFIX}"
    );

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
