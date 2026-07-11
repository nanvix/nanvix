// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Configuration constants for sandbox management.
//!
//! This module provides configuration constants used throughout the sandbox implementation,
//! including timeouts for various operations and socket path utilities.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::error;
use ::tokio::time::Duration;
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Timeout for waiting for graceful shutdown of standalone UserVM instances.
///
/// Standalone workloads normally terminate themselves, so this is a short final cleanup bound.
///
#[cfg(feature = "standalone")]
pub const STANDALONE_CLEANUP_TIMEOUT: Duration = Duration::from_secs(1);

///
/// # Description
///
/// Timeout for waiting for graceful shutdown of single-process UserVM instances.
///
/// This exceeds the UserVM's five-second vCPU shutdown watchdog and its two bounded worker joins,
/// preventing the sandbox from detaching a teardown task while those safeguards are still active.
///
#[cfg(feature = "single-process")]
pub const SINGLE_PROCESS_CLEANUP_TIMEOUT: Duration = Duration::from_secs(16);

///
/// # Description
///
/// Timeout for accepting connections on the control plane.
///
#[cfg(not(feature = "standalone"))]
pub const CONTROL_PLANE_ACCEPT_TIMEOUT: Duration = Duration::from_secs(60);

///
/// Maximum number of early (unregistered) control-plane connections buffered before eviction.
///
#[cfg(not(feature = "standalone"))]
pub const MAX_EARLY_CONTROL_PLANE_CONNECTIONS: usize = 64;

///
/// Maximum number of concurrent in-flight connection handler tasks in the control-plane acceptor.
///
#[cfg(not(feature = "standalone"))]
pub const MAX_CONCURRENT_CONTROL_PLANE_HANDLERS: usize = 128;

///
/// # Description
///
/// Timeout for connecting to gateway.
///
#[cfg(not(feature = "standalone"))]
pub const GATEWAY_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

///
/// # Description
///
/// Provides the timeout we should use when waiting for Linux Daemon to shut down.
///
#[cfg(not(feature = "standalone"))]
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);

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
pub const UNIX_SOCKET_SUFFIX: &str = ".debug.socket";

///
/// # Description
///
/// Suffix for Unix sockets in release builds.
///
#[cfg(not(debug_assertions))]
pub const UNIX_SOCKET_SUFFIX: &str = ".socket";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds the control plane socket addresses for nanvixd (bind) and for all linuxd and
/// user VM instances.
///
/// # Parameters
///
/// - `tmp_str`: Temporary directory path.
///
/// # Returns
///
/// On success, returns the (bind, connect) control plane socket addresses pair. On failure, returns an error.
///
pub fn control_plane_sockaddr_builder(tmp_str: &str) -> Result<(String, String)> {
    let unix_socket_name: String =
        format!("{tmp_str}/{NAMED_RESOURCE_PREFIX}:cp{UNIX_SOCKET_SUFFIX}");

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

    // Bind and connect socket addresses are the same.
    Ok((unix_socket_name.clone(), unix_socket_name))
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
///
/// # Returns
///
/// On success, returns the user VM socket address. On failure, returns an error.
///
pub fn user_vm_sockaddr_builder(tmp_str: &str, tenant_id: &str) -> Result<String> {
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
///
/// # Returns
///
/// On success, returns the gateway socket address. On failure, returns an error.
///
pub fn gateway_sockaddr_builder(
    tmp_str: &str,
    tenant_id: &str,
    sandbox_id: UserVmIdentifier,
) -> Result<String> {
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
