// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sandbox::tcp_port::TcpPort;
use ::anyhow::Result;
use ::linuxd::config::l2_system_vm_guest_ip;
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
/// Default binary directory path for Nanvix binaries.
///
pub const DEFAULT_BIN_DIRECTORY: &str = "./bin";

///
/// # Description
///
/// Default binary directory path for toolchain-related binaries.
///
pub const DEFAULT_TOOLCHAIN_BIN_DIRECTORY: &str = "./toolchain/bin";

///
/// # Description
///
/// Default directory for logs
///
pub const DEFAULT_LOG_DIRECTORY: &str = "./logs";

/// Suffix for Unix sockets.
#[cfg(debug_assertions)]
const UNIX_SOCKET_SUFFIX: &str = ".debug.socket";
#[cfg(not(debug_assertions))]
const UNIX_SOCKET_SUFFIX: &str = ".socket";

/// Path to the temporary directory.
pub const DEFAULT_TMP_DIRECTORY: &str = "/tmp";

pub const HTTP_HEADER_MESSAGE_TYPE: &str = "X-NVX-Message-Type";

/// Maximum length for a Unix socket name, including the null terminator.
/// This is a workaround for the fact that `libc::UNIX_PATH_MAX` is not available.
/// On Linux, this is defined in `<linux/un.h>`.
/// TODO: replace this with `libc::UNIX_PATH_MAX` when it becomes available.
const UNIX_PATH_MAX: usize = 108;

///
/// # Description
///
/// We use control-plane messages to synchronize the graceful shutdown of different components.
/// However, if components are faulty or hang, nanvixd cannot block. Instead, we wait for this
/// timeout and revert to non-graceful shutdowns if the timeout is met.
///
/// This constant is only used when building nanvixd as a binary, so we need to allow(dead_code)
/// for when we build nanvixd as a library.
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
/// # Arguments
///
/// - tmp_str: Temporary directory path.
/// - tenant_id: Tenant ID.
/// - l2: Flag to enable deploying linuxd inside an L2 VM.
///
/// # Returns
///
/// On success, returns the name of the control plane socket. On failure, returns an error.
///
pub fn control_plane_sockaddr_builder(tmp_str: &str, tenant_id: &str, l2: bool) -> Result<String> {
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
/// # Arguments
///
/// - tmp_str: Temporary directory path.
/// - tenant_id: Tenant ID.
/// - l2: Flag to enable deploying linuxd inside an L2 VM.
///
/// # Returns
///
/// On success, returns the name of the user VM Unix socket. On failure, returns an error.
///
pub fn user_vm_sockaddr_builder(tmp_str: &str, tenant_id: &str, l2: bool) -> Result<String> {
    if l2 {
        return Ok(format!("{}:{}", l2_system_vm_guest_ip(), config::linuxd::USER_VM_PORT));
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
/// Builds the gateway Unix socket address for a given tenant and sandbox ID.
///
/// # Arguments
///
/// - tmp_str: Temporary directory path.
/// - tenant_id: Tenant ID.
/// - sandbox_id: Sandbox ID.
/// - l2_port: Optional value to indicate deployment in an L2 VM. If set, it contains the TCP port
///   for the gateway in the L2 VM.
///
/// # Returns
///
/// On success, returns the name of the gateway Unix socket. On failure, returns an error.
///
pub fn gateway_sockaddr_builder(
    tmp_str: &str,
    tenant_id: &str,
    sandbox_id: UserVmIdentifier,
    l2_port: &Option<TcpPort>,
) -> Result<String> {
    if let Some(l2_port) = l2_port {
        return Ok(format!("{}:{:?}", l2_system_vm_guest_ip(), l2_port));
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

///
/// # Description
///
/// Gets the absolute path for the source root.
///
/// # Returns
///
/// The absolute path to the source code root.
///
#[cfg(not(feature = "single-process"))]
fn get_proj_root() -> String {
    format!("{}/../../..", env!("CARGO_MANIFEST_DIR"))
}

///
/// # Description
///
/// Gets the absolute path for cloud-hypervisor's binary directory given a
/// path (potentially sym-linked) to the toolchain binary directory.
///
/// During toolchain build we set the CAP_NET_ADMIN to the cloud-hypervisor
/// binary and, depending on the file-system type, these capabilities do not
/// propagate well.
///
/// # Arguments
///
/// - `toolchain_bin_dir`: path to Nanvix's toolchain binary directory.
///
/// # Returns
///
/// The absolute path to cloud-hypervisor's binary directory.
///
#[cfg(not(feature = "single-process"))]
pub fn get_clh_bin_dir(toolchain_bin_dir: &str) -> Result<String> {
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
pub fn get_clh_snapshot_path() -> String {
    format!("{}/images/{}", get_proj_root(), config::linuxd::SNAPSHOT_NAME)
}

///
/// # Description
///
/// Gets the absolute path for cloud-hypervisor's API socket.
///
/// # Arguments
///
/// - tmp_dir: Temporary directory.
///
/// # Returns
///
/// The absolute path to cloud-hypervisor's snapshot directory.
///
#[cfg(not(feature = "single-process"))]
pub fn get_clh_api_socket_path(tmp_dir: &str) -> String {
    format!("{tmp_dir}/nanvixd-clh{UNIX_SOCKET_SUFFIX}")
}
