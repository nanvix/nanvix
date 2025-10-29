// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Configuration constants for sandbox management.
//!
//! This module provides configuration constants used throughout the sandbox implementation,
//! including timeouts for various operations and path utilities for L2 deployment.

//==================================================================================================
// Imports
//==================================================================================================

use ::tokio::time::Duration;

#[cfg(not(feature = "single-process"))]
use ::anyhow::Result;
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
/// Suffix for Unix sockets in debug builds.
///
#[cfg(all(debug_assertions, not(feature = "single-process")))]
const UNIX_SOCKET_SUFFIX: &str = ".debug.socket";

///
/// # Description
///
/// Suffix for Unix sockets in release builds.
///
#[cfg(all(not(debug_assertions), not(feature = "single-process")))]
const UNIX_SOCKET_SUFFIX: &str = ".socket";

//==================================================================================================
// Standalone Functions
//==================================================================================================

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
    format!("{}/images/{}", get_proj_root(), ::config::linuxd::SNAPSHOT_NAME)
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
pub fn get_clh_api_socket_path(tmp_dir: &str) -> String {
    format!("{tmp_dir}/nanvixd-clh{UNIX_SOCKET_SUFFIX}")
}
